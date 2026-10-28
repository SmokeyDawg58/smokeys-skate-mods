//! `Sk8::ParticlePresentation`'s world half: `cWorldParticleController` over the emitter
//! waypoints of the registered presentation arenas (`cWaypointManager`), driving `Sk8::ParticleSystem`
//! (`cParticleManager`, `cParticleEffectFactory`, `cParticleEffect`, `cParticleEmitter`), and
//! the quads `cParticleEffect::Update` (0x82de6ac8) writes for `cParticleRender::Render`
//! (0x82de5f08). The skater and living world controllers, and `cPebbleEffect`'s own quads, are
//! not ported.
//!
//! Each frame (`ParticlePresentation::Update` 0x827ed528) the world controller
//! (`Update` 0x827ee178) starts an effect at the first waypoint of an emitter group that comes
//! within its effect's `visible_distance` of the camera, stops it (its particles live out) past
//! 1.5 times that distance squared, and keeps the ambient effect at the local skater outside
//! one box of the city; then every effect of each type updates and writes its quads.

use crate::attribute_runtime::AttributeRuntimeVault;
use skate2_formats::mount::MountSet;
use std::{collections::BTreeMap, sync::Arc};

/// `pegasus::RegisterArenaTypes` (0x829c8e34) registers `tEmitterWayPointData` as this type.
pub const EMITTER_WAYPOINT_DATA_TYPE_ID: u32 = 0x00eb_001c;
/// `eParticleEffect_Max`.
pub const EFFECT_TYPES: usize = 14;
/// `Attrib::Gen::particleeffects` (`cParticleEffectDescManager::Init` 0x82de5870).
const PARTICLE_EFFECTS_CLASS: u64 = 0x492a_cce7_fbc7_cfdf;
/// `Attrib::Gen::particleeffects_locators` (`cWaypointManager` 0x827ed998).
const LOCATORS_CLASS: u64 = 0x913e_8216_b8c0_88da;
/// `Attrib::Gen::particleeffects_ambient` (`cWorldParticleController` 0x827ecd5c).
const AMBIENT_CLASS: u64 = 0x2ed7_8b7b_06a0_0e35;
/// `default`, which `GetParticleEffect` (0x827ec384) answers for an unknown locator.
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
/// The ambient collection's effect RefSpec (`Update` 0x827ee4b0..cc).
const AMBIENT_EFFECT_ATTRIBUTE: u64 = 0x138b_a0c3_11b2_04f8;
/// `UseEffect` (0x82de7520) refuses a 65th effect of a type.
const MAX_EFFECTS_PER_TYPE: usize = 64;
/// "Leaf Particle Waypoints KDTree MemStore" (`cWaypointManager` 0x827eda24).
const MAX_WAYPOINTS: usize = 256;
/// `KDTree::FindNear` (0x827ee218): within 250 m of the camera on x and z, at most 50.
const SEARCH_RADIUS: f32 = 250.0;
const SEARCH_COUNT: usize = 50;
/// `cParticleBuffer::Init(0x7d0, ...)` (0x82de78f8) per effect type.
const MAX_PARTICLES_PER_TYPE: usize = 2000;
/// `cParticleRandomManager::mBaseSeed` (0x83024db4) plus the 42 `cParticleManager`'s
/// constructor (0x82de6584) adds.
const FIRST_SEED: u32 = 0x1234_5678 + 42;

/// One `Attrib::Gen::particleeffects` layout (0xc0 bytes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectDescriptor {
    pub velocity_variance: [f32; 3],
    pub velocity: [f32; 3],
    pub translation_variance: [f32; 3],
    pub translation: [f32; 3],
    pub rotation_velocity_variance: [f32; 3],
    pub rotation_velocity: [f32; 3],
    pub rgb_variance: [f32; 4],
    pub texture: String,
    pub visible_distance: f32,
    pub effect_type: u32,
    pub scale_variance: f32,
    pub scale_growth_rate: f32,
    pub scale: f32,
    pub life_variance: f32,
    pub life: f32,
    pub gravity: f32,
    pub emission_rate_variance: f32,
    pub emission_rate: f32,
    pub emission_duration: f32,
    pub drag: f32,
    pub alpha_fade_out_commence: i16,
    pub alpha_fade_in_complete: i16,
}

impl EffectDescriptor {
    fn parse(layout: &[u8], texture: String) -> Option<Self> {
        let float = |offset: usize| -> Option<f32> {
            Some(f32::from_be_bytes(layout.get(offset..offset + 4)?.try_into().ok()?))
        };
        let vector3 = |offset: usize| -> Option<[f32; 3]> {
            Some([float(offset)?, float(offset + 4)?, float(offset + 8)?])
        };
        let short = |offset: usize| -> Option<i16> {
            Some(i16::from_be_bytes(layout.get(offset..offset + 2)?.try_into().ok()?))
        };
        Some(Self {
            velocity_variance: vector3(0x00)?,
            velocity: vector3(0x10)?,
            translation_variance: vector3(0x20)?,
            translation: vector3(0x30)?,
            rotation_velocity_variance: vector3(0x40)?,
            rotation_velocity: vector3(0x50)?,
            rgb_variance: [float(0x60)?, float(0x64)?, float(0x68)?, float(0x6c)?],
            texture,
            visible_distance: float(0x84)?,
            effect_type: u32::from_be_bytes(layout.get(0x88..0x8c)?.try_into().ok()?),
            scale_variance: float(0x8c)?,
            scale_growth_rate: float(0x90)?,
            scale: float(0x94)?,
            life_variance: float(0x9c)?,
            life: float(0xa0)?,
            gravity: float(0xa4)?,
            emission_rate_variance: float(0xa8)?,
            emission_rate: float(0xac)?,
            emission_duration: float(0xb0)?,
            drag: float(0xb4)?,
            alpha_fade_out_commence: short(0xba)?,
            alpha_fade_in_complete: short(0xbc)?,
        })
    }
}

/// One `pegasus::tEmitterWayPoint` with its group's `m_uiGuidLocal` and attribute instance key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterWaypoint {
    pub position: [f32; 3],
    pub group: u64,
    pub locator: u64,
}

/// `tEmitterWayPointData` (24 bytes; `Fixup` 0x829c6278 rebases its pointers on the object):
/// its groups (96 bytes: `m_uiGuidLocal` +0x38, `m_AttribKey.m_InstanceKey` +0x48,
/// `m_NumEmitterWayPoints` +0x50, `m_EmitterWayPoints` +0x54) and their waypoints (48 bytes,
/// `m_Position` first). `AddBlockToKDTree` (0x827ec3c8) and `AddGroupToKDTree` (0x827eb6e0)
/// read both counts as bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterWaypointData {
    groups: Vec<EmitterGroup>,
}

#[derive(Clone, Debug, PartialEq)]
struct EmitterGroup {
    guid_local: u64,
    locator: u64,
    positions: Vec<[f32; 3]>,
}

impl EmitterWaypointData {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        parse_emitter_waypoints(bytes).map(|groups| Self {
            groups: groups
                .into_iter()
                .map(|(guid_local, locator, positions)| EmitterGroup { guid_local, locator, positions })
                .collect(),
        })
    }
}

fn parse_emitter_waypoints(bytes: &[u8]) -> Result<Vec<(u64, u64, Vec<[f32; 3]>)>, String> {
    let word = |offset: usize| -> Result<u32, String> {
        bytes
            .get(offset..offset + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| format!("emitter waypoint data field +{offset:#x} truncated"))
    };
    let long = |offset: usize| -> Result<u64, String> {
        Ok(u64::from(word(offset)?) << 32 | u64::from(word(offset + 4)?))
    };
    let float = |offset: usize| -> Result<f32, String> { word(offset).map(f32::from_bits) };
    let group_count = usize::from(*bytes.get(3).ok_or("emitter waypoint data truncated")?);
    let groups = usize::try_from(word(0x0c)?).map_err(|e| e.to_string())?;
    let mut parsed = Vec::with_capacity(group_count);
    for index in 0..group_count {
        let group = groups + index * 96;
        let count = usize::from(*bytes.get(group + 0x53).ok_or("emitter group truncated")?);
        let first = usize::try_from(word(group + 0x54)?).map_err(|e| e.to_string())?;
        let positions = (0..count)
            .map(|point| {
                let at = first + point * 48;
                Ok([float(at)?, float(at + 4)?, float(at + 8)?])
            })
            .collect::<Result<Vec<_>, String>>()?;
        parsed.push((long(group + 0x38)?, long(group + 0x48)?, positions));
    }
    Ok(parsed)
}

/// `cParticleRandomManager`: `seed = seed * 69069 + 41`, a draw in [0, 1) from its mantissa.
struct Random(u32);

impl Random {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(69069).wrapping_add(41);
        f32::from_bits(self.0 & 0x007f_ffff | 0x3f80_0000) - 1.0
    }
    /// `Build` (Vector3, 0x82de5700): z, y, then x drawn.
    fn build3(&mut self, base: [f32; 3], variance: [f32; 3]) -> [f32; 3] {
        let z = self.next().mul_add(variance[2], base[2]);
        let y = self.next().mul_add(variance[1], base[1]);
        let x = self.next().mul_add(variance[0], base[0]);
        [x, y, z]
    }
    /// `Build` (Vector4, 0x828009a8): w, z, y, then x drawn.
    fn build4(&mut self, base: [f32; 4], variance: [f32; 4]) -> [f32; 4] {
        let w = self.next().mul_add(variance[3], base[3]);
        let z = self.next().mul_add(variance[2], base[2]);
        let y = self.next().mul_add(variance[1], base[1]);
        let x = self.next().mul_add(variance[0], base[0]);
        [x, y, z, w]
    }
}

/// `tParticle`.
#[derive(Clone, Debug)]
struct Particle {
    color: [f32; 4],
    position: [f32; 3],
    velocity: [f32; 3],
    rotation: [f32; 3],
    rotation_velocity: [f32; 3],
    life: f32,
    life_percent: f32,
    scale: f32,
}

/// `cParticleEffect` with its `cParticleEmitter` (identity axes, as the world controller leaves
/// them).
#[derive(Debug)]
struct Effect {
    id: u64,
    /// `mValid`: cleared by `StopUsingEffect` (0x82de52e0); the effect then emits no more and
    /// goes once its particles have.
    valid: bool,
    particles: Vec<Particle>,
    position: [f32; 3],
    time: f32,
    next_emission: f32,
    /// The particles its last update wrote quads for.
    count: usize,
}

/// One vertex of `cParticleRender`'s quad lists: FLOAT3 position, D3DCOLOR, FLOAT2 UV.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

/// The camera `CameraPresentation::GetMatrix` gives: its axes and position.
#[derive(Clone, Copy, Debug)]
pub struct ParticleCamera {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    pub position: [f32; 3],
}

pub struct WorldParticles {
    /// `cParticleEffectDescManager::mEffectDescriptors`: the last leaf of each type.
    descriptors: [Option<EffectDescriptor>; EFFECT_TYPES],
    /// `mValidKeys`: a locator collection's `particleeffect` collection.
    locators: BTreeMap<u64, u64>,
    /// A `particleeffects` collection's `type` and `visible_distance`.
    kinds: BTreeMap<u64, (u32, f32)>,
    /// The ambient collection's effect `type`.
    ambient_type: Option<u32>,
    /// `mParticleWaypointBlockMap`: the registered blocks, in registration order.
    blocks: Vec<Arc<EmitterWaypointData>>,
    /// The KD tree's waypoints.
    waypoints: Vec<EmitterWaypoint>,
    effects: [Vec<Effect>; EFFECT_TYPES],
    /// `mParticleActiveWaypointMap`: a group's running effect.
    active: BTreeMap<u64, u64>,
    /// `mLoSpeedEffect`.
    ambient: Option<u64>,
    random: Random,
    next_id: u64,
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[2].mul_add(d[2], d[1].mul_add(d[1], d[0] * d[0]))
}

impl WorldParticles {
    /// `ParticlePresentation::Init` (0x827ec4a8) and `cWaypointManager`'s constructor: the
    /// descriptors and locator keys from `skater.vlt`.
    pub fn load(mounts: &MountSet) -> Result<Self, String> {
        let vault = AttributeRuntimeVault::try_load_mounted(
            mounts,
            "data/db/skater.vlt",
            &[PARTICLE_EFFECTS_CLASS, LOCATORS_CLASS, AMBIENT_CLASS],
        )
        .map_err(|e| format!("particle vault: {e:?}"))?;
        let mut descriptors: [Option<EffectDescriptor>; EFFECT_TYPES] = Default::default();
        let mut kinds = BTreeMap::new();
        for collection in vault.leaf_collections_sorted(PARTICLE_EFFECTS_CLASS).map_err(|e| e.to_string())? {
            let instance = vault.instance(PARTICLE_EFFECTS_CLASS, collection);
            // The layout's two plain halves around its Texture pointer.
            let Some(head) = instance.try_cached_layout_field(0, 0x80).map_err(|e| e.to_string())? else { continue };
            let Some(tail) = instance.try_cached_layout_field(0x84, 0x3c).map_err(|e| e.to_string())? else { continue };
            let mut layout = head.bytes().to_vec();
            layout.extend_from_slice(&[0; 4]);
            layout.extend_from_slice(tail.bytes());
            let texture = instance
                .try_cached_layout_text(0x80)
                .map_err(|e| e.to_string())?
                .map(|text| String::from_utf8_lossy(text.bytes()).trim_end_matches('\0').to_owned())
                .unwrap_or_default();
            let Some(descriptor) = EffectDescriptor::parse(&layout, texture) else { continue };
            kinds.insert(collection, (descriptor.effect_type, descriptor.visible_distance));
            if let Some(slot) = usize::try_from(descriptor.effect_type).ok().and_then(|t| descriptors.get_mut(t)) {
                *slot = Some(descriptor);
            }
        }
        let mut locators = BTreeMap::new();
        for collection in vault.leaf_collections_sorted(LOCATORS_CLASS).map_err(|e| e.to_string())? {
            let instance = vault.instance(LOCATORS_CLASS, collection);
            if let Some(data) = instance.try_cached_layout_field(8, 8).map_err(|e| e.to_string())? {
                let key = u64::from_be_bytes(data.bytes().try_into().map_err(|_| "locator RefSpec")?);
                locators.insert(collection, key);
            }
        }
        let ambient = vault.instance(AMBIENT_CLASS, DEFAULT_COLLECTION);
        let ambient_type = ambient
            .try_ref_spec_collection(AMBIENT_EFFECT_ATTRIBUTE)
            .map_err(|e| e.to_string())?
            .and_then(|collection| kinds.get(&collection).map(|&(effect_type, _)| effect_type));
        Ok(Self {
            descriptors,
            locators,
            kinds,
            ambient_type,
            blocks: Vec::new(),
            waypoints: Vec::new(),
            effects: Default::default(),
            active: BTreeMap::new(),
            ambient: None,
            random: Random(FIRST_SEED),
            next_id: 0,
        })
    }

    pub fn waypoint_count(&self) -> usize {
        self.waypoints.len()
    }

    /// `Presentation::RegArena`'s `AddWaypointData` (0x827ed340) for each newly registered
    /// block and `RemWaypointData` (0x827ee790) for each departed one: a departed block's groups
    /// stop their effects (`cWorldParticleController::RemWaypoint` 0x827ebdc8), and the tree is
    /// regenerated from the blocks held (`RegenerateKDTree` 0x827ec8b0).
    pub fn sync_waypoints(&mut self, blocks: Vec<Arc<EmitterWaypointData>>) {
        let same = blocks.len() == self.blocks.len() && blocks.iter().zip(&self.blocks).all(|(a, b)| Arc::ptr_eq(a, b));
        if same {
            return;
        }
        let departed: Vec<u64> = self
            .blocks
            .iter()
            .filter(|held| !blocks.iter().any(|block| Arc::ptr_eq(block, held)))
            .flat_map(|held| held.groups.iter().map(|group| group.guid_local))
            .collect();
        for group in departed {
            if let Some(id) = self.active.remove(&group)
                && let Some((_, effect)) = self.find(id)
            {
                effect.valid = false;
            }
        }
        self.blocks = blocks;
        self.waypoints.clear();
        'blocks: for block in &self.blocks {
            // AddBlockToKDTree (0x827ec3c8): only groups whose locator is a valid key.
            for group in block.groups.iter().filter(|group| self.locators.contains_key(&group.locator)) {
                // MemStore::HasSpaceFor: a group that does not fit fills the tree.
                if self.waypoints.len() + group.positions.len() > MAX_WAYPOINTS {
                    break 'blocks;
                }
                self.waypoints.extend(group.positions.iter().map(|&position| EmitterWaypoint {
                    position,
                    group: group.guid_local,
                    locator: group.locator,
                }));
            }
        }
    }

    /// The texture names of the effect types, by type.
    pub fn textures(&self) -> [Option<&str>; EFFECT_TYPES] {
        std::array::from_fn(|t| self.descriptors[t].as_ref().map(|d| d.texture.as_str()))
    }

    /// `CreateEffect` (0x82de7740) and `UseEffect` (0x82de74e8).
    fn create(&mut self, position: [f32; 3], effect_type: u32, enabled: bool) -> Option<u64> {
        let effect_type = usize::try_from(effect_type).ok().filter(|&t| t < EFFECT_TYPES)?;
        if !enabled || self.descriptors[effect_type].is_none() || self.effects[effect_type].len() >= MAX_EFFECTS_PER_TYPE {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.effects[effect_type].push(Effect {
            id,
            valid: true,
            particles: Vec::new(),
            position,
            time: 0.0,
            next_emission: 0.0,
            count: 0,
        });
        Some(id)
    }

    fn find(&mut self, id: u64) -> Option<(usize, &mut Effect)> {
        self.effects
            .iter_mut()
            .enumerate()
            .find_map(|(t, effects)| effects.iter_mut().find(|e| e.id == id).map(|e| (t, e)))
    }

    /// `ParticlePresentation::Update`'s world controller then `cParticleManager::Update`:
    /// `enabled` is `mEnableCreation`, `local` the local skater's anchor for the ambient effect.
    pub fn update(&mut self, camera: &ParticleCamera, local: Option<[f32; 3]>, dt: f32, enabled: bool) {
        self.update_world(camera, local, enabled);
        for effect_type in 0..EFFECT_TYPES {
            let Some(descriptor) = self.descriptors[effect_type].clone() else {
                self.effects[effect_type].clear();
                continue;
            };
            let mut effects = std::mem::take(&mut self.effects[effect_type]);
            // cParticleEffectFactory::Update 0x82de6408: a stopped effect without particles goes.
            effects.retain(|effect| effect.valid || effect.count != 0);
            for effect in &mut effects {
                update_effect(effect, &descriptor, camera, dt, enabled, &mut self.random);
            }
            self.effects[effect_type] = effects;
        }
    }

    /// `cWorldParticleController::Update` (0x827ee178).
    fn update_world(&mut self, camera: &ParticleCamera, local: Option<[f32; 3]>, enabled: bool) {
        let mut near: Vec<(f32, usize)> = self
            .waypoints
            .iter()
            .enumerate()
            .filter_map(|(index, waypoint)| {
                let dx = waypoint.position[0] - camera.position[0];
                let dz = waypoint.position[2] - camera.position[2];
                let distance = dz.mul_add(dz, dx * dx);
                (distance < SEARCH_RADIUS * SEARCH_RADIUS).then_some((distance, index))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        near.truncate(SEARCH_COUNT);
        for (_, index) in near {
            let waypoint = self.waypoints[index];
            if self.active.contains_key(&waypoint.group) {
                continue;
            }
            let collection = self.locators.get(&waypoint.locator).copied().unwrap_or(DEFAULT_COLLECTION);
            let (effect_type, visible) = self.kinds.get(&collection).copied().unwrap_or((0, 0.0));
            if visible * visible > distance_squared(camera.position, waypoint.position)
                && let Some(id) = self.create(waypoint.position, effect_type, enabled)
            {
                self.active.insert(waypoint.group, id);
            }
        }
        let active: Vec<(u64, u64)> = self.active.iter().map(|(&g, &i)| (g, i)).collect();
        for (group, id) in active {
            let Some((effect_type, effect)) = self.find(id) else {
                self.active.remove(&group);
                continue;
            };
            let position = effect.position;
            let visible = self.descriptors[effect_type].as_ref().map_or(0.0, |d| d.visible_distance);
            if distance_squared(camera.position, position) > visible * visible * 1.5 {
                self.active.remove(&group);
                if let Some((_, effect)) = self.find(id) {
                    effect.valid = false;
                }
            }
        }
        // The ambient effect follows the local skater outside x (-360, 215), z (-2200, -1850).
        let Some(anchor) = local else { return };
        let inside = anchor[0] > -360.0 && 215.0 > anchor[0] && -1850.0 > anchor[2] && anchor[2] > -2200.0;
        if inside {
            if let Some(id) = self.ambient.take()
                && let Some((_, effect)) = self.find(id)
            {
                effect.valid = false;
            }
            return;
        }
        let Some(ambient_type) = self.ambient_type else { return };
        if let Some(id) = self.ambient {
            match self.find(id).map(|(effect_type, _)| effect_type) {
                Some(effect_type) if effect_type as u32 == ambient_type => {}
                Some(_) => {
                    if let Some((_, effect)) = self.find(id) {
                        effect.valid = false;
                    }
                    self.ambient = None;
                }
                None => self.ambient = None,
            }
        }
        if self.ambient.is_none() {
            self.ambient = self.create(anchor, ambient_type, enabled);
        }
        if let Some(id) = self.ambient
            && let Some((_, effect)) = self.find(id)
        {
            effect.position = anchor;
        }
    }

    /// The quads each effect type's effects wrote, for `cParticleRender::Render`.
    pub fn vertices(&self, camera: &ParticleCamera) -> [Vec<ParticleVertex>; EFFECT_TYPES] {
        std::array::from_fn(|effect_type| {
            let mut vertices = Vec::new();
            for effect in &self.effects[effect_type] {
                if effect.count == 0 {
                    continue;
                }
                for particle in effect.particles.iter().take(effect.count) {
                    if vertices.len() / 4 >= MAX_PARTICLES_PER_TYPE {
                        break;
                    }
                    push_quad(&mut vertices, particle, camera);
                }
            }
            vertices
        })
    }
}

/// `cParticleEffect::Update` (0x82de6ac8) without its quads: the emitter, then the camera test
/// that leaves an effect behind the camera without particles to draw.
fn update_effect(
    effect: &mut Effect,
    descriptor: &EffectDescriptor,
    camera: &ParticleCamera,
    dt: f32,
    enabled: bool,
    random: &mut Random,
) {
    effect.time += dt;
    let emit = enabled && effect.valid;
    effect.count = update_emitter(effect, descriptor, dt, emit, random);
    if effect.count == 0 {
        return;
    }
    let offset = [
        effect.position[0] - camera.position[0],
        effect.position[1] - camera.position[1],
        effect.position[2] - camera.position[2],
    ];
    let depth = camera.forward[2].mul_add(offset[2], camera.forward[1].mul_add(offset[1], camera.forward[0] * offset[0]));
    if 0.0 > depth {
        effect.count = 0;
    }
}

/// `cParticleEmitter::Update` (0x82de67c8): every particle, then at most one new one.
fn update_emitter(
    effect: &mut Effect,
    descriptor: &EffectDescriptor,
    dt: f32,
    emit: bool,
    random: &mut Random,
) -> usize {
    effect.particles.retain_mut(|particle| update_particle(particle, descriptor, dt));
    let count = effect.particles.len();
    let duration = descriptor.emission_duration;
    if !emit || (duration != 0.0 && duration <= effect.time) || effect.time <= effect.next_emission {
        return count;
    }
    let color = random.build4([1.0, 1.0, 1.0, 0.0], descriptor.rgb_variance);
    let offset = random.build3(descriptor.translation, descriptor.translation_variance);
    let life = (random.next()).mul_add(descriptor.life_variance, descriptor.life);
    let scale = (random.next()).mul_add(descriptor.scale_variance, descriptor.scale);
    let rotation_velocity = random.build3(descriptor.rotation_velocity, descriptor.rotation_velocity_variance);
    let velocity = random.build3(descriptor.velocity, descriptor.velocity_variance);
    effect.particles.push(Particle {
        color,
        position: std::array::from_fn(|lane| effect.position[lane] + offset[lane]),
        velocity,
        rotation: [0.0; 3],
        rotation_velocity,
        life,
        life_percent: 0.0,
        scale,
    });
    let rate = random.next().mul_add(descriptor.emission_rate_variance, descriptor.emission_rate);
    effect.next_emission = 1.0 / rate + effect.time;
    count + 1
}

/// `cParticleEmitter::UpdateParticle` (0x82de59e0): life, drag, motion, the fades, spin, growth
/// and gravity; false once the particle is over or invisible.
fn update_particle(particle: &mut Particle, descriptor: &EffectDescriptor, dt: f32) -> bool {
    particle.life_percent += dt / particle.life;
    let drag = descriptor.drag * 0.01;
    for lane in 0..3 {
        particle.velocity[lane] -= particle.velocity[lane] * drag * dt;
        particle.position[lane] = particle.velocity[lane].mul_add(dt, particle.position[lane]);
    }
    let fade_in = f32::from(descriptor.alpha_fade_in_complete) * 0.01;
    let fade_out = f32::from(descriptor.alpha_fade_out_commence).mul_add(-0.01, 1.0);
    let alpha = if particle.life_percent > fade_in {
        if particle.life_percent < fade_out {
            1.0
        } else {
            1.0 - (particle.life_percent - fade_out) / (1.0 - fade_out)
        }
    } else {
        particle.life_percent / fade_in
    };
    particle.color[3] = alpha;
    for lane in 0..3 {
        particle.rotation[lane] = particle.rotation_velocity[lane].mul_add(dt, particle.rotation[lane]);
    }
    particle.scale = descriptor.scale_growth_rate.mul_add(dt, particle.scale);
    particle.velocity[1] -= descriptor.gravity * dt;
    particle.life_percent < 1.0 && alpha > 0.0
}

/// One particle's quad (0x82de6c04..6f08): the x and y columns of its rotation
/// (z by y by x), in the camera's axes, at half its scale around its position.
fn push_quad(vertices: &mut Vec<ParticleVertex>, particle: &Particle, camera: &ParticleCamera) {
    let (sx, cx) = particle.rotation[0].sin_cos();
    let (sy, cy) = particle.rotation[1].sin_cos();
    let (sz, cz) = particle.rotation[2].sin_cos();
    let first = [cy * cz, cy * sz, -sy];
    let second = [sy * sx * cz - cx * sz, sy * sx * sz + cx * cz, cy * sx];
    let world = |v: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|lane| {
            camera.forward[lane].mul_add(v[2], camera.up[lane].mul_add(v[1], camera.right[lane] * v[0]))
        })
    };
    let half = particle.scale * 0.5;
    let a = world(first).map(|lane| lane * half);
    let b = world(second).map(|lane| lane * half);
    let p = particle.position;
    // fctiwz of each lane times 255, shifted together unmasked, then stored as the D3DCOLOR
    // bytes A, R, G, B: a lane outside 0..1 spills into its neighbours as on the console.
    let [red, green, blue, alpha] = particle.color.map(|lane| (lane * 255.0) as i32 as u32);
    let packed = (((red << 8 | green) << 8 | blue) << 8) | alpha;
    let byte = |shift: u32| ((packed >> shift) & 0xff) as f32 / 255.0;
    let color = [byte(24), byte(16), byte(8), byte(0)];
    let corner = |sa: f32, sb: f32| -> [f32; 3] { std::array::from_fn(|lane| p[lane] + sa * a[lane] + sb * b[lane]) };
    for (position, uv) in [
        (corner(-1.0, -1.0), [0.0, 0.0]),
        (corner(-1.0, 1.0), [0.0, 1.0]),
        (corner(1.0, 1.0), [1.0, 1.0]),
        (corner(1.0, -1.0), [1.0, 0.0]),
    ] {
        vertices.push(ParticleVertex { position, color, uv });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_random_stream_and_fades_follow_the_native_formulas() {
        let mut random = Random(FIRST_SEED);
        let first = random.next();
        let expected = FIRST_SEED.wrapping_mul(69069).wrapping_add(41);
        assert_eq!(first, f32::from_bits(expected & 0x007f_ffff | 0x3f80_0000) - 1.0);
        let descriptor = EffectDescriptor {
            alpha_fade_in_complete: 20,
            alpha_fade_out_commence: 50,
            ..EffectDescriptor::default()
        };
        let mut particle = Particle {
            color: [1.0; 4],
            position: [0.0; 3],
            velocity: [0.0; 3],
            rotation: [0.0; 3],
            rotation_velocity: [0.0; 3],
            life: 1.0,
            life_percent: 0.0,
            scale: 1.0,
        };
        assert!(update_particle(&mut particle, &descriptor, 0.1));
        assert!((particle.color[3] - 0.5).abs() < 1e-6);
        assert!(update_particle(&mut particle, &descriptor, 0.2));
        assert_eq!(particle.color[3], 1.0);
        assert!(update_particle(&mut particle, &descriptor, 0.45));
        assert!((particle.color[3] - 0.5).abs() < 1e-5);
        assert!(!update_particle(&mut particle, &descriptor, 0.3));
    }
}
