use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    fs::copy("ld/loader.x", out_dir.join("loader.x")).unwrap();
    println!("cargo:rerun-if-changed=ld/loader.x");

    #[cfg(feature = "esp32c3")]
    let chip = "esp32c3";
    #[cfg(feature = "esp32c6")]
    let chip = "esp32c6";
    #[cfg(feature = "esp32h2")]
    let chip = "esp32h2";

    
    {
        fs::copy(format!("ld/{}.x", chip), out_dir.join(format!("{}.x", chip))).unwrap();
        println!("cargo:rerun-if-changed=ld/{}.x", chip);
        println!("cargo:rustc-link-arg=-Tld/{}.x", chip);
    }
}
