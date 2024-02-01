use std::{env, ffi::OsString, process::Command};

fn main() {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));

    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to run `rustc --version`");

    if String::from_utf8_lossy(&output.stdout).contains("nightly") {
        println!("cargo:rustc-cfg=nightly");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
