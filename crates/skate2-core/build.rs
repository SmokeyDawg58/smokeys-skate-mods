use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=SKATE3_PARK_ASSETS");
    let generated = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("park_assets.rs");
    let contents = if let Some(path) = env::var_os("SKATE3_PARK_ASSETS") {
        let path = PathBuf::from(path);
        println!("cargo:rerun-if-changed={}", path.display());
        assert!(path.is_file(), "SKATE3_PARK_ASSETS does not name a file");
        format!("static BUNDLED_PARK_ASSETS: &[u8] = include_bytes!({:?});", path.to_string_lossy())
    } else {
        "static BUNDLED_PARK_ASSETS: &[u8] = &[];".to_owned()
    };
    fs::write(generated, contents).expect("write park asset definition");
}
