//! `SKATE2_WGPU_LOG`: wgpu's warnings and errors on stderr, with `WGPU_VALIDATION=1` also the
//! Vulkan or DX12 validation layer's, to see why a GPU refuses the renderer.

struct Stderr;

impl log::Log for Stderr {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

static STDERR: Stderr = Stderr;

pub(crate) fn install() {
    if log::set_logger(&STDERR).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
}
