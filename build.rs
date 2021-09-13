use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    fs::copy("ld/loader.x", out_dir.join("loader.x")).unwrap();
    println!("cargo:rerun-if-changed=ld/loader.x");

    // TODO feature gate per chip?
    fs::copy("ld/esp32c3.x", out_dir.join("esp32c3.x")).unwrap();
    println!("cargo:rerun-if-changed=ld/rom.x");
}