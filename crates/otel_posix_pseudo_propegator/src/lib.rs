// Cargo.toml:
// [lib] crate-type = ["cdylib"]

use libc::{RTLD_NEXT, c_char, pthread_attr_t, pthread_t};
use opentelemetry::{Context, trace::TraceContextExt};
use std::{ffi::c_void, sync::OnceLock};

type PthreadCreateFn = unsafe extern "C" fn(
    *mut pthread_t,
    *const pthread_attr_t,
    extern "C" fn(*mut c_void) -> *mut c_void,
    *mut c_void,
) -> i32;

static REAL_CREATE: OnceLock<PthreadCreateFn> = OnceLock::new();

// A little launcher holding the real fn + its arg + the OTEL Context
struct Launch {
    real_fn: extern "C" fn(*mut c_void) -> *mut c_void,
    real_arg: *mut c_void,
    ctx: Context,
}

extern "C" fn trampoline(v: *mut c_void) -> *mut c_void {
    // recover the Launch struct
    let launch: Box<Launch> = unsafe { Box::from_raw(v as *mut Launch) };
    println!(
        "Running thread with OTEL Context: {:?}",
        launch.ctx.span().span_context().span_id()
    );
    // activate the captured Context
    let _guard = launch.ctx.attach();

    // call the original thread entry point
    (launch.real_fn)(launch.real_arg)
}

unsafe fn real_pthread_create() -> PthreadCreateFn {
    *REAL_CREATE.get_or_init(|| {
        let sym = unsafe { libc::dlsym(RTLD_NEXT, b"pthread_create\0".as_ptr() as *const c_char) };
        if sym.is_null() {
            panic!("failed to find original pthread_create");
        }
        unsafe { std::mem::transmute::<*mut c_void, PthreadCreateFn>(sym) }
    })
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
        println!(
            "No active OTEL Context, calling original pthread_create directly, ctx={:?}",
            cx
        );
        // if no context, just call the original pthread_create
        return unsafe { real_pthread_create()(tid, attr, start_routine, arg) };
    }

    // 2. box up the real fn, its arg, and our Context
    let launch = Box::new(Launch {
        real_fn: start_routine,
        real_arg: arg,
        ctx: cx,
    });

    println!("Wrapping pthread_create with OTEL Context propagation");
    // 3. invoke it with our trampoline + boxed launcher
    unsafe { real_pthread_create()(tid, attr, trampoline, Box::into_raw(launch) as *mut c_void) }
}

unsafe extern "C" {
    fn __real_pthread_create(
        tid: *mut pthread_t,
        attr: *const pthread_attr_t,
        start_routine: extern "C" fn(*mut c_void) -> *mut c_void,
        arg: *mut c_void,
    ) -> i32;
}
