//! One startup check against Smokey's signed shared updater, with no watcher.
use skate2_core::ui::update_prompt::{UpdateRequest,UpdateStatus};
use std::sync::atomic::Ordering;
use std::path::PathBuf;
const HELPER:&[u8]=include_bytes!(concat!(env!("OUT_DIR"),"/smokey-updater.exe"));
const REVISION:&str="2";
mod legacy_cleanup { include!("legacy_updater_cleanup.rs"); }
fn hash(bytes:&[u8])->String {use sha2::{Digest,Sha256};Sha256::digest(bytes).iter().map(|v|format!("{v:02x}")).collect()}

/// Install the embedded public helper beside this EXE without overwriting an
/// unrelated file. Its marker records only the helper version and checksum.
pub(super) fn install_helper()->Option<PathBuf> {
    use std::io::Write;use std::os::windows::fs::MetadataExt;
    if std::env::var_os("SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW").is_some() || HELPER.is_empty(){return None;}
    let root=std::env::current_exe().ok()?.parent()?.to_owned();
    let primary=root.join("Smokey's Skate Mods Updater.exe");let marker=root.join("smokey-updater-version.txt");
    let wanted=hash(HELPER);
    let existing=|path:&std::path::Path|->Option<String>{
        let meta=std::fs::symlink_metadata(path).ok()?;
        if !meta.is_file() || meta.file_attributes()&0x400!=0 || meta.len()>8_000_000{return None;}
        Some(hash(&std::fs::read(path).ok()?))
    };
    let target=match existing(&primary) {
        Some(found) if found==wanted=>primary,
        Some(_)=>root.join(format!("Smokey's Skate Mods Updater-{}.exe",&wanted[..8])),
        None if primary.exists()=>return None,
        None=>primary,
    };
    if target.exists(){let found=existing(&target)?;if found!=wanted{return None;}}
    else {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        let temporary=root.join(format!(".smokey-updater-{}-{stamp}.tmp",std::process::id()));
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).ok()?;
        if f.write_all(HELPER).and_then(|()|f.sync_all()).is_err(){let _=std::fs::remove_file(&temporary);return None;}
        drop(f);if std::fs::rename(&temporary,&target).is_err(){let _=std::fs::remove_file(&temporary);return None;}
    }
    if let Some(found)=existing(&target) {
        if let Ok(mut file)=std::fs::OpenOptions::new().write(true).create_new(true).open(&marker) {
            let _=file.write_all(format!("{REVISION}:{found}").as_bytes());
        }
    }
    legacy_cleanup::archive_known(&root,&existing);
    Some(target)
}
pub(super) struct SharedCheck {path:PathBuf,receiver:Option<std::sync::mpsc::Receiver<(Option<i32>,String)>>,status:UpdateStatus,current:String,opening:bool}
impl SharedCheck {
    pub(super) fn new(path:PathBuf)->Self {let mut this=Self{path,receiver:None,status:UpdateStatus::Unavailable,current:"Local preview".into(),opening:false};this.check();this}
    pub(super) fn current(&self)->&str {&self.current}
    fn check(&mut self){
        if self.receiver.is_some(){return;}
        self.status=UpdateStatus::Checking;
        let path=self.path.clone();let(tx,rx)=std::sync::mpsc::channel();
        if std::thread::Builder::new().name("Smokey mod update check".into()).spawn(move||{
            use std::os::windows::process::CommandExt;
            let output=std::process::Command::new(&path).arg("--check-only").current_dir(path.parent().unwrap())
                .creation_flags(0x0800_0000).output().ok();
            let result=output.map(|o|(o.status.code(),String::from_utf8_lossy(&o.stdout).trim().to_owned())).unwrap_or((None,String::new()));
            let _=tx.send(result);
        }).is_ok(){self.receiver=Some(rx);}else{self.status=UpdateStatus::Unavailable;}
    }
    pub(super) fn status(&mut self)->UpdateStatus {
        if let Some(rx)=&self.receiver {match rx.try_recv(){
            Ok((code,version))=>{
                if let Ok(seq)=version.parse::<u64>() {if seq>0 {self.current=format!("Update {seq}");}}
                self.status=match code {Some(0)=>UpdateStatus::UpToDate,Some(10)=>UpdateStatus::Available{version:"Smokey mod".into()},Some(11)=>UpdateStatus::GameUpdatePending,_=>UpdateStatus::Unavailable};
                self.receiver=None;
            },
            Err(std::sync::mpsc::TryRecvError::Disconnected)=>{self.receiver=None;self.status=UpdateStatus::Unavailable;},_=>{}
        }}
        if self.opening{UpdateStatus::OpeningUpdater}else{self.status.clone()}
    }
    pub(super) fn request(&mut self,request:UpdateRequest){match request {
        UpdateRequest::Check=>self.check(),UpdateRequest::Install=>{self.opening=true;super::OPEN_SMOKEY.store(true,Ordering::Relaxed);}
    }}
}
