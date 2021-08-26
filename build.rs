use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    #[cfg(not(feature = "standalone"))]
    {
        fs::copy("ld/loader.x", out_dir.join("loader.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/loader.x");
    }

    #[cfg(feature = "standalone")]
    {
        fs::copy("ld/esp32c3-standalone.x", out_dir.join("esp32c3-standalone.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32c3-standalone.x");
    }
}