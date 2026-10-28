// Move only known, retired updater files out of the game folder.
use std::path::Path;
use std::os::windows::fs::MetadataExt;
const FILES: &[(&str, &[&str])] = &[
    ("Update Skate 2.exe", &[
        "5beb3280b40450b6c5bb98c5d900f348366e3dc98afd8c94328560fb06954077",
        "ab6f481e7db7673ed83a372d442fcbdb65116106ee39266e0d45fcc25a470c27",
        "47e5df4a6f5b36be3f9dfb081e6b3270f01fd85100ee0c0309c8250ca43d57d6"]),
    ("UPDATER-LICENSE.txt", &["86c094ba94cd8a2762c93d0a806a0300ee10be72d5de61a00aa3252b8b67e0a6"]),
    ("UPDATER-SOURCE.zip", &[
        "db9fd85e0ca922034bbf8224c7e3d748cda811d8963213c3ca8156d8c9e8f1d5",
        "d0bd4fd19157238a27e653abfa65c9d7395b4bfa3fa4de7478d104bfb3953283",
        "fc93e91c68573d24d5b507e4d7cfa4f845b92b1ce4e2e31c8875f813049f71de"]),
    ("MANUAL-UPDATES.txt", &[
        "1de52d55a924edc2ce8737cb1bb5cfc7ae4ec586c0b6811bee9fec20c1146fb4",
        "7628c16b09424cba4cf6b800195520577c8a5f489dd352d739b5d610f6ef7849"]),
    ("INSTALL.txt", &["baf1983ba2951966708c6d12319fc999dd076fe8b3da077ae8cd31ea53f4ad5b"]),
];
fn safe_directory(path:&Path)->bool {
    if !path.exists() && std::fs::create_dir(path).is_err() {return false;}
    std::fs::symlink_metadata(path).is_ok_and(|m|m.is_dir() && m.file_attributes() & 0x400 == 0)
}
/// Never touch unrecognized files or existing backups; missing or locked files are kept.
pub(super) fn archive_known(root:&Path, hash_file:&dyn Fn(&Path)->Option<String>) {
    let candidates:Vec<_>=FILES.iter().filter_map(|(name,hashes)| {
        let path=root.join(name);
        let valid=hash_file(&path).is_some_and(|hash|hashes.contains(&hash.as_str()));
        valid.then_some((*name,path))
    }).collect();
    if candidates.is_empty(){return;}
    let parent=root.join("mod-update-backups");
    if !safe_directory(&parent){return;}
    let backup=parent.join("legacy-updater");
    if !safe_directory(&backup){return;}
    for (name,source) in candidates {
        let target=backup.join(name);
        if !target.exists(){let _=std::fs::rename(source,target);}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moves_only_recognized_files_and_preserves_existing_backup() {
        let root=std::env::temp_dir().join(format!("smokey-clean-test-{}",std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("Update Skate 2.exe"),"recognized").unwrap();
        std::fs::write(root.join("UPDATER-SOURCE.zip"),"user data").unwrap();
        std::fs::write(root.join("update-state.json"),"keep").unwrap();
        let fake=|p:&Path|match std::fs::read_to_string(p).ok().as_deref(){Some("recognized")=>Some(FILES[0].1[0].into()),_=>Some("different".into())};
        archive_known(&root,&fake);
        assert!(!root.join("Update Skate 2.exe").exists());
        assert!(root.join("mod-update-backups/legacy-updater/Update Skate 2.exe").is_file());
        assert_eq!(std::fs::read_to_string(root.join("UPDATER-SOURCE.zip")).unwrap(),"user data");
        assert_eq!(std::fs::read_to_string(root.join("update-state.json")).unwrap(),"keep");
        std::fs::write(root.join("Update Skate 2.exe"),"recognized").unwrap();
        archive_known(&root,&fake);assert!(root.join("Update Skate 2.exe").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
