use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("assets")
        .join("iroha-zip-settings.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bin=iroha-zip-settings=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=iroha-zip-settings=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
