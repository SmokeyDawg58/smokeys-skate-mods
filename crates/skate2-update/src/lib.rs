//! The game's and the dedicated server's updates from the project's GitHub releases (PC port
//! addition). A release holds a zip for each platform (Windows: the game; Linux: the game and the
//! dedicated server), `latest-release.txt` naming the version and each zip's size and SHA-256,
//! and that manifest's Ed25519 signature. A release build carries the public key of the release
//! key's signing key (`apps/build_secret.rs`), so it takes only releases signed with the same
//! `release-secret.txt`; a development build never updates. See `docs/release/auto-update.md`.

mod fetch;
mod install;
pub mod manifest;
pub mod release_key;
mod updater;

pub use install::{cleanup, restart};
pub use manifest::{Platform, Version};
pub use updater::{Config, Failure, Status, Updater};

/// The newest release's version when it is newer than this build and has a zip for this
/// platform.
///
/// # Errors
/// What stopped the check.
pub fn newer_release(config: &Config) -> Result<Option<Version>, String> {
    let manifest = config.latest()?;
    Ok(manifest.asset(config.platform).filter(|_| manifest.version > config.version).map(|_| manifest.version))
}

/// Checks for the newest release and, when it is newer, puts it in place, all on this thread:
/// the dedicated server's `--update`. The version it installed, if any; `progress` hears the
/// bytes downloaded and the zip's size.
///
/// # Errors
/// What stopped the check or the update; the files are then as they were.
pub fn update_now(config: &Config, progress: &mut dyn FnMut(u64, u64)) -> Result<Option<Version>, String> {
    let manifest = config.latest()?;
    let Some(asset) = manifest.asset(config.platform).filter(|_| manifest.version > config.version) else { return Ok(None) };
    let total = asset.size;
    config.update(manifest.version, asset, &mut |done| progress(done, total)).map_err(|failure| failure.message)?;
    Ok(Some(manifest.version))
}

/// `<program> --update`: what an earlier update left is deleted, then the newest release is put
/// in place of this one with its progress printed. False when this build does not update or the
/// update failed, which leaves the files as they were.
#[must_use]
pub fn update_from_command_line(config: Option<&Config>, program: &str) -> bool {
    let Some(config) = config else {
        eprintln!("this build does not update: only a release build that names its repository does (docs/release/auto-update.md)");
        return false;
    };
    let _ = cleanup(&config.directory);
    println!("{program} {}: looking for a newer release in {}", config.version, config.repository);
    let mut shown = 0;
    let result = update_now(config, &mut |done, total| {
        let percent = done.saturating_mul(100) / total.max(1);
        if percent >= shown {
            println!("downloading: {percent}%");
            shown = percent + 10;
        }
    });
    match result {
        Ok(Some(version)) => {
            println!("release {version} is in place: start {program} again to run it");
            true
        }
        Ok(None) => {
            println!("this is the newest release");
            true
        }
        Err(error) => {
            eprintln!("update failed, nothing changed: {error}");
            false
        }
    }
}

/// A server's start: what an earlier update left is deleted, and a thread of its own logs when a
/// newer release is out and how to get it.
pub fn announce_newer_release(config: Option<Config>, program: &'static str) {
    let Some(config) = config else { return };
    let _ = cleanup(&config.directory);
    std::thread::spawn(move || match newer_release(&config) {
        Ok(Some(version)) => eprintln!("release {version} is out (this is {}): stop the server and run {program} --update", config.version),
        Ok(None) => {}
        Err(error) => eprintln!("update check failed: {error}"),
    });
}
