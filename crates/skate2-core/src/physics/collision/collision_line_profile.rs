//! Optional aggregate host diagnostics; does not change query scheduling/results.
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Default)]
struct Stats {
    rays: u64,
    meshes: u64,
    survivors: u64,
    units: u64,
    triangles: u64,
    gp: Duration,
    intersection: Duration,
    mesh: Duration,
}
thread_local! { static STATS: RefCell<Option<Stats>> = const { RefCell::new(None) }; }
pub(crate) struct Scope {
    label: &'static str,
    start: Instant,
    previous: Option<Stats>,
}
pub(crate) fn begin(label: &'static str) -> Option<Scope> {
    if std::env::var_os("SKATE2_PROFILE_COLLISION").is_none() {
        return None;
    }
    let previous = STATS.with(|s| s.replace(Some(Stats::default())));
    Some(Scope {
        label,
        start: Instant::now(),
        previous,
    })
}
pub(crate) fn enabled() -> bool {
    STATS.with(|s| s.borrow().is_some())
}
pub(crate) fn ray() {
    STATS.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.rays += 1;
        }
    });
}
pub(crate) fn collection(meshes: u64, survivors: u64) {
    STATS.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.meshes += meshes;
            s.survivors += survivors;
        }
    });
}
pub(crate) fn mesh(
    units: u64,
    triangles: u64,
    gp: Duration,
    intersection: Duration,
    total: Duration,
) {
    STATS.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.units += units;
            s.triangles += triangles;
            s.gp += gp;
            s.intersection += intersection;
            s.mesh += total;
        }
    });
}
impl Drop for Scope {
    fn drop(&mut self) {
        let Some(s) = STATS.with(|slot| slot.replace(self.previous)) else {
            return;
        };
        let total = self.start.elapsed();
        let message = format!(
            "diagnostic line internals {}: collections={} meshes={} survivors={} units={} triangles={} gp={}ms intersection={}ms mesh_other={}ms collection_other={}ms total={}ms",
            self.label,
            s.rays,
            s.meshes,
            s.survivors,
            s.units,
            s.triangles,
            s.gp.as_secs_f64() * 1000.0,
            s.intersection.as_secs_f64() * 1000.0,
            s.mesh.saturating_sub(s.gp + s.intersection).as_secs_f64() * 1000.0,
            total.saturating_sub(s.mesh).as_secs_f64() * 1000.0,
            total.as_secs_f64() * 1000.0
        );
        eprintln!("{message}");
    }
}
