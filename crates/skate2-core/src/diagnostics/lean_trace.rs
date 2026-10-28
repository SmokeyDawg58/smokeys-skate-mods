//! Opt-in host observations for an intermittent visual lean discontinuity.
//! No gameplay writes, per-frame file I/O, or output through the quiet logger.
use std::{cell::RefCell, collections::VecDeque, io::Write, path::PathBuf, sync::OnceLock};

const CAPACITY: usize = 131_072;
struct Event {
    tick: u64,
    stage: &'static str,
    owner: usize,
    id: u64,
    values: [f32; 16],
    count: usize,
}
#[derive(Default)]
struct Capture {
    tick: u64,
    events: VecDeque<Event>,
}
thread_local! { static CAPTURE: RefCell<Capture> = RefCell::new(Capture::default()); }
fn path() -> Option<&'static PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("SKATE2_LEAN_TRACE").map(PathBuf::from))
        .as_ref()
}
pub fn enabled() -> bool {
    path().is_some()
}
/// Called on the same window/simulation thread before each attempted tick.
pub fn begin_tick() {
    if enabled() {
        CAPTURE.with(|capture| {
            let mut capture = capture.borrow_mut();
            if capture.events.capacity() == 0 {
                capture.events = VecDeque::with_capacity(CAPACITY);
            }
            capture.tick += 1;
        });
    }
}
/// Copies already-read values only. IDs remain integers (no lossy f32 IDs).
pub fn record(stage: &'static str, owner: usize, id: u64, values: &[f32]) {
    if !enabled() {
        return;
    }
    CAPTURE.with(|capture| {
        let mut capture = capture.borrow_mut();
        if capture.tick == 0 {
            return;
        }
        let mut event = Event {
            tick: capture.tick,
            stage,
            owner,
            id,
            values: [0.; 16],
            count: values.len().min(16),
        };
        event.values[..event.count].copy_from_slice(&values[..event.count]);
        if capture.events.len() == CAPACITY {
            capture.events.pop_front();
        }
        capture.events.push_back(event);
    });
}
/// Lives across the live window loop; saves on Escape and returned errors.
/// A forced process termination cannot run this destructor.
pub struct SaveOnExit;
impl Drop for SaveOnExit {
    fn drop(&mut self) {
        let Some(path) = path() else { return };
        let result = CAPTURE.with(|capture| -> std::io::Result<()> {
            let capture = capture.borrow();
            let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
            write!(out, "tick,stage,owner,id")?;
            for lane in 0..16 {
                write!(out, ",v{lane}")?;
            }
            writeln!(out)?;
            for event in &capture.events {
                write!(
                    out,
                    "{},{},{},{}",
                    event.tick, event.stage, event.owner, event.id
                )?;
                for lane in 0..16 {
                    if lane < event.count {
                        write!(out, ",{}", event.values[lane])?;
                    } else {
                        write!(out, ",")?;
                    }
                }
                writeln!(out)?;
            }
            out.flush()
        });
        if let Err(error) = result {
            std::eprintln!("failed to save lean capture {}: {error}", path.display());
        }
    }
}
