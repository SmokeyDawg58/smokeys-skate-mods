//! Automatic diagnostics for both ordinary player launches and bounded live tests.
use sha2::{Digest, Sha256};
use skate2_core::{diagnostics::test_log, edit_skater_profile::EditSkaterProfileStore};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

pub(crate) fn install() -> Option<SupportRun> {
    let result = (|| -> io::Result<_> {
        let root = match std::env::var_os("SKATE2_TEST_LOG_ROOT") {
            Some(root) => PathBuf::from(root),
            None => support_directory().ok_or_else(||io::Error::other("support directory absent"))?.join("sessions"),
        };
        mark_unfinished(&root)?;
        let run = test_log::install(&root)?;
        let lease=fs::OpenOptions::new().read(true).write(true).create_new(true).open(run.directory().join("active.lock"))?;
        lease.try_lock().map_err(io::Error::other)?;
        fs::write(run.directory().join("session-state.txt"),"running\n")?;
        fs::write(root.join("LATEST.txt"),format!("Send this complete run folder to the mod author:\n{}\nAlso include Mod Logs/crash_reports if present. No files are uploaded automatically.\n",run.directory().display()))?;
        if let Some(directory)=support_directory() {
            fs::create_dir_all(&directory)?;
            fs::write(directory.join("READ ME - SEND LOGS.txt"),"After an online disconnect or crash, send the newest folder under sessions plus crash_reports. LATEST.txt inside sessions points to the newest run. A hard crash/forced close may leave session-state running; the next launch marks the unfinished session. No automatic uploads. These logs contain build/GPU/game diagnostics; review them before sharing.\n")?;
        }
        let manifest = format!(
            "Skate 2 test run\nLog schema: 1\nVersion: {}\nSource SHA256: {}\nGit revision: {}\nBuilt at Unix seconds: {}\nTarget: {}\nProfile: {}\nCompiler: {}\nSave: {}\nExecutable: {}\n",
            env!("CARGO_PKG_VERSION"),
            env!("SKATE2_SOURCE_SHA256"),
            env!("SKATE2_SOURCE_REVISION"),
            env!("SKATE2_BUILD_TIME"),
            env!("SKATE2_BUILD_TARGET"),
            env!("SKATE2_BUILD_PROFILE"),
            env!("SKATE2_BUILD_COMPILER"),
            EditSkaterProfileStore::default_path()?.display(),
            std::env::current_exe()?.display(),
        );
        fs::write(run.directory().join("build.txt"), &manifest)?;
        test_log::event("build", format_args!("{manifest}"));
        // Never log the build secret, account data, chat text or arbitrary env.
        for key in [
            "SKATE2_SERIAL_SIM",
            "SKATE2_NO_PRESENTATION_RESTORE",
            "SKATE2_DIAGNOSTIC_TIMER_HZ",
            "SKATE2_TRACE_SELECTED_STATES",
            "RUST_MIN_STACK",
        ] {
            test_log::event(
                "diagnostic_option",
                format_args!("{key}={:?}", std::env::var_os(key)),
            );
        }
        let directory = run.directory().to_owned();
        std::thread::Builder::new()
            .name("build fingerprint".into())
            .spawn(move || {
                let result = (|| -> io::Result<String> {
                    let mut file = fs::File::open(std::env::current_exe()?)?;
                    let mut hash = Sha256::new();
                    let mut buffer = [0_u8; 65536];
                    loop {
                        let read = file.read(&mut buffer)?;
                        if read == 0 {
                            break;
                        }
                        hash.update(&buffer[..read]);
                    }
                    Ok(format!("{:x}", hash.finalize()))
                })();
                match result {
                    Ok(hash) => {
                        test_log::event("executable_sha256", format_args!("{hash}"));
                        if let Err(error) =
                            fs::write(directory.join("executable.sha256"), format!("{hash}\n"))
                        {
                            test_log::event("fingerprint_error", format_args!("{error}"));
                        }
                    }
                    Err(error) => test_log::event("fingerprint_error", format_args!("{error}")),
                }
            })?;
        std::eprintln!("Test logs: {}", run.directory().display());
        Ok((run,lease))
    })();
    match result {
        Ok((run,lease)) => Some(SupportRun{run:Some(run),_lease:lease}),
        Err(error) => {
            std::eprintln!("Could not start test journal: {error}");
            None
        }
    }
}


static FAILED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
pub(crate) fn mark_failure(){FAILED.store(true,std::sync::atomic::Ordering::Relaxed);}
/// Diagnostic runs stay in their scratch profile; player logs live beside the EXE.
pub(crate) fn support_directory()->Option<PathBuf> {
    if let Some(root)=std::env::var_os("SKATE2_SAVE_ROOT") {return Some(PathBuf::from(root).join("Mod Logs"));}
    std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("Mod Logs")))
        .or_else(||EditSkaterProfileStore::default_path().ok().and_then(|p|p.parent().map(|p|p.join("Mod Logs"))))
}
pub(crate) struct SupportRun {run:Option<test_log::RunGuard>,_lease:fs::File}
impl Drop for SupportRun {
    fn drop(&mut self){
        if let Some(run)=self.run.take(){let path=run.directory().join("session-state.txt");drop(run);
            let state=if FAILED.load(std::sync::atomic::Ordering::Relaxed){"failed - see crash/failure reports\n"}else{"clean_exit\n"};
            let _=fs::write(path,state);
        }
    }
}
fn mark_unfinished(root:&std::path::Path)->io::Result<()> {
    let entries=match fs::read_dir(root){Ok(v)=>v,Err(e)if e.kind()==io::ErrorKind::NotFound=>return Ok(()),Err(e)=>return Err(e)};
    for entry in entries.flatten(){
        if !entry.file_type()?.is_dir() || !entry.file_name().to_string_lossy().starts_with("run-"){continue;}
        let directory=entry.path();let state=directory.join("session-state.txt");
        if fs::read_to_string(&state).ok().as_deref()==Some("running\n") {
            // An OS-held file lock distinguishes another live game instance
            // from an abrupt exit without a watcher or process enumeration.
            let Ok(lease)=fs::OpenOptions::new().read(true).write(true).open(directory.join("active.lock")) else {continue;};
            if lease.try_lock().is_ok() {
                fs::write(directory.join("UNEXPECTED EXIT.txt"),"The game did not record a clean exit. This may be a crash, forced close or power loss. See the last events and GPU/failure reports; the exact cause may be unavailable.\n")?;
                fs::write(state,"unexpected_exit\n")?;
            }
        }
    }
    Ok(())
}
