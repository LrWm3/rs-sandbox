// src/lib.rs

use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Inner<T> {
    data: T,
    strong: AtomicUsize, // number of live clones
    read: AtomicBool,    // has someone cloned yet?
}

pub struct QuasiArc<T> {
    ptr: NonNull<Inner<T>>,
}

impl<T> QuasiArc<T> {
    pub fn new(data: T) -> Self {
        let boxed = Box::new(Inner {
            data,
            strong: AtomicUsize::new(0),
            read: AtomicBool::new(false),
        });
        QuasiArc {
            ptr: NonNull::new(Box::into_raw(boxed)).unwrap(),
        }
    }

    /// Cancels the QuasiArc, dropping the inner data if it has not been read or cloned.
    ///
    /// This will panic if the QuasiArc has already been read or cloned.
    /// If you want to cancel without panicking, use `try_cancel`.
    pub fn cancel(self) {
        //
        match self.try_cancel() {
            Ok(()) => {}
            Err(()) => {
                panic!("cannot cancel QuasiArc after it has been cloned or read.");
            }
        }
    }

    /// Attempts to cancel the QuasiArc, dropping the inner data if it has not been read or cloned.
    /// Returns `Ok(())` if the inner data was dropped, or `Err(())` if the QuasiArc has already been read or cloned
    /// and cannot be canceled.
    pub fn try_cancel(self) -> Result<(), ()> {
        let inner = unsafe { self.ptr.as_ref() };
        if !inner.read.load(Ordering::Acquire) && inner.strong.load(Ordering::Acquire) == 0 {
            // drop the Inner<T> immediately
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<T> Clone for QuasiArc<T> {
    /// Clones the QuasiArc, incrementing the strong reference count.
    fn clone(&self) -> Self {
        let inner = unsafe { self.ptr.as_ref() };
        inner.read.store(true, Ordering::Release);
        inner.strong.fetch_add(1, Ordering::AcqRel);
        QuasiArc { ptr: self.ptr }
    }
}

impl<T> Deref for QuasiArc<T> {
    type Target = T;
    /// Dereferences the QuasiArc to access the inner data.
    fn deref(&self) -> &T {
        &unsafe { self.ptr.as_ref() }.data
    }
}

impl<T> Drop for QuasiArc<T> {
    /// Drops the QuasiArc, decrementing the strong reference count.
    /// If the strong reference count reaches zero and the inner data has been read,
    /// the inner data is dropped.
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if inner.strong.fetch_sub(1, Ordering::AcqRel) == 1 && inner.read.load(Ordering::Acquire) {
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QuasiArc;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    /// A simple type whose Drop increments a counter.
    struct Counter(Arc<AtomicUsize>);
    impl Drop for Counter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn no_clone_leaks() {
        let drops = Arc::new(AtomicUsize::new(0));
        {
            // original never reads or deallocates
            let _qa = QuasiArc::new(Counter(drops.clone()));
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            // dropping original alone should *not* drop inner
        }
        assert_eq!(
            drops.load(Ordering::SeqCst),
            0,
            "Inner must not deallocate until first clone"
        );
    }

    #[test]
    fn single_clone_drops_inner() {
        let drops = Arc::new(AtomicUsize::new(0));
        {
            let qa = QuasiArc::new(Counter(drops.clone()));
            let qa2 = qa.clone();
            // still not dropped, because qa2 is alive
            assert_eq!(drops.load(Ordering::SeqCst), 0);

            drop(qa2);
            // qa was original; qa2 was the only clone.
            // dropping that clone should free Inner<T> and run Counter::drop
            assert_eq!(
                drops.load(Ordering::SeqCst),
                1,
                "Inner should drop once after last clone is dropped"
            );
        }
    }

    #[test]
    fn multiple_clones() {
        let drops = Arc::new(AtomicUsize::new(0));
        {
            let qa = QuasiArc::new(Counter(drops.clone()));
            let qa2 = qa.clone();
            let qa3 = qa.clone();
            assert_eq!(drops.load(Ordering::SeqCst), 0);

            drop(qa2);
            // one clone gone, but one still alive: no drop yet
            assert_eq!(drops.load(Ordering::SeqCst), 0);

            drop(qa3);
            // last clone gone → free and drop Counter
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn cancel_before_clone_frees_immediately() {
        let drops = Arc::new(AtomicUsize::new(0));
        // create and then cancel without any clone
        let qa = QuasiArc::new(Counter(drops.clone()));
        qa.cancel();
        // Box::from_raw ran, so Counter::drop was called exactly once
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    // This one should panic in debug because debug_assert trips.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "cannot cancel QuasiArc after it has been cloned or read")]
    fn cancel_after_clone_panics_debug_assert() {
        let drops = Arc::new(AtomicUsize::new(0));
        let qa = QuasiArc::new(Counter(drops.clone()));
        let _qa2 = qa.clone();
        // first call to cancel() after clone must panic in debug
        qa.cancel();
    }
    // In release builds debug_assertions are off, so cancel() won't panic.
    #[test]
    fn try_cancel_after_clone_returns_err() {
        let drops = Arc::new(AtomicUsize::new(0));
        let qa = QuasiArc::new(Counter(drops.clone()));
        let qa2 = qa.clone();
        // silently drop into normal behavior
        let r = qa.try_cancel();
        drop(qa2);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(r.is_err(), "try_cancel should return Err(()) after clone");
    } // In release builds debug_assertions are off, so cancel() won't panic.
    #[test]
    fn try_cancel_no_clone_returns_ok() {
        let drops = Arc::new(AtomicUsize::new(0));
        let qa = QuasiArc::new(Counter(drops.clone()));
        // silently drop into normal behavior
        let r = qa.try_cancel();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(r.is_ok(), "try_cancel should return Ok(()) before clone");
        // after this, the inner data is dropped, so we can't clone anymore
    }
}
