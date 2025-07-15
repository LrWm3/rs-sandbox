// build.rs
fn main() {
    // Note: CARGO_CFG_TARGET_OS is set by Cargo for the current compile target
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        // emit a linker-arg only on Linux
        println!("cargo:rustc-link-arg=-Wl,-wrap,pthread_create");
    }
}
