use crate::schema::{Manifest, valid_id};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Cache {
    entries: HashMap<PathBuf, (u64, PathBuf)>,
    stamps: HashMap<PathBuf, (u64, std::time::SystemTime)>,
    session: Option<PathBuf>,
    next: u64,
}

impl Cache {
    pub fn invalidate(&mut self) {
        self.stamps.clear();
    }

    pub fn materialize(&mut self, root: &Path, archive: &Path) -> Result<PathBuf, String> {
        let metadata = fs::metadata(archive).map_err(|e| e.to_string())?;
        let stamp = (
            metadata.len(),
            metadata.modified().map_err(|e| e.to_string())?,
        );
        if self.stamps.get(archive) == Some(&stamp) {
            if let Some((_, path)) = self.entries.get(archive) {
                if path.is_dir() {
                    return Ok(path.clone());
                }
            }
        }
        let bytes = fs::read(archive).map_err(|e| e.to_string())?;
        let hash = {
            let mut h = 0u64;
            for b in blake3::hash(&bytes).as_bytes() {
                h = h.wrapping_mul(31).wrapping_add(*b as u64);
            }
            h
        };
        if let Some((prev, path)) = self.entries.get(archive) {
            if *prev == hash && path.is_dir() {
                self.stamps.insert(archive.to_owned(), stamp);
                return Ok(path.clone());
            }
        }
        if self.session.is_none() {
            let cache = root.join(".cache");
            if cache.exists()
                && fs::symlink_metadata(&cache)
                    .map_err(|e| e.to_string())?
                    .file_type()
                    .is_symlink()
            {
                return Err("ZIP cache must not be a link".into());
            }
            fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
            if !cache
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(root.canonicalize().map_err(|e| e.to_string())?)
            {
                return Err("ZIP cache escapes mods folder".into());
            }
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let session = cache.join(format!("{}-{nonce}", std::process::id()));
            fs::create_dir(&session).map_err(|e| e.to_string())?;
            self.session = Some(session);
        }
        self.next += 1;
        let destination = self.session.as_ref().unwrap().join(self.next.to_string());
        fs::create_dir(&destination).map_err(|e| e.to_string())?;
        let file = fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = file
                .enclosed_name()
                .ok_or_else(|| "zip entry escapes archive".to_string())?
                .to_path_buf();
            if name
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err("zip path contains ..".into());
            }
            let out = destination.join(&name);
            if file.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut dest = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut dest).map_err(|e| e.to_string())?;
        }
        self.entries
            .insert(archive.to_owned(), (hash, destination.clone()));
        self.stamps.insert(archive.to_owned(), stamp);
        Ok(destination)
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        if let Some(path) = &self.session {
            if let (Ok(actual), Ok(parent)) =
                (path.canonicalize(), path.parent().unwrap().canonicalize())
            {
                if actual.parent() == Some(parent.as_path())
                    && actual.file_name() == path.file_name()
                {
                    let _ = fs::remove_dir_all(path);
                }
            }
        }
    }
}

pub fn validate_package(path: &Path) -> Result<Manifest, String> {
    validate_package_content(path).map(|(manifest, _)| manifest)
}

/// [`validate_package`] plus the deep check of the package's audio content overlay
/// (`audio.json`: schema and every file it names; None without one).
pub fn validate_package_content(
    path: &Path,
) -> Result<(Manifest, Option<crate::audio_content::Loaded>), String> {
    validate_package_content_at(path).map(|(manifest, audio, _)| (manifest, audio))
}

/// Owns a validated package's extraction for as long as callers read its files.
pub struct PackageRoot {
    path: PathBuf,
    _cache: Cache,
}
impl PackageRoot {
    pub fn as_path(&self) -> &Path {
        &self.path
    }
}
impl std::ops::Deref for PackageRoot {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

/// [`validate_package_content`], also returning an owner of the package's root.
/// A ZIP's extraction stays alive until the returned root is dropped.
pub fn validate_package_content_at(
    path: &Path,
) -> Result<(Manifest, Option<crate::audio_content::Loaded>, PackageRoot), String> {
    let source = path.canonicalize().map_err(|e| e.to_string())?;
    let mut cache = Cache::default();
    let root = if source.is_dir() {
        source.clone()
    } else {
        let parent = source.parent().ok_or("Missing package parent")?;
        cache.materialize(parent, &source)?
    };
    let bytes = read_bounded(&root, "mod.json", 64 * 1024)?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate()?;
    let _ = valid_id(&manifest.id);
    let entry = read_bounded(&root, &manifest.entry, 256 * 1024)?;
    let source = std::str::from_utf8(&entry).map_err(|e| e.to_string())?;
    if source.starts_with('\u{1b}') {
        return Err("Lua bytecode is unsupported".into());
    }
    mlua::Lua::new()
        .load(source)
        .set_mode(mlua::chunk::ChunkMode::Text)
        .into_function()
        .map_err(|e| e.to_string())?;
    let audio = crate::audio_content::load(&root)?;
    Ok((
        manifest,
        audio,
        PackageRoot {
            path: root,
            _cache: cache,
        },
    ))
}

pub fn read_bounded(root: &Path, relative: &str, limit: u64) -> Result<Vec<u8>, String> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || relative.contains(':')
    {
        return Err("Use a relative path without traversal".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let path = root.join(path).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(&root) {
        return Err("Asset escapes mod root".into());
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Asset too large".into());
    }
    Ok(bytes)
}

pub fn fingerprint(root: &Path) -> Result<u64, String> {
    let mut paths = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                walk(&path, out)?;
            } else {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(root, &mut paths)?;
    paths.sort();
    let mut hasher = blake3::Hasher::new();
    for path in paths {
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy();
        hasher.update(rel.as_bytes());
        hasher.update(&[0]);
        hasher.update(&fs::read(&path).map_err(|e| e.to_string())?);
        hasher.update(&[0]);
    }
    let hash = hasher.finalize();
    let mut out = 0u64;
    for b in hash.as_bytes() {
        out = out.wrapping_mul(31).wrapping_add(*b as u64);
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn write_temp(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut f = fs::File::create(path).map_err(|e| e.to_string())?;
    f.write_all(bytes).map_err(|e| e.to_string())
}

/// Unchanged files are statted, not reread. Explicit rescans force content reads.
#[derive(Default)]
pub(crate) struct Fingerprints {
    entries: HashMap<PathBuf, (Vec<(PathBuf, u64, std::time::SystemTime)>, u64)>,
}
impl Fingerprints {
    #[cfg(test)]
    pub fn get(&mut self, root: &Path, force: bool) -> Result<u64, String> {
        self.get_with_changes(root, force).map(|(hash, _)| hash)
    }
    /// The package's fingerprint and the files (relative, `/`-separated) whose size or time
    /// changed, appeared or went since the last call (every file on the first call).
    pub fn get_with_changes(
        &mut self,
        root: &Path,
        force: bool,
    ) -> Result<(u64, Vec<String>), String> {
        fn walk(
            dir: &Path,
            out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>,
        ) -> Result<(), String> {
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let m = entry.metadata().map_err(|e| e.to_string())?;
                if m.is_dir() {
                    walk(&entry.path(), out)?;
                } else {
                    out.push((
                        entry.path(),
                        m.len(),
                        m.modified().map_err(|e| e.to_string())?,
                    ));
                }
            }
            Ok(())
        }
        let mut stamp = Vec::new();
        walk(root, &mut stamp)?;
        stamp.sort_by(|a, b| a.0.cmp(&b.0));
        if !force {
            if let Some((old, hash)) = self.entries.get(root) {
                if old == &stamp {
                    return Ok((*hash, Vec::new()));
                }
            }
        }
        let rel = |p: &Path| {
            p.strip_prefix(root)
                .unwrap_or(p)
                .to_string_lossy()
                .replace('\\', "/")
        };
        let changed: Vec<String> = match self.entries.get(root) {
            Some((old, _)) => {
                let mut c: Vec<String> = stamp
                    .iter()
                    .filter(|s| !old.contains(s))
                    .map(|s| rel(&s.0))
                    .collect();
                c.extend(
                    old.iter()
                        .filter(|o| !stamp.iter().any(|s| s.0 == o.0))
                        .map(|o| rel(&o.0)),
                );
                c.sort();
                c.dedup();
                c
            }
            None => stamp.iter().map(|s| rel(&s.0)).collect(),
        };
        let hash = fingerprint(root)?;
        self.entries.insert(root.to_owned(), (stamp, hash));
        Ok((hash, changed))
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;
    #[test]
    fn validated_zip_files_survive_until_the_returned_owner_is_dropped() {
        let parent = std::env::temp_dir().join(format!(
            "skate-validated-zip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&parent).unwrap();
        let archive = parent.join("test.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file("mod.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id":"zip-lifetime","api":2,"name":"ZIP lifetime","version":"1.0.0","author":"test","description":"test","entry":"main.lua"}"#).unwrap();
        zip.start_file("main.lua", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"return {}").unwrap();
        zip.finish().unwrap();
        let (_, _, root) = validate_package_content_at(&archive).unwrap();
        let extracted = root.as_path().to_owned();
        assert_eq!(fs::read(root.join("main.lua")).unwrap(), b"return {}");
        drop(root);
        assert!(!extracted.exists());
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn failed_changed_archive_is_not_cached_as_the_old_package() {
        let root = std::env::temp_dir().join(format!("skate-zip-stamp-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("test.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file("main.lua", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"return {}").unwrap();
        zip.finish().unwrap();
        let mut cache = Cache::default();
        let initial = cache.materialize(&root, &archive).unwrap();
        assert_eq!(cache.materialize(&root, &archive).unwrap(), initial);
        fs::write(&archive, b"invalid changed zip").unwrap();
        assert!(cache.materialize(&root, &archive).is_err());
        assert!(cache.materialize(&root, &archive).is_err());
        drop(cache);
        fs::remove_file(&archive).unwrap();
        fs::remove_dir(root.join(".cache")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn metadata_cache_preserves_content_hash_and_detects_add_edit_remove_and_force() {
        let root = std::env::temp_dir().join(format!("skate-fingerprint-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.lua"), b"one").unwrap();
        let mut cache = Fingerprints::default();
        let one = cache.get(&root, false).unwrap();
        assert_eq!(one, fingerprint(&root).unwrap());
        assert_eq!(one, cache.get(&root, false).unwrap());
        fs::write(root.join("asset"), b"new").unwrap();
        let two = cache.get(&root, false).unwrap();
        assert_ne!(one, two);
        fs::write(root.join("main.lua"), b"longer").unwrap();
        assert_ne!(two, cache.get(&root, false).unwrap());
        fs::remove_file(root.join("asset")).unwrap();
        let current = cache.get(&root, false).unwrap();
        assert_eq!(current, fingerprint(&root).unwrap());
        // Force must bypass even a matching metadata stamp.
        cache.entries.get_mut(&root).unwrap().1 = 0;
        assert_eq!(current, cache.get(&root, true).unwrap());
        fs::remove_file(root.join("main.lua")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    #[ignore = "headless installed-package scan benchmark"]
    fn compare_unchanged_skyline_scans() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/Skyline_Drive_Mod");
        let mut cache = Fingerprints::default();
        let expected = cache.get(&root, false).unwrap();
        let begin = std::time::Instant::now();
        for _ in 0..20 {
            assert_eq!(fingerprint(&root).unwrap(), expected);
        }
        let eager = begin.elapsed();
        let begin = std::time::Instant::now();
        for _ in 0..20 {
            assert_eq!(cache.get(&root, false).unwrap(), expected);
        }
        let cached = begin.elapsed();
        eprintln!(
            "SCAN_BENCH eager_ms={:.3} cached_ms={:.3}",
            eager.as_secs_f64() * 50.,
            cached.as_secs_f64() * 50.
        );
    }
}
