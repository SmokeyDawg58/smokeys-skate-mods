//! What a release says of itself: `latest-release.txt`, uploaded with the release's zips, holds
//! the version and, for each platform, its zip's name, size and SHA-256; `latest-release.txt.sig`
//! holds the Ed25519 signature of those exact bytes under the release key's signing key
//! ([`crate::release_key`]). A build trusts a manifest only through that signature.
//!
//! A build reads the lines it knows and passes over the rest: a zip for a platform it does not
//! know, anything a later release adds. So releases can add zips (a native Linux game beside the
//! Linux server) without older builds refusing them; only a new first line, a new format, does.
//!
//! ```text
//! skate2 release 1
//! version 0.2.0
//! asset windows skate2-windows-x64.zip 48123456 <SHA-256, 64 hexadecimal digits>
//! asset linux skate2-linux-x64.zip 23123456 <SHA-256>
//! ```

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::fmt;

/// The manifest's and the signature's file names in a release.
pub const MANIFEST: &str = "latest-release.txt";
pub const SIGNATURE: &str = "latest-release.txt.sig";
/// The manifest's first line, which names its format.
const HEADER: &str = "skate2 release 1";
/// What the signature covers before the manifest's bytes, so that it signs nothing else.
const CONTEXT: &[u8] = b"skate2 release manifest\0";
/// The largest zip a manifest may name.
pub const MAX_ASSET: u64 = 1 << 30;

/// A release's `major.minor.patch`, the workspace version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Three decimal numbers joined by dots; nothing else is a version.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.').map(|part| {
            (!part.is_empty() && part.len() <= 9 && part.bytes().all(|byte| byte.is_ascii_digit())).then(|| part.parse().ok()).flatten()
        });
        let version = Self { major: parts.next()??, minor: parts.next()??, patch: parts.next()?? };
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The platform a zip is for: the game on Windows, the dedicated server on Linux.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    Linux,
}

impl Platform {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Windows, Self::Linux].into_iter().find(|platform| platform.name() == name)
    }

    /// The platform this build runs on, when releases have one for it.
    #[must_use]
    pub const fn current() -> Option<Self> {
        if cfg!(windows) {
            Some(Self::Windows)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }
}

/// One platform's zip in a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub platform: Platform,
    /// The zip's file name in the release, which its download address ends with.
    pub name: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// A release's manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub version: Version,
    pub assets: Vec<Asset>,
}

impl Manifest {
    /// Reads a manifest: its version and the zips for the platforms this build knows, every one of
    /// them valid; lines it does not know are passed over.
    ///
    /// # Errors
    /// What is wrong with the text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) {
            return Err("not a release manifest".into());
        }
        let version = lines
            .next()
            .and_then(|line| line.strip_prefix("version "))
            .and_then(Version::parse)
            .ok_or("the manifest names no version")?;
        let mut assets: Vec<Asset> = Vec::new();
        for line in lines {
            let fields: Vec<&str> = line.split(' ').collect();
            let ["asset", platform, name, size, sha256] = fields[..] else { continue };
            let Some(platform) = Platform::parse(platform) else { continue };
            if assets.iter().any(|asset| asset.platform == platform) {
                return Err(format!("the manifest names two {} zips", platform.name()));
            }
            if !valid_name(name) {
                return Err(format!("the manifest names an invalid file: {name}"));
            }
            let size = size.parse().ok().filter(|&size| size > 0 && size <= MAX_ASSET).ok_or_else(|| format!("invalid size for {name}"))?;
            let sha256 = unhex(sha256).and_then(|bytes| <[u8; 32]>::try_from(bytes).ok()).ok_or_else(|| format!("invalid SHA-256 for {name}"))?;
            assets.push(Asset { platform, name: name.to_owned(), size, sha256 });
        }
        Ok(Self { version, assets })
    }

    /// The manifest's text, as [`Manifest::parse`] reads it.
    #[must_use]
    pub fn text(&self) -> String {
        use fmt::Write as _;
        let mut text = format!("{HEADER}\nversion {}\n", self.version);
        for asset in &self.assets {
            let _ = writeln!(text, "asset {} {} {} {}", asset.platform.name(), asset.name, asset.size, hex(&asset.sha256));
        }
        text
    }

    /// The zip for `platform`, if the release has one.
    #[must_use]
    pub fn asset(&self, platform: Platform) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.platform == platform)
    }
}

/// A file name a release may hold: letters, digits, dots, dashes and underscores, not starting
/// with a dot.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// The public key of the signing key `seed` makes.
#[must_use]
pub fn public_key(seed: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
}

/// The signature file's text for a manifest's bytes.
#[must_use]
pub fn sign(seed: &[u8; 32], manifest: &[u8]) -> String {
    let signature = SigningKey::from_bytes(seed).sign(&[CONTEXT, manifest].concat());
    format!("{}\n", hex(&signature.to_bytes()))
}

/// Whether the signature file's text signs the manifest's bytes under `public_key`.
#[must_use]
pub fn verify(public_key: &[u8; 32], manifest: &[u8], signature: &str) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else { return false };
    let Some(bytes) = unhex(signature.trim()).and_then(|bytes| <[u8; 64]>::try_from(bytes).ok()) else { return false };
    key.verify_strict(&[CONTEXT, manifest].concat(), &Signature::from_bytes(&bytes)).is_ok()
}

/// Lowercase hexadecimal.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
        use fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// The bytes of an even-length hexadecimal text.
#[must_use]
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|at| text.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            version: Version { major: 0, minor: 2, patch: 10 },
            assets: vec![
                Asset { platform: Platform::Windows, name: "skate2-windows-x64.zip".into(), size: 1234, sha256: [7; 32] },
                Asset { platform: Platform::Linux, name: "skate2-linux-x64.zip".into(), size: 99, sha256: [0xab; 32] },
            ],
        }
    }

    #[test]
    fn versions_are_three_numbers_in_order() {
        assert_eq!(Version::parse("0.2.10"), Some(Version { major: 0, minor: 2, patch: 10 }));
        assert!(Version::parse("0.2.10") > Version::parse("0.2.9"));
        assert!(Version::parse("1.0.0") > Version::parse("0.99.99"));
        for bad in ["", "1", "1.2", "1.2.3.4", "1.2.x", "v1.2.3", "1.2.3-beta", "1..3", " 1.2.3", "1.2.-3"] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
        assert_eq!(Version::parse("3.4.5").map(|version| version.to_string()).as_deref(), Some("3.4.5"));
    }

    #[test]
    fn a_manifest_reads_back_what_it_writes() {
        let manifest = manifest();
        assert_eq!(Manifest::parse(&manifest.text()), Ok(manifest.clone()));
        assert_eq!(manifest.asset(Platform::Linux).map(|asset| asset.size), Some(99));
    }

    #[test]
    fn a_manifest_with_a_bad_line_for_its_platforms_is_refused() {
        let text = manifest().text();
        let windows_line = text.lines().nth(2).expect("windows line").to_owned();
        for bad in [
            text.replacen("skate2 release 1", "skate2 release 2", 1),
            text.replacen("version 0.2.10", "version 0.2", 1),
            text.replacen("skate2-linux-x64.zip", "../skate2-linux-x64.zip", 1),
            text.replacen("skate2-linux-x64.zip", "dir/skate2-linux-x64.zip", 1),
            text.replacen("skate2-linux-x64.zip", ".hidden.zip", 1),
            text.replacen(" 99 ", " 0 ", 1),
            text.replacen(" 99 ", " 99999999999 ", 1),
            text.replacen(&"ab".repeat(32), &"ab".repeat(31), 1),
            text.replacen(&"ab".repeat(32), &"zz".repeat(32), 1),
            format!("{text}{windows_line}\n"),
        ] {
            assert!(Manifest::parse(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn lines_a_later_release_adds_are_passed_over() {
        let text = manifest().text();
        let later = format!("{text}asset linux-game skate2-linux-game-x64.zip 5 {}\nnotes Faster lobbies\n", "cd".repeat(32));
        assert_eq!(Manifest::parse(&later), Ok(manifest()));
        let other = text.replacen("asset linux", "asset mac", 1);
        assert_eq!(Manifest::parse(&other).map(|manifest| manifest.assets.len()), Ok(1), "only the zips this build knows");
    }

    #[test]
    fn only_the_signing_key_signs_a_manifest() {
        let seed = crate::release_key::update_signing_seed(&"5".repeat(64));
        let key = public_key(&seed);
        let text = manifest().text();
        let signature = sign(&seed, text.as_bytes());
        assert!(verify(&key, text.as_bytes(), &signature));
        assert!(!verify(&key, text.replacen("0.2.10", "0.2.11", 1).as_bytes(), &signature), "another manifest");
        let other = public_key(&crate::release_key::update_signing_seed(&"6".repeat(64)));
        assert!(!verify(&other, text.as_bytes(), &signature), "another key");
        assert!(!verify(&key, text.as_bytes(), "00"), "a short signature");
        let bare = SigningKey::from_bytes(&seed).sign(text.as_bytes());
        assert!(!verify(&key, text.as_bytes(), &hex(&bare.to_bytes())), "a signature without the context");
    }

    #[test]
    fn the_release_key_is_64_hexadecimal_digits() {
        assert_eq!(crate::release_key::parse(&format!(" {}\r\n", "AB".repeat(32))), Some("ab".repeat(32)));
        assert_eq!(crate::release_key::parse(&"a".repeat(63)), None);
        assert_eq!(crate::release_key::parse(&"g".repeat(64)), None);
        assert_ne!(crate::release_key::update_signing_seed(&"a".repeat(64)), crate::release_key::update_signing_seed(&"b".repeat(64)));
    }
}
