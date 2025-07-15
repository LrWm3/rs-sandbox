# otel_posix_pseudo_propagator

A lightweight Rust-based proxy library for propagating OpenTelemetry contexts across POSIX threads by intercepting `pthread_create`. This crate builds as a `cdylib` and can be loaded at runtime to automatically capture and attach trace contexts when new threads are spawned, ensuring seamless distributed tracing in multi-threaded native applications.

## Features

- **Context Propagation**: Automatically captures the current OpenTelemetry `Context` before thread creation and restores it inside the new thread.
- **Trampoline Function**: Uses a safe trampoline to invoke the original thread entry point under the captured `Context` guard.
- **Zero-Code Changes**: No modifications required in application source; works via `LD_PRELOAD` or dynamic linker injection.
- **Minimal Overhead**: Directly wraps and links to `pthread_create`, ensuring low performance impact.

## Prerequisites

- Rust toolchain (edition 2024)
- `cargo` build system
- Compatible POSIX environment (Linux, macOS)
- OpenTelemetry collector or backend (Jaeger, Zipkin, etc.) running for exporting spans

## Usage

### LD_PRELOAD Injection

Use `LD_PRELOAD` (or `DYLD_INSERT_LIBRARIES` on macOS) to inject the library into your native application at runtime:

```bash
# On Linux
export LD_PRELOAD=$(pwd)/target/release/libotel_posix_pseudo_propegator.so

# Configure the OpenTelemetry exporter, e.g., Jaeger
export OTEL_EXPORTER_JAEGER_AGENT_ENDPOINT="http://localhost:6831"

# Run your application
./my_native_app
```

Threads created by your application (via `pthread_create`) will now inherit the active OpenTelemetry context and continue tracing spans transparently across thread boundaries.

### Direct Linking

Alternatively, link the library directly when building your C/Rust application by passing the crate as a linker argument:

```bash
rustc main.rs -L target/release -lotel_posix_pseudo_propegator
```

Note: Ensure the dynamic library is located in your system's library search path (e.g., `/usr/local/lib`) or use `LD_LIBRARY_PATH`.

## Example

```c
#include <pthread.h>
#include <stdio.h>

void* worker(void* arg) {
    // This function runs with the parent thread's trace context attached
    printf("Hello from worker thread!\n");
    return NULL;
}

int main() {
    pthread_t tid;
    pthread_create(&tid, NULL, worker, NULL);
    pthread_join(tid, NULL);
    return 0;
}
```

Build and run under `LD_PRELOAD` to see spans correctly propagated into `worker`.

## License

This project is licensed under the [Apache-2.0 License](LICENSE).
