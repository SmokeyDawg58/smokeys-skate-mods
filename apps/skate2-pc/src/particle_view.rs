//! The PC side of `Sk8::ParticlePresentation`: the world particles
//! (`skate2_core::world_particles`) and miscboot's `particletextures.rx2`, loaded once; then
//! the presentation's updates at 60 Hz and each frame's quads, faced to that frame's camera.

use crate::gpu_renderer::{
    CameraRelativeClip, GpuRenderer, RenderResult,
    particles::{Frame, Textures},
};
use skate2_core::world_particles::{EFFECT_TYPES, EmitterWaypointData, ParticleCamera, WorldParticles};
use std::sync::Arc;
use skate2_formats::{hash::rw_hash64_string, mount::MountSet, rw4::Arena as Rw4Arena};

const TEXTURE_PATH: &str = "data\\content\\textures\\particletextures.rx2";
/// `ParticlePresentation::Update` runs once per 60 Hz presentation update.
const STEP: f32 = 1.0 / 60.0;
const MAX_STEPS: u32 = 4;

pub(crate) struct ParticleView {
    particles: Option<WorldParticles>,
    textures: Textures,
    /// Each effect type's descriptor texture (`cParticleEffectDescManager::Init` 0x82de5970).
    texture_ids: [Option<u64>; EFFECT_TYPES],
    pending: f32,
    /// Loaded once; a failed load leaves the particles undrawn.
    loaded: Option<bool>,
}

impl ParticleView {
    pub(crate) fn new() -> Self {
        Self {
            particles: None,
            textures: Textures::default(),
            texture_ids: [None; EFFECT_TYPES],
            pending: 0.0,
            loaded: None,
        }
    }

    fn load(&mut self, renderer: &GpuRenderer, mounts: &MountSet) -> RenderResult<()> {
        let particles = WorldParticles::load(mounts)?;
        let bytes = mounts.read_path(TEXTURE_PATH)?;
        let resources = Rw4Arena::split_file_resources(&bytes)?;
        let arena = Rw4Arena::parse(&resources)?;
        let owned = skate2_core::mounted_character_scene::own_texture_arena(&arena, &resources)?;
        self.texture_ids = particles.textures().map(|name| {
            let name = name?;
            let id = rw_hash64_string(name);
            owned
                .textures
                .iter()
                .find(|texture| texture.guid == id || texture.name.eq_ignore_ascii_case(name))
                .map(|texture| texture.guid)
        });
        if std::env::var_os("SKATE2_TRACE_PARTICLES").is_some() {
            eprintln!(
                "particles: {} emitter waypoints, textures {:?}, arena {:?}",
                particles.waypoint_count(),
                particles.textures(),
                owned.textures.iter().map(|t| (&t.name, t.guid)).collect::<Vec<_>>()
            );
        }
        self.textures = renderer.particle_textures(&owned.textures)?;
        self.particles = Some(particles);
        Ok(())
    }

    /// The presentation's updates for `dt` of presented time, then this frame's quads.
    pub(crate) fn frame(
        &mut self,
        renderer: &GpuRenderer,
        mounts: &MountSet,
        camera: ParticleCamera,
        relative: &CameraRelativeClip,
        dt: f32,
        local: Option<[f32; 3]>,
        waypoints: &[Arc<EmitterWaypointData>],
    ) -> Option<Frame> {
        if self.loaded.is_none() {
            let loaded = self.load(renderer, mounts);
            if let Err(error) = &loaded {
                eprintln!("world particles not loaded ({error}); smoke and steam stay undrawn");
            }
            self.loaded = Some(loaded.is_ok());
        }
        let particles = self.particles.as_mut()?;
        particles.sync_waypoints(waypoints.to_vec());
        self.pending += dt.clamp(0.0, MAX_STEPS as f32 * STEP);
        let mut steps = 0;
        while self.pending >= STEP && steps < MAX_STEPS {
            particles.update(&camera, local, STEP, true);
            self.pending -= STEP;
            steps += 1;
        }
        let quads = particles.vertices(&camera);
        if std::env::var_os("SKATE2_TRACE_PARTICLES").is_some() {
            static FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 240 == 0 {
                let counts: Vec<usize> = quads.iter().map(|q| q.len() / 4).collect();
                eprintln!(
                    "particles: {} waypoints, quads by type {counts:?} at {:?}",
                    particles.waypoint_count(),
                    camera.position
                );
            }
        }
        renderer.particle_frame(&quads, &self.texture_ids, &self.textures, relative)
    }
}
