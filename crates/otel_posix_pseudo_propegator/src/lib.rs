// Cargo.toml:
// [lib] crate-type = ["cdylib"]

use libc::{pthread_attr_t, pthread_t};
use opentelemetry::{Context, trace::TraceContextExt};
use std::ffi::c_void;

// A little launcher holding the real fn + its arg + the OTEL Context
struct Launch {
    real_fn: extern "C" fn(*mut c_void) -> *mut c_void,
    real_arg: *mut c_void,
    ctx: Context,
}

extern "C" fn trampoline(v: *mut c_void) -> *mut c_void {
    // recover the Launch struct
    let launch: Box<Launch> = unsafe { Box::from_raw(v as *mut Launch) };
    // activate the captured Context
    let _guard = launch.ctx.attach();
    // call the original thread entry point
    (launch.real_fn)(launch.real_arg)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_create(
    tid: *mut pthread_t,
    attr: *const pthread_attr_t,
    start_routine: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> i32 {
    // 1. capture the current OTEL Context
    let cx = Context::current();

    // if no context, just call the original pthread_create
    // This is a fast path to avoid unnecessary overhead when no context is active.
    if !cx.has_active_span() {
        // if no context, just call the original pthread_create
        return unsafe { __real_pthread_create(tid, attr, start_routine, arg) };
    }

    // 2. box up the real fn, its arg, and our Context
    let launch = Box::new(Launch {
        real_fn: start_routine,
        real_arg: arg,
        ctx: cx,
    });

    // 3. invoke it with our trampoline + boxed launcher
    unsafe { __real_pthread_create(tid, attr, trampoline, Box::into_raw(launch) as *mut c_void) }
}

unsafe extern "C" {
    fn __real_pthread_create(
        tid: *mut pthread_t,
        attr: *const pthread_attr_t,
        start_routine: extern "C" fn(*mut c_void) -> *mut c_void,
        arg: *mut c_void,
    ) -> i32;
}
