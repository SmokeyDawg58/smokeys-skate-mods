//! Public build fingerprint. Deliberately independent from the network secret.
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn emit() {
    let root = Path::new("../..");
    let mut files = Vec::new();
    for relative in [
        "apps/skate2-pc/src",
        "apps/skate2-pc/build.rs",
        "apps/skate2-pc/Cargo.toml",
        "apps/build_secret.rs",
        "apps/test_build_metadata.rs",
        "crates",
        "vendor/skate3-engine/crates",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        let path = root.join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        collect(&path, &mut files);
    }
    files.sort();
    let mut hash = Sha256::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .expect("build input under workspace")
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(&path).expect("read build fingerprint input");
        hash.update(relative.as_bytes());
        hash.update([0]);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    println!("cargo:rustc-env=SKATE2_SOURCE_SHA256={:x}", hash.finalize());
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    let compiler = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    println!("cargo:rustc-env=SKATE2_SOURCE_REVISION={revision}");
    println!("cargo:rustc-env=SKATE2_BUILD_COMPILER={compiler}");
    println!(
        "cargo:rustc-env=SKATE2_BUILD_TIME={}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    for (source, name) in [
        ("TARGET", "SKATE2_BUILD_TARGET"),
        ("PROFILE", "SKATE2_BUILD_PROFILE"),
    ] {
        println!(
            "cargo:rustc-env={name}={}",
            std::env::var(source).expect("cargo build metadata")
        );
    }
}
fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("read source directory") {
            let entry = entry.expect("read source entry");
            if entry.file_name() != "target" {
                collect(&entry.path(), out);
            }
        }
    } else if path
        .extension()
        .is_some_and(|ext| ext == "rs" || ext == "toml" || ext == "lock")
    {
        out.push(path.into());
    }
}
