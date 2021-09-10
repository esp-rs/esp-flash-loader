use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    fs::copy("ld/loader.x", out_dir.join("loader.x")).unwrap();
    println!("cargo:rerun-if-changed=ld/loader.x");

    // fs::copy("ld/rom.x", out_dir.join("rom.x")).unwrap();
    // println!("cargo:rerun-if-changed=ld/rom.x");
}