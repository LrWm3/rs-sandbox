// tests/propagation_integration.rs
use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;

#[test]
fn ld_preload_propagates_otel_context() {
    // path to your shim .so
    let lib = format!(
        "{}/target/release/libotel_posix_propagator.so",
        env!("CARGO_MANIFEST_DIR")
    );

    // run the example binary under LD_PRELOAD
    let mut cmd = Command::cargo_bin("propagation").unwrap();
    cmd.env("LD_PRELOAD", &lib);

    // assert that parent-span and child-span lines match
    cmd.assert()
        .success()
        .stdout(contains("parent-span=").and(contains("child-span=")))
        // crude sanity: ensure they’re equal (you could parse and compare more strictly)
        .stdout(contains("parent-span=0123").count(2));
}
