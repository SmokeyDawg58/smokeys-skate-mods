//! Source LWPedestrianModel/LWVehicleModel data owned by the existing ModelMan.
//! Definition volumes reuse the host Volume constructor and BodyMassState. No
//! runtime body/entity is fabricated by model admission.
use super::{dmo_models::NULL_ASSET_ID, dmo_objects::bbox_volume_for_dimensions as bbox_volume};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,
    player::mounted_recipe::finalize::{FinalizedRecipeScene, RecipeFinalizeError},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_body_definition::fresh_volume,
    skeleton_body_mass::BodyMassState,
    world::CollisionArena,
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, Volume, VolumeType};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[path = "living_world_character_collision_assembly.rs"]
pub(crate) mod collision_assembly;
#[path = "living_world_character_collision_mass.rs"]
pub(crate) mod collision_mass;

#[derive(Debug)]
pub(crate) enum CharacterModelError {
    Attribute(AttributeRuntimeError),
    Finalize(RecipeFinalizeError),
    MissingGeometry(u64),
    MissingCollisionModel(usize),
    MissingVolume(usize),
    AssemblyMesh(u32),
    MissingRenderModel(usize),
    BoneMatrix(usize),
    Capacity,
    Culling(super::dmo_models::DmoModelError),
}
impl From<AttributeRuntimeError> for CharacterModelError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Attribute(value)
    }
}
impl std::fmt::Display for CharacterModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld model: {self:?}")
    }
}

pub(crate) const DIMENSIONS: u64 = 0xf983_f251_8b33_5286;
const MAIN_CAPSULE: u64 = 0xb470_a934_64f8_4774;
const ARM_CAPSULE: u64 = 0xe8de_c586_45cb_5eaf;
const WORLD_CAPSULE: u64 = 0xc972_76e3_3f3e_aaf3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CapsuleParams {
    pub(crate) radius: f32,
    pub(crate) length: f32,
    pub(crate) mass: f32,
}
impl CapsuleParams {
    fn read(instance: &AttributeRuntimeInstance, key: u64) -> Result<Self, AttributeRuntimeError> {
        let [radius, length, mass] = node_floats::<3>(instance, key)?;
        Ok(Self {
            radius,
            length,
            mass,
        })
    }
    fn volume(self) -> Volume {
        fresh_volume(
            VolumeType::Capsule,
            self.radius,
            [(self.length * 0.5).to_bits(), 0, 0],
        )
    }
}
#[derive(Debug)]
pub(crate) struct ModelCollisionPart {
    pub(crate) geometry: ModelCollisionGeometry,
    pub(crate) mass: BodyMassState,
    pub(crate) material: AssemblyPartMaterial,
    pub(crate) is_static_body: bool,
}
#[derive(Debug)]
pub(crate) enum ModelCollisionGeometry {
    Primitive(Arc<Volume>),
    /// Native SimpleMappedArray's two children, in order. The first is the
    /// source volume with its original aggregate backing; the second is the
    /// constructed box. This preserves nested volume ownership without making
    /// up an arena object ID for a newly allocated host aggregate.
    VehicleMappedArray {
        source: Arc<CollisionArena>,
        root: usize,
        extra_box: Volume,
    },
}
impl ModelCollisionPart {
    /// PhysicsUtility::CreateAssemblyDefinition82e4a4d8 calls native
    /// PartDefinition::Initialize82ae9058. CapsuleMass is applied by the entity,
    /// not this initializer; do not silently compute mass at model-load time.
    fn new(volume: Volume) -> Self {
        Self {
            geometry: ModelCollisionGeometry::Primitive(Arc::new(volume)),
            mass: BodyMassState::native_default(),
            material: AssemblyPartMaterial {
                static_friction: 0.5,
                dynamic_friction: 0.3,
                restitution: 0.4,
            },
            is_static_body: false,
        }
    }
}
#[derive(Debug)]
pub(crate) struct ModelCollisionDefinition {
    pub(crate) parts: Vec<ModelCollisionPart>,
    pub(crate) culling_table: Arc<BitTable>,
    pub(crate) from_pipe: bool,
    binding: Option<Arc<collision_assembly::ModelPartBinding>>,
}
pub(crate) type SharedCollisionDefinition = Arc<Mutex<ModelCollisionDefinition>>;
impl ModelCollisionDefinition {
    fn shared(
        parts: Vec<ModelCollisionPart>,
        culling_table: Arc<BitTable>,
        from_pipe: bool,
    ) -> SharedCollisionDefinition {
        Arc::new(Mutex::new(Self {
            parts,
            culling_table,
            from_pipe,
            binding: None,
        }))
    }
}
#[derive(Debug)]
pub(crate) struct CharacterModel {
    pub(crate) asset_id: u64,
    pub(crate) dimensions: [f32; 4],
    pub(crate) bbox_min: [f32; 4],
    pub(crate) bbox_max: [f32; 4],
    pub(crate) bbox_volume: Arc<Volume>,
    pub(crate) world_collision: Option<SharedCollisionDefinition>,
    pub(crate) skater_collision: SharedCollisionDefinition,
    pub(crate) kind: CharacterModelKind,
}
#[derive(Debug)]
pub(crate) enum CharacterModelKind {
    Pedestrian {
        main: CapsuleParams,
        arms: CapsuleParams,
        world: CapsuleParams,
    },
    Vehicle {
        wheel_radius: f32,
    },
}
impl CharacterModel {
    /// `LWModel::GetBBoxTransform` 0x82e4cd60 of an entity transform: rows 0..2
    /// copied; the bbox centre is transformed as `fma(z, c.z, fma(y, c.y, fma(x, c.x, w)))`.
    pub(crate) fn bbox_transform(&self, transform: &[f32; 16]) -> [f32; 16] {
        let mut result = *transform;
        let centre: [f32; 4] =
            std::array::from_fn(|lane| (self.bbox_min[lane] + self.bbox_max[lane]) * 0.5);
        for lane in 0..4 {
            let x = transform[lane].mul_add(centre[0], transform[12 + lane]);
            let y = transform[4 + lane].mul_add(centre[1], x);
            result[12 + lane] = transform[8 + lane].mul_add(centre[2], y);
        }
        result
    }
    /// CreateFromMesh82d1efd0 takes the first collidable instance in native
    /// map order, its RModel bounds, and LeftFront_wheel's |IBP.translation.Y|.
    pub(crate) fn create_vehicle(
        vault: &Arc<AttributeRuntimeVault>,
        key: u64,
        asset_id: u64,
        scene: &FinalizedRecipeScene,
        geometry: &BTreeMap<u64, Arc<CollisionArena>>,
        table: Arc<BitTable>,
    ) -> Result<Self, CharacterModelError> {
        if let Some((arena, root, render)) = vehicle_mesh(scene, geometry)? {
            let bbox_min = render.bounding_box.minimum;
            let bbox_max = render.bounding_box.maximum;
            let dimensions = std::array::from_fn(|i| bbox_max[i] - bbox_min[i]);
            let wheel_radius = match render
                .bone_names
                .iter()
                .take(usize::from(render.named_bone_count))
                .position(|name| name == "LeftFront_wheel")
            {
                Some(i) => render
                    .inverse_bind_pose
                    .get(i)
                    .ok_or(CharacterModelError::BoneMatrix(i))?[13]
                    .abs(),
                None => 0.0,
            };
            let mut extra_box = bbox_volume(dimensions);
            // gJ * half-height (all four lanes); the pipe volume is not shifted.
            let half_height = dimensions[1] * 0.5;
            for (lane, axis) in [0.0_f32, 1.0, 0.0, 0.0].into_iter().enumerate() {
                extra_box.transform[12 + lane] = axis * half_height;
            }
            let mut part = ModelCollisionPart::new(extra_box.clone());
            part.geometry = ModelCollisionGeometry::VehicleMappedArray {
                source: Arc::clone(arena),
                root,
                extra_box,
            };
            return Ok(Self {
                asset_id,
                dimensions,
                bbox_min,
                bbox_max,
                bbox_volume: Arc::new(bbox_volume(dimensions)),
                world_collision: None,
                skater_collision: ModelCollisionDefinition::shared(vec![part], table, true),
                kind: CharacterModelKind::Vehicle { wheel_radius },
            });
        }
        // Source fallback82d23b18, then LWModel.Create. Both definitions are
        // independently constructed boxes; native does not shift either here.
        let instance = vault.instance(LIVING_WORLD_MODELS_CLASS_ID, key);
        let _initial_dimensions = node_floats::<4>(&instance, DIMENSIONS)?;
        let [wheel_radius] = node_floats::<1>(&instance, 0xfd7a_6614_2f16_b9cc)?;
        let dimensions = dimensions(&instance)?;
        let (bbox_min, bbox_max) = dimension_bounds(dimensions);
        let definition = || {
            ModelCollisionDefinition::shared(
                vec![ModelCollisionPart::new(bbox_volume(dimensions))],
                Arc::clone(&table),
                false,
            )
        };
        Ok(Self {
            asset_id: NULL_ASSET_ID,
            dimensions,
            bbox_min,
            bbox_max,
            bbox_volume: Arc::new(bbox_volume(dimensions)),
            world_collision: Some(definition()),
            skater_collision: definition(),
            kind: CharacterModelKind::Vehicle { wheel_radius },
        })
    }
    /// Pedestrian.Create82e73890 -> LWModel.Create82e51010. Raw VMX preserves
    /// replicated X in min/max.W and arm axes' padding lanes.
    pub(crate) fn create_pedestrian(
        vault: &Arc<AttributeRuntimeVault>,
        key: u64,
        table: Arc<BitTable>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(LIVING_WORLD_MODELS_CLASS_ID, key);
        let main = CapsuleParams::read(&instance, MAIN_CAPSULE)?;
        let arms = CapsuleParams::read(&instance, ARM_CAPSULE)?;
        let world = CapsuleParams::read(&instance, WORLD_CAPSULE)?;
        let dimensions = dimensions(&instance)?;
        let (bbox_min, bbox_max) = dimension_bounds(dimensions);
        // World capsule remains the constructor's Z-axis volume here.
        let world_collision = ModelCollisionDefinition::shared(
            vec![ModelCollisionPart::new(world.volume())],
            Arc::clone(&table),
            false,
        );
        let arm = || {
            let mut volume = arms.volume();
            let (s, c) = try_sin_cos(f32::from_bits(0x3fc9_0fdb), &mut HostReckoningSinCosNumeric)
                .expect("infallible native polynomial service");
            volume.transform = [
                c, 0.0, -s, c, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, s, 0.0, 0.0, 0.0, 0.0,
            ];
            volume
        };
        let skater_collision = ModelCollisionDefinition::shared(
            vec![
                ModelCollisionPart::new(main.volume()),
                ModelCollisionPart::new(arm()),
                ModelCollisionPart::new(arm()),
            ],
            table,
            false,
        );
        Ok(Self {
            asset_id: NULL_ASSET_ID,
            dimensions,
            bbox_min,
            bbox_max,
            bbox_volume: Arc::new(bbox_volume(dimensions)),
            world_collision: Some(world_collision),
            skater_collision,
            kind: CharacterModelKind::Pedestrian { main, arms, world },
        })
    }
}

fn vehicle_mesh<'a>(
    scene: &'a FinalizedRecipeScene,
    geometry: &'a BTreeMap<u64, Arc<CollisionArena>>,
) -> Result<
    Option<(
        &'a Arc<CollisionArena>,
        usize,
        &'a crate::presentation::PresentationModel,
    )>,
    CharacterModelError,
> {
    let Some((lod, instance, arena, collision)) = first_collision_instance(scene, geometry)? else {
        return Ok(None);
    };
    let Some(render) = instance.render_model else {
        return Ok(None);
    };
    let render = lod
        .arena
        .models
        .get(render)
        .ok_or(CharacterModelError::MissingRenderModel(render))?;
    if let Some(assembly) = collision.assembly_object_id {
        return Err(CharacterModelError::AssemblyMesh(assembly));
    }
    let root = collision.collision_roots[0];
    if root >= arena.aggregate_roots.len() {
        return Err(CharacterModelError::MissingVolume(root));
    }
    Ok(Some((arena, root, render)))
}
/// GetTInstanceWithCollisionVolume82d10f10: same ordered recipe search for
/// model creation and InitSplines. Count!=0 includes the -1 Assembly form;
/// callers, rather than this selector, own unsupported-Assembly handling.
pub(crate) fn first_collision_instance<'a>(
    scene: &'a FinalizedRecipeScene,
    geometry: &'a BTreeMap<u64, Arc<CollisionArena>>,
) -> Result<
    Option<(
        &'a crate::character::CharacterLodAsset,
        &'a crate::presentation::PresentationInstance,
        &'a Arc<CollisionArena>,
        &'a crate::world::CollisionModel,
    )>,
    CharacterModelError,
> {
    for (lod, instance) in scene
        .ordered_instances()
        .map_err(CharacterModelError::Finalize)?
    {
        let arena = geometry
            .get(&lod.loaded_asset_id)
            .ok_or(CharacterModelError::MissingGeometry(lod.loaded_asset_id))?;
        let Some(cinstance) = arena.instances.iter().find(|c| c.guid == instance.guid) else {
            continue;
        };
        let Some(model) = cinstance.collision_model else {
            continue;
        };
        let collision = arena
            .collision_models
            .get(model)
            .ok_or(CharacterModelError::MissingCollisionModel(model))?;
        if collision.collision_roots.is_empty() && collision.assembly_object_id.is_none() {
            continue;
        }
        return Ok(Some((lod, instance, arena, collision)));
    }
    Ok(None)
}

fn dimensions(instance: &AttributeRuntimeInstance) -> Result<[f32; 4], AttributeRuntimeError> {
    if instance.is_null() {
        Ok([1.0, 1.0, 1.0, 0.0])
    } else {
        node_floats::<4>(instance, DIMENSIONS)
    }
}
fn dimension_bounds(d: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let h = d.map(|v| v * 0.5);
    ([-h[0], 0.0, -h[2], -h[0]], [h[0], d[1], h[2], h[0]])
}
pub(crate) fn node_floats<const N: usize>(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<[f32; N], AttributeRuntimeError> {
    let Some(data) = instance.try_get_data(key, 0)? else {
        return Ok([0.0; N]);
    };
    if data.bytes().len() != N * 4 {
        return Err(AttributeRuntimeError::PayloadSize {
            key,
            expected: N * 4,
            actual: data.bytes().len(),
        });
    }
    Ok(std::array::from_fn(|i| {
        f32::from_be_bytes(
            data.bytes()[i * 4..i * 4 + 4]
                .try_into()
                .expect("checked field size"),
        )
    }))
}
