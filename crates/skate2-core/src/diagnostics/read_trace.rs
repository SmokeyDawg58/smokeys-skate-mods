//! Archive reads for host diagnostics, off unless set. `SKATE2_TRACE_READS`: every read on
//! stderr with its thread, size and time; a read on a thread the variable names (comma-separated,
//! `main` the window's) prints its stack as well. `SKATE2_DIAGNOSTIC_READ_DELAY_MS`: every read
//! takes that much longer, one read at a time, as on a busy hard disk.
use skate2_formats::{
    big::BigEntryKey,
    mount::{ArchiveRead, observe_reads},
};
use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

struct Settings {
    trace: bool,
    stack_threads: Vec<String>,
    delay: Duration,
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();
/// The one disk the delayed reads queue for.
static DISK: Mutex<()> = Mutex::new(());

/// Observes the archive reads when either variable is set.
pub fn install() {
    let trace = std::env::var("SKATE2_TRACE_READS").ok();
    let delay = std::env::var("SKATE2_DIAGNOSTIC_READ_DELAY_MS").ok().and_then(|ms| ms.parse().ok()).map(Duration::from_millis);
    if trace.is_none() && delay.is_none() {
        return;
    }
    let stack_threads = trace
        .iter()
        .flat_map(|names| names.split(','))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    let settings = Settings { trace: trace.is_some(), stack_threads, delay: delay.unwrap_or_default() };
    if SETTINGS.set(settings).is_ok() {
        observe_reads(observed);
    }
}

fn observed(read: &ArchiveRead<'_>) {
    let Some(settings) = SETTINGS.get() else { return };
    if !settings.delay.is_zero() {
        let _disk = DISK.lock();
        std::thread::sleep(settings.delay);
    }
    if !settings.trace {
        return;
    }
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    let archive = read.archive.file_name().map_or_else(|| read.archive.display().to_string(), |file| file.to_string_lossy().into_owned());
    let entry = match &read.entry.key {
        BigEntryKey::Name(path) => path.clone(),
        BigEntryKey::Hash(hash) => format!("{hash:08x}"),
    };
    let ms = read.elapsed.as_secs_f64() * 1000.0;
    if settings.stack_threads.iter().any(|stacked| stacked == name) {
        eprintln!("read {ms:.2} ms {} B on {name}: {archive} {entry}\n{}", read.bytes, std::backtrace::Backtrace::force_capture());
    } else {
        eprintln!("read {ms:.2} ms {} B on {name}: {archive} {entry}", read.bytes);
    }
}
