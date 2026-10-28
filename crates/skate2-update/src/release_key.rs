//! The release key (`release-secret.txt`, `SKATE2_RELEASE_SECRET`): 64 hexadecimal digits, kept
//! private. The build secret (`apps/build_secret.rs`) and the key that signs every release's update
//! manifest both come from it, each under its own label, so one file makes a release. The game's
//! and the server's build scripts include this file, and the release tool signs with it.

use sha2::{Digest, Sha256};

/// The key, trimmed and lowercase, when it is 64 hexadecimal digits.
#[must_use]
pub fn parse(key: &str) -> Option<String> {
    let key = key.trim().to_ascii_lowercase();
    (key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(key)
}

/// The seed of the Ed25519 key that signs the update manifests. It does not depend on the
/// version, so each release's build checks the next release's manifest.
#[must_use]
pub fn update_signing_seed(key: &str) -> [u8; 32] {
    let digest = Sha256::new().chain_update(b"skate2 update signing\0").chain_update(key).finalize();
    let mut seed = [0_u8; 32];
    seed.copy_from_slice(&digest);
    seed
}
