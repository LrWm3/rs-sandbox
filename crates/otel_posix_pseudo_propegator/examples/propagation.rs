// examples/propagation.rs
use opentelemetry::{
    Context, global,
    trace::{TraceContextExt, Tracer},
};
use std::thread;

fn main() {
    // 1. Install a simple in-memory tracer provider
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    global::set_tracer_provider(provider);
    let tracer = global::tracer("prop-test");

    // 2. Start & attach a parent span
    let span = tracer.start("parent");
    let cx = opentelemetry::Context::current_with_span(span);
    let parent_id = cx.span().span_context().span_id().to_string();
    let _guard = cx.attach();

    println!("parent-span={}", parent_id);
    println!(
        "Parent span active: {}",
        Context::current().has_active_span()
    );

    // 3. Spawn a thread (this will go through your LD_PRELOAD shim)
    let handle = thread::spawn(move || {
        let child_id = opentelemetry::Context::current()
            .span()
            .span_context()
            .span_id()
            .to_string();
        println!("child-span={}", child_id);
    });

    handle.join().unwrap();
}
