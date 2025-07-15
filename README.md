# Rust Sandbox

This is a sandbox for experimenting with Rust code and libraries.

## Ideas so far

| Crate Name                     | Description                                                                                                                                                      |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `otel_posix_pseudo_propagator` | A library to propagate OpenTelemetry context across threads in native applications using `LD_PRELOAD` or direct linking.                                         |
| `quasi_arc`                    | An experimental Arc-like type that will not deallocate its contents when dropped, unless it has been cloned once before. Likely useless. In general, prefer Arc. |

## Creating a New Idea

To create a new idea, use the following command:

```bash
cargo new crates/your_new_idea --bin   # or --lib
```

## Building

To build and run the project, use:

```bash
cargo build
```

## Testing

```bash
cargo test
```

## Running

See individual crate directories for specific run commands; generally speaking
there are no specific run commands as tests drive the majority of the test cuts.
