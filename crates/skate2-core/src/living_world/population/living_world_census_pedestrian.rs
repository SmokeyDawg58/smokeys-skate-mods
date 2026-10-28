//! Pedestrian CensusMan admission over the retained recipe, RNG and entity owners.
use super::{
    CensusState,
    random::random_position,
    targets::{LiveCensusRandom, PopulationKind},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        dmo_models::NULL_ASSET_ID,
        dmo_objects::{CENSUS_RANGES_CLASS, CensusCircle, ENTITIES_CLASS, evaluate_census_circle},
    },
    living_world_rain::geometry::Transform,
    living_world_recipe_manager::{
        LIVING_WORLD_MODELS_CLASS_ID, LOAD_MODEL_ATTRIBUTE_ID, attributes::RecipeAttributes,
        runtime::RecipeMan,
    },
    native_skater_random::{NativeRandomGate, NativeSkaterRandom},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    world_painter_layer::WorldPainterLayerMan,
};
use std::{fmt::Debug, sync::Arc};

fn error(value: impl Debug) -> String {
    format!("pedestrian census: {value:?}")
}

/// Native LWCommonSpawnInfo plus pedestrian flags (SpawnPedestrian82720fe0).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PedestrianSpawnInfo {
    pub(crate) entity_key: u64,
    pub(crate) model_key: u64,
    pub(crate) recipe_id: u64,
    pub(crate) transform: Transform,
    pub(crate) initial: bool,
    pub(crate) allow_culling: bool,
    pub(crate) force_spawn: bool,
    pub(crate) allow_world_collisions: bool,
    pub(crate) load_model: bool,
    pub(crate) ignore_navmesh_ground: bool,
    pub(crate) allow_hand_prop: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct PedestrianCullEntry {
    pub(crate) id: u32,
    pub(crate) position: [f32; 4],
    pub(crate) allow_culling: bool,
    pub(crate) security_guard: bool,
}

/// Borrowed adapter to the existing ModelMan / concrete Pedestrian EntityMan.
/// No population cache, IDs or entity state is stored in the census adapter.
pub(crate) trait PedestrianCensusServices<N: ReckoningNumeric> {
    fn count(&self) -> usize;
    fn contains_model(&self, recipe_id: u64) -> bool;
    fn request_spawn(
        &mut self,
        info: PedestrianSpawnInfo,
        numeric: &mut N,
    ) -> Result<Option<u32>, String>;
    fn cull_entries(&self) -> Vec<PedestrianCullEntry>;
    fn request_cull(&mut self, id: u32, random:&mut crate::native_skater_random::NativeSkaterRandom) -> Result<(), String>;
    fn set_security_outside_cull_radius(&mut self, id: u32, outside: bool) -> Result<(), String>;
    /// Source evaluates prop-driven pedestrians BEFORE ordinary population.
    /// A missing prop owner must remain an error, not an empty successful pass.
    fn prop_driven_spawn(
        &mut self,
        census: &mut PedestrianCensusFrame<'_>,
        center: [f32; 4],
        inner: f32,
        outer: f32,
        initial: bool,
        numeric: &mut N,
    ) -> Result<(), String>;
}

pub(crate) struct PedestrianCensusFrame<'a> {
    pub(crate) census: &'a CensusState,
    pub(crate) random: &'a mut NativeSkaterRandom,
    pub(crate) recipe_man: &'a RecipeMan,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) painter: &'a WorldPainterLayerMan,
    pub(crate) override_model_name: bool,
}

impl PedestrianCensusFrame<'_> {
    /// SpawnPedestrian82720fe0: security and zone gates precede world enable,
    /// then recipe selection/model residency precede the force/reserve count gate.
    pub(crate) fn spawn<N: ReckoningNumeric>(
        &mut self,
        services: &mut impl PedestrianCensusServices<N>,
        entity_key: u64,
        transform: Transform,
        initial: bool,
        allow_culling: bool,
        force_spawn: bool,
        allow_world_collisions: bool,
        ignore_navmesh_ground: bool,
        allow_hand_prop: bool,
        numeric: &mut N,
    ) -> Result<Option<u32>, String>
    where
        N::Error: Debug,
    {
        let key = skate2_formats::attrib::string_hash64;
        let nsz = entity_key == key("security_nsz01") || entity_key == key("security_nsz02");
        if !self.cvars.try_enable_spawn_security().map_err(error)?
            && (nsz || entity_key == key("security_street01"))
        {
            return Ok(None);
        }
        if nsz
            && self
                .painter
                .try_region_name(6, [transform[3][0], transform[3][2]])
                .map_err(error)?
                .is_none()
        {
            return Ok(None);
        }
        if self.census.enable & 0x80 == 0 {
            return Ok(None);
        }
        let entity = self.vault.instance(ENTITIES_CLASS, entity_key);
        if entity.is_null() {
            return Ok(None);
        }
        let parent = entity
            .try_cached_layout_field(8, 8)
            .map_err(error)?
            .map_or(0, |data| {
                u64::from_be_bytes(data.bytes().try_into().expect("checked model key"))
            });
        let attributes = RecipeAttributes::new(self.vault, self.cvars);
        let model_key = self
            .recipe_man
            .select_model(parent, &attributes)
            .map_err(error)?;
        if self.override_model_name {
            return Err(error("Juice model override owner absent"));
        }
        let model = self.vault.instance(LIVING_WORLD_MODELS_CLASS_ID, model_key);
        if model.is_null() {
            return Ok(None);
        }
        let load_model = model
            .try_get_data(LOAD_MODEL_ATTRIBUTE_ID, 0)
            .map_err(error)?
            .is_some_and(|data| data.bytes().first().is_some_and(|&v| v != 0));
        let recipe_id = self
            .recipe_man
            .model_asset_id(model_key, &attributes)
            .map_err(error)?;
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static CANDIDATES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some()) {
            let n=CANDIDATES.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
            if n < 8 || n % 120 == 0 {
                std::eprintln!("pedestrian candidate={n} entity={entity_key:016x} model={model_key:016x} recipe={recipe_id:016x} load={load_model} resident={} count={} initial={initial}",
                    services.contains_model(recipe_id),services.count());
            }
        }
        if load_model && (recipe_id == NULL_ASSET_ID || !services.contains_model(recipe_id)) {
            return Ok(None);
        }
        let allowance = if initial {
            self.cvars
                .try_initial_spawn_pedestrian_count()
                .map_err(error)?
        } else {
            self.census
                .max_pedestrians
                .ok_or_else(|| error("frame-saver pedestrian allowance absent"))?
        };
        let limit = if force_spawn {
            self.cvars.try_max_pedestrians().map_err(error)?
        } else {
            allowance.wrapping_sub(self.census.pedestrian_reserve)
        };
        if limit <= 0 || limit <= services.count() as i32 {
            return Ok(None);
        }
        services.request_spawn(
            PedestrianSpawnInfo {
                entity_key,
                model_key,
                recipe_id,
                transform,
                initial,
                allow_culling,
                force_spawn,
                allow_world_collisions,
                load_model,
                ignore_navmesh_ground,
                allow_hand_prop,
            },
            numeric,
        )
    }

    /// EvaluativeSpawnPedestrians8271ea90: captured population count, original
    /// Manager random placement, extra yaw draw, then weighted census selection.
    pub(crate) fn evaluative_spawn<N: ReckoningNumeric>(
        &mut self,
        services: &mut impl PedestrianCensusServices<N>,
        center: [f32; 4],
        inner: f32,
        outer: f32,
        initial: bool,
        ghost: bool,
        network: bool,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        let captured_count = services.count() as u32;
        let enabled = self
            .vault
            .instance(
                crate::living_world_rain::DEBUG_CLASS,
                crate::living_world_rain::DEBUG_COLLECTION,
            )
            .try_cached_layout_field(121, 1)
            .map_err(error)?
            .is_some_and(|data| data.bytes()[0] != 0);
        if !enabled || ghost || network {
            return Ok(());
        }
        let (attempts, mut remaining, inner, outer) = if initial {
            (6000, 600, 8., 80.)
        } else {
            (2, 1, inner, outer)
        };
        for _ in 0..attempts {
            let position = random_position(
                self.random,
                &NativeRandomGate::production_shared(),
                center,
                inner,
                outer,
                numeric,
            )
            .map_err(error)?;
            let yaw = self
                .random
                .try_living_world_range(
                    &NativeRandomGate::production_shared(),
                    0.,
                    f32::from_bits(0x40c90fdb),
                )
                .map_err(error)?;
            let (sin, cos) = try_sin_cos(yaw, &mut HostReckoningSinCosNumeric).map_err(error)?;
            let transform = [
                [cos, 0., -sin, 0.],
                [0., 1., 0., 0.],
                [sin, 0., cos, 0.],
                position,
            ];
            let target = self
                .census
                .target(
                    PopulationKind::Pedestrian,
                    [position[0], position[1], position[2]],
                    captured_count,
                    self.vault,
                    self.painter,
                    self.cvars,
                    &mut LiveCensusRandom(self.random),
                )
                .map_err(error)?;
            let Some(target) = target else {
                continue;
            };
            if self
                .spawn(
                    services, target, transform, initial, true, false, true, false, true, numeric,
                )?
                .is_some()
            {
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        }
        Ok(())
    }

    /// EvaluativeCullPedestrians8271ef70. Includes altitude, preserves boundary
    /// ties, and signals guards outside radius instead of culling protected ones.
    pub(crate) fn evaluative_cull<N: ReckoningNumeric>(
        &mut self,
        services: &mut impl PedestrianCensusServices<N>,
        center: [f32; 4],
        radius: f32,
        outside: bool,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        for entry in services.cull_entries() {
            let delta = std::array::from_fn(|i| center[i] - entry.position[i]);
            let distance = numeric.dot3(delta, delta).map_err(error)? - radius * radius;
            if entry.allow_culling {
                if if outside {
                    !(distance < 0.)
                } else {
                    !(distance > 0.)
                } {
                    services.request_cull(entry.id,self.random)?;
                }
            } else if entry.security_guard {
                services.set_security_outside_cull_radius(entry.id, distance >= 0.)?;
            }
        }
        Ok(())
    }

    /// EvaluatePedestrianCensus82723d90, existing census-circle interpolation.
    pub(crate) fn evaluate<N: ReckoningNumeric>(
        &mut self,
        services: &mut impl PedestrianCensusServices<N>,
        inputs: &super::super::dmo_objects::DmoCensusInputs,
        do_cull: bool,
        do_spawn: bool,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        let key = self
            .census
            .pedestrian_range_key
            .ok_or_else(|| error("pedestrian census range missing"))?;
        let ranges = self.vault.instance(CENSUS_RANGES_CLASS, key);
        let data = ranges
            .try_cached_layout_field(0, 0x28)
            .map_err(error)?
            .ok_or_else(|| error("pedestrian census range layout missing"))?;
        let scalar = |offset: usize| {
            f32::from_be_bytes(
                data.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("checked census layout"),
            )
        };
        let circle = |base| CensusCircle {
            no_spawn: scalar(base),
            spawn: scalar(base + 4),
            cull: scalar(base + 8),
            shift: scalar(base + 12),
            speed: scalar(base + 16),
        };
        let speed = crate::reckoning_numeric::try_length(inputs.velocity, numeric)
            .map_err(error)?
            * f32::from_bits(0x40666666);
        let circle = evaluate_census_circle(speed, circle(0), circle(0x14));
        let direction =
            crate::reckoning_numeric::try_normalize_safe(inputs.velocity, [0.; 4], numeric)
                .map_err(error)?;
        let center = std::array::from_fn(|i| direction[i].mul_add(circle.shift, inputs.center[i]));
        let banned = self.census.pedestrians_banned;
        if do_cull {
            let radius = if banned { 0. } else { circle.cull };
            self.evaluative_cull(services, center, radius, true, numeric)?;
        }
        if do_spawn
            && !banned
            && self.cvars.try_enable_spawn_pedestrians().map_err(error)?
            && self.census.enable & 0x20 != 0
        {
            if self.census.enable & 0x10 != 0 {
                services.prop_driven_spawn(
                    self,
                    center,
                    circle.no_spawn,
                    circle.spawn,
                    inputs.initial,
                    numeric,
                )?;
            }
            self.evaluative_spawn(
                services,
                center,
                circle.no_spawn,
                circle.spawn,
                inputs.initial,
                inputs.ghost,
                inputs.network_game,
                numeric,
            )?;
        }
        Ok(())
    }
}
