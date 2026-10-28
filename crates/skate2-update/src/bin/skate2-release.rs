//! Makes a release's update files (`docs/release/auto-update.md`).
//!
//! ```text
//! skate2-release pack <zip> <file>...
//!     A platform's zip: each file at its top level, by its file name. Each file must be newer
//!     than the workspace's Cargo.toml, which names the release's version.
//! skate2-release sign [--secret <file>] [--out <folder>] [--windows <zip>] [--linux <zip>]
//!     latest-release.txt for this workspace version and its signature latest-release.txt.sig.
//! skate2-release public-key [--secret <file>]
//!     The key release builds check update signatures with.
//! ```
//!
//! The secret is `release-secret.txt` in the current folder unless `--secret` names another.

use sha2::{Digest, Sha256};
use skate2_update::manifest::{self, Asset, Manifest, Platform, Version};
use skate2_update::release_key;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        Some("pack") => pack(&arguments[1..]),
        Some("sign") => sign(&arguments[1..]),
        Some("public-key") => public_key(&arguments[1..]),
        _ => Err("usage: skate2-release pack <zip> <file>... | sign [--secret <file>] [--out <folder>] [--windows <zip>] [--linux <zip>] | public-key [--secret <file>]".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("skate2-release: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The value after each `--name` in `arguments`; anything else is refused.
fn options<'a>(arguments: &'a [String], names: &[&str]) -> Result<Vec<(&'a str, &'a str)>, String> {
    let mut found = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        let name = argument.strip_prefix("--").filter(|name| names.contains(name)).ok_or_else(|| format!("unexpected argument {argument}"))?;
        let value = rest.next().ok_or_else(|| format!("--{name} takes a value"))?;
        found.push((name, value.as_str()));
    }
    Ok(found)
}

fn option<'a>(found: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    found.iter().rev().find(|(found, _)| *found == name).map(|(_, value)| *value)
}

/// The signing seed of the release key in `--secret`, or in `release-secret.txt`.
fn seed(found: &[(&str, &str)]) -> Result<[u8; 32], String> {
    let path = option(found, "secret").unwrap_or("release-secret.txt");
    let text = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let key = release_key::parse(&text).ok_or_else(|| format!("{path} must hold 64 hexadecimal digits"))?;
    Ok(release_key::update_signing_seed(&key))
}

fn public_key(arguments: &[String]) -> Result<(), String> {
    let found = options(arguments, &["secret"])?;
    println!("{}", manifest::hex(&manifest::public_key(&seed(&found)?)));
    Ok(())
}

fn pack(arguments: &[String]) -> Result<(), String> {
    let [zip, files @ ..] = arguments else { return Err("pack takes the zip and its files".into()) };
    if files.is_empty() {
        return Err("pack takes the zip and its files".into());
    }
    // A program built before the version changed reports the old one, and the players who
    // update to it would be offered the same release again at every start.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let modified = |path: &Path| std::fs::metadata(path).and_then(|metadata| metadata.modified()).map_err(|error| format!("{}: {error}", path.display()));
    let versioned = modified(&manifest)?;
    for file in files {
        if modified(Path::new(file))? < versioned {
            return Err(format!("{file} is older than Cargo.toml, which names the release's version: build it again"));
        }
    }
    let mut writer = zip::ZipWriter::new(File::create(zip).map_err(|error| format!("{zip}: {error}"))?);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o755);
    for file in files {
        let name = Path::new(file).file_name().and_then(|name| name.to_str()).filter(|name| manifest::valid_name(name)).ok_or_else(|| format!("{file}: a release file needs a plain name"))?;
        let mut source = File::open(file).map_err(|error| format!("{file}: {error}"))?;
        writer.start_file(name, options).map_err(|error| format!("{zip}: {error}"))?;
        io::copy(&mut source, &mut writer).map_err(|error| format!("{zip}: {error}"))?;
        println!("{zip}: {name}");
    }
    writer.finish().map_err(|error| format!("{zip}: {error}"))?;
    Ok(())
}

/// A zip's entry in the manifest: its name, size and SHA-256.
fn asset(platform: Platform, path: &str) -> Result<Asset, String> {
    let name = Path::new(path).file_name().and_then(|name| name.to_str()).filter(|name| manifest::valid_name(name)).ok_or_else(|| format!("{path}: a release file needs a plain name"))?;
    let mut file = File::open(path).map_err(|error| format!("{path}: {error}"))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut size = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(|error| format!("{path}: {error}"))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        size += read as u64;
    }
    if size == 0 || size > manifest::MAX_ASSET {
        return Err(format!("{path}: {size} bytes"));
    }
    let mut sha256 = [0_u8; 32];
    sha256.copy_from_slice(&hash.finalize());
    Ok(Asset { platform, name: name.to_owned(), size, sha256 })
}

fn sign(arguments: &[String]) -> Result<(), String> {
    let found = options(arguments, &["secret", "out", "windows", "linux"])?;
    let seed = seed(&found)?;
    let version = Version::parse(env!("CARGO_PKG_VERSION")).ok_or("the workspace version is not major.minor.patch")?;
    let mut assets = Vec::new();
    for platform in [Platform::Windows, Platform::Linux] {
        if let Some(path) = option(&found, platform.name()) {
            assets.push(asset(platform, path)?);
        }
    }
    if assets.is_empty() {
        return Err("sign takes --windows and/or --linux zips".into());
    }
    let text = Manifest { version, assets }.text();
    let signature = manifest::sign(&seed, text.as_bytes());
    if !manifest::verify(&manifest::public_key(&seed), text.as_bytes(), &signature) {
        return Err("the signature does not verify".into());
    }
    let out = PathBuf::from(option(&found, "out").unwrap_or("."));
    for (name, contents) in [(manifest::MANIFEST, text.as_str()), (manifest::SIGNATURE, signature.as_str())] {
        let path = out.join(name);
        File::create(&path).and_then(|mut file| file.write_all(contents.as_bytes())).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    print!("{text}");
    println!("Publish the GitHub release tagged v{version} (not a pre-release) with these files:");
    for platform in [Platform::Windows, Platform::Linux] {
        if let Some(path) = option(&found, platform.name()) {
            println!("  {path}");
        }
    }
    println!("  {}\n  {}", out.join(manifest::MANIFEST).display(), out.join(manifest::SIGNATURE).display());
    Ok(())
}
