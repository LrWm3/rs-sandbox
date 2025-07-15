// in src/lib.rs (or wherever your spawn_with_otel lives)
use opentelemetry::Context;
use std::thread::{self};

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{
        global,
        trace::{TraceContextExt, Tracer},
    };
    use std::sync::mpsc::channel;

    #[test]
    fn test_spawn_with_otel_propagates_context() {
        // 0. Set LD_PRELOAD to the path of the otel_posix_pseudo_propegator library
        // This is typically done outside of the test, in the environment setup.

        // 1. Install a simple in-memory tracer provider
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);
        let tracer = global::tracer("test");
        println!("Tracer initialized");

        // 2. Start a span in the parent and attach it
        let span = tracer.start("parent-span");
        let cx = Context::current_with_span(span);
        // 4. Assert they match
        let parent_span_id = cx.span().span_context().span_id();
        let _parent_guard = cx.attach();

        println!(
            "Parent span active: {}",
            Context::current().has_active_span()
        );

        // 3. Spawn a thread that reads the current span ID
        let (tx, rx) = channel();
        let handle = thread::spawn(move || {
            // In the child thread, `Context::current()` should give us the same span
            let binding = Context::current();
            let child_span = binding.span();
            tx.send(child_span.span_context().span_id()).unwrap();
        });

        handle.join().unwrap();
        let child_span_id = rx.recv().unwrap();

        // 4. Assert they match
        assert_eq!(
            child_span_id, parent_span_id,
            "OTEL Context was not propagated into the child thread"
        );
    }
}
