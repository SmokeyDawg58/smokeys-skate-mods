//! The original game's files on the host's file system (PC port addition). The Xbox 360's file
//! system ignores case, so the game's data names a file in any case: a script's movie name, an
//! audio stream's path. Windows ignores case as well; on Linux a name finds its file by ignoring
//! ASCII case.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

/// `path` as the file system spells it. When `path` is missing on a system whose names keep case
/// apart (Linux), each missing component becomes its directory's entry with the same name
/// ignoring ASCII case. `path` itself when it exists, on Windows, or when a component matches
/// nothing.
#[must_use]
pub fn on_disk(path: &Path) -> PathBuf {
    if cfg!(windows) || path.exists() {
        return path.to_owned();
    }
    let mut found = PathBuf::new();
    for component in path.components() {
        let exact = found.join(component);
        if exact.exists() {
            found = exact;
            continue;
        }
        let Component::Normal(name) = component else { return path.to_owned() };
        let directory = if found.as_os_str().is_empty() { Path::new(".") } else { found.as_path() };
        let Some(entry) = entry_ignoring_case(directory, name) else { return path.to_owned() };
        found.push(entry);
    }
    found
}

/// The bytes of the file at `path` ([`on_disk`]), or none when it is missing, a missing folder on
/// the way included: a debug build's game folder has loose configuration (`data\startup.ini`,
/// `data\juice`) that a retail game's does not.
///
/// # Errors
/// The file is there but could not be read.
pub fn read_if_present(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(on_disk(path)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The entry of `directory` named `name` ignoring ASCII case; the first in byte order when
/// several are.
fn entry_ignoring_case(directory: &Path, name: &OsStr) -> Option<OsString> {
    let name = name.to_str()?;
    std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|entry| entry.to_str().is_some_and(|entry| entry.eq_ignore_ascii_case(name)))
        .min()
}

#[cfg(test)]
mod tests {
    use super::{on_disk, read_if_present};
    use std::fs;

    #[test]
    fn a_name_in_another_case_finds_its_file() {
        let root = std::env::temp_dir().join(format!("skate2-host-path-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Data").join("Audio")).expect("folders");
        fs::write(root.join("Data").join("Audio").join("MixMapSK8.mxb"), b"mix").expect("file");
        let named = root.join("data").join("audio").join("mixmapsk8.MXB");
        assert_eq!(fs::read(on_disk(&named)).ok().as_deref(), Some(&b"mix"[..]));
        let exact = root.join("Data").join("Audio").join("MixMapSK8.mxb");
        assert_eq!(on_disk(&exact), exact);
        let missing = root.join("data").join("audio").join("missing.mxb");
        assert_eq!(on_disk(&missing), missing);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_file_or_folder_reads_as_none() {
        let root = std::env::temp_dir().join(format!("skate2-host-path-optional-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("folder");
        fs::write(root.join("data").join("startup.ini"), b"[Config]").expect("file");
        let present = read_if_present(&root.join("data").join("startup.ini")).expect("read");
        assert_eq!(present.as_deref(), Some(&b"[Config]"[..]));
        assert_eq!(read_if_present(&root.join("data").join("debugfe.ini")).expect("missing file"), None);
        let in_missing_folder = root.join("data").join("juice").join("Juice_m.ini");
        assert_eq!(read_if_present(&in_missing_folder).expect("missing folder"), None);
        let _ = fs::remove_dir_all(&root);
    }
}
