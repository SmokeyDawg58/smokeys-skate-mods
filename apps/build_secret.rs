//! The build's secret (`skate2_net::crypto::Build`), shared by the game's and the dedicated
//! server's build scripts. With a release key in `SKATE2_RELEASE_SECRET` (64 hexadecimal digits,
//! which `build-release.bat` and `build-release.sh` read from `release-secret.txt`) every build of
//! one version gets the same secret, on Windows and on Linux, so a release's game and its server
//! meet and the next version's do not. Without one a build makes 32 random bytes, and its game and
//! server match only each other. A release build also carries `SKATE2_UPDATE_PUBLIC_KEY`, the
//! public key of the release key's update signing key, which checks the releases it updates to
//! (`crates/skate2-update`); a development build has none and never updates.

use sha2::{Digest, Sha256};

#[path = "../crates/skate2-update/src/release_key.rs"]
mod release_key;

/// Hands the secret to the crate as `SKATE2_BUILD_SECRET`; `inputs` are the paths whose change
/// makes a development build's secret anew.
pub fn emit(inputs: &[&str]) {
    println!("cargo:rerun-if-env-changed=SKATE2_RELEASE_SECRET");
    for path in inputs {
        println!("cargo:rerun-if-changed={path}");
    }
    // Authorized compatibility builds may use an existing release's NETWORK
    // identity. This is a build input, never a runtime file or an update signing
    // key. The wire handshake, proof checks and server permissions stay intact.
    println!("cargo:rerun-if-env-changed=SKATE2_NETWORK_BUILD_SECRET_FILE");
    let secret = match std::env::var_os("SKATE2_NETWORK_BUILD_SECRET_FILE") {
        Some(path) => {
            assert!(std::env::var_os("SKATE2_RELEASE_SECRET").is_none(),
                "Choose either a network compatibility identity or a release signing key");
            println!("cargo:rerun-if-changed={}", std::path::Path::new(&path).display());
            let bytes = std::fs::read(path).expect("read network compatibility build input");
            <[u8; 32]>::try_from(bytes).expect("network compatibility build input must contain exactly 32 bytes")
        }
        None => match std::env::var("SKATE2_RELEASE_SECRET") {
        Ok(key) => {
            let key = release_key::parse(&key).expect("SKATE2_RELEASE_SECRET (release-secret.txt) must be 64 hexadecimal digits");
            let update = ed25519_dalek::SigningKey::from_bytes(&release_key::update_signing_seed(&key)).verifying_key();
            println!("cargo:rustc-env=SKATE2_UPDATE_PUBLIC_KEY={}", hex(update.as_bytes()));
            release(&key)
        }
        Err(_) => random(),
        },
    };
    println!("cargo:rustc-env=SKATE2_BUILD_SECRET={}", hex(&secret));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// The release key (lowercase, as `release_key::parse` gives it) and the workspace version,
/// hashed.
fn release(key: &str) -> [u8; 32] {
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo names the package version");
    let digest = Sha256::new().chain_update(b"skate2 release build\0").chain_update(key).chain_update(b"\0").chain_update(version).finalize();
    let mut secret = [0_u8; 32];
    secret.copy_from_slice(&digest);
    secret
}

fn random() -> [u8; 32] {
    let mut secret = [0_u8; 32];
    getrandom::fill(&mut secret).expect("the system's random source");
    secret
}
