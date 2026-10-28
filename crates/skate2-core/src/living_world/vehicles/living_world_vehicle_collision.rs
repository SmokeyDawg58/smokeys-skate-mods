//! Vehicle collision ownership and the selected native admission/drive paths.
//! InitBodies82d28eb8, AddModels82e5cd40, UpdateTransforms82e5cf28. Owns the
//! constructed candidate until Island admission transfers it to the SAME pool.
//! Factory/CollideableMan registration and behavioral collision recovery remain
//! outer callers; this module does not claim those stages have already run.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        character_models::{
            CharacterModel,
            collision_assembly::{VehicleCollisionCreateError, try_create_vehicle_skater_body},
        },
        collision_drive::collision_part_target,
        entity_core::{CommonEntityCore, CoreError},
    },
    living_world_rain::{LIVING_WORLD_CLASS, geometry::Transform},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_island::vehicle::{
        LivingWorldBodyDriveError, LivingWorldBodyKind, VehicleAdmissionFailure,
        VehicleIslandError, VehiclePlacementError,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::{
        SimulationDmoAssemblyHandle,
        character::{CharacterAssemblyCandidate, CharacterPoolError},
    },
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
};
use std::{convert::Infallible, sync::Arc};

enum Body {
    /// Can retain an attached candidate on a checked pool-insertion failure;
    /// such a failure requires explicit recovery, never implicit Drop cleanup.
    Constructed(CharacterAssemblyCandidate),
    InWorld(SimulationDmoAssemblyHandle),
}

pub(crate) struct VehicleCollision {
    pub(crate) ignore_gravity: bool,
    pub(crate) recovery_attempts: u32,
    pub(crate) required_to_begin_colliding: bool,
    pub(crate) forced_to_recover: bool,
    pub(crate) colliding: bool,
    pub(crate) colliding_with_skater: bool,
    model: Option<Arc<CharacterModel>>,
    skater_body: Option<Body>,
    attributes: AttributeRuntimeInstance,
}

#[derive(Debug)]
pub(crate) enum VehicleCollisionError {
    AlreadyInitialized,
    MissingModel,
    PoisonedDefinition,
    Attribute(AttributeRuntimeError),
    EmptyAttribute(u64),
    Create(VehicleCollisionCreateError<Infallible>),
    Placement(VehiclePlacementError),
    Insert(CharacterPoolError),
    Island(VehicleIslandError),
    Core(CoreError<Infallible>),
    Drive(LivingWorldBodyDriveError<ReckoningHostNumericError, Infallible>),
    Detach(crate::dynamic_assembly_session::DynamicAssemblyBodyOnlyPartDetachError),
}

impl VehicleCollision {
    /// Vehicle ctor82d1a810 -> base82e571e8; attribute collection full64 key
    /// confirmed from insrdi82d1a84c, not the decompiler's low word.
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            ignore_gravity: true,
            recovery_attempts: 0,
            required_to_begin_colliding: false,
            forced_to_recover: false,
            colliding: false,
            colliding_with_skater: false,
            model: None,
            skater_body: None,
            attributes: vault.instance(LIVING_WORLD_CLASS, 0x8be1_d151_f9da_2d29),
        }
    }

    /// Selected fresh initialization suffix: model assignment BEFORE InitBodies,
    /// including the null-model early return. No world body is created for a
    /// vehicle, even when its fallback model carries a world box Definition.
    pub(crate) fn initialize_bodies(
        &mut self,
        model: Option<Arc<CharacterModel>>,
        entity_id: u32,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
    ) -> Result<(), VehicleCollisionError> {
        if self.model.is_some() || self.skater_body.is_some() {
            return Err(VehicleCollisionError::AlreadyInitialized);
        }
        self.model = model;
        let Some(model) = &self.model else {
            return Ok(());
        };
        let assembly = try_create_vehicle_skater_body(
            model,
            world.backend_mut(),
            cvars,
            &mut HostSimulationVolumeIntegralNumeric,
            &mut HostSkeletonBodyMassNumeric,
        )
        .map_err(VehicleCollisionError::Create)?;
        self.skater_body = Some(Body::Constructed(CharacterAssemblyCandidate::new(
            assembly, entity_id,
        )));
        Ok(())
    }

    pub(crate) fn assembly_handle(&self) -> Option<&SimulationDmoAssemblyHandle> {
        self.skater_body.as_ref().map(|body| match body {
            Body::Constructed(candidate) => candidate.handle(),
            Body::InWorld(handle) => handle,
        })
    }

    pub(crate) fn model_dimensions(&self) -> Option<[f32; 4]> {
        self.model.as_ref().map(|model| model.dimensions)
    }

    /// `cLivingWorldCollision::LivingWorldModel`'s `mBBoxMin` and `mBBoxMax`.
    pub(crate) fn model_bounds(&self) -> Option<([f32; 4], [f32; 4])> {
        self.model.as_ref().map(|model| (model.bbox_min, model.bbox_max))
    }

    /// GrabSplineQuery82d23190/GetGrabSpline82d1ee58 require Body.mIsland
    /// BEFORE reading mAssembly. A constructed but unadmitted body is skipped.
    pub(crate) fn attached_assembly_handle(&self) -> Option<&SimulationDmoAssemblyHandle> {
        match &self.skater_body {
            Some(Body::InWorld(handle)) => Some(handle),
            _ => None,
        }
    }

    fn attribute_flag(&self, key: u64) -> Result<bool, VehicleCollisionError> {
        let Some(value) = self
            .attributes
            .try_get_data(key, 0)
            .map_err(VehicleCollisionError::Attribute)?
        else {
            return Ok(false);
        };
        value
            .bytes()
            .first()
            .map(|v| *v != 0)
            .ok_or(VehicleCollisionError::EmptyAttribute(key))
    }
    pub(crate) fn do_world_collisions(&self) -> Result<bool, VehicleCollisionError> {
        self.attribute_flag(0xaf6d_ee43_dcf7_3377) // 82d1a890
    }
    pub(crate) fn do_skater_collisions(&self) -> Result<bool, VehicleCollisionError> {
        self.attribute_flag(0x3bca_c7a1_4c4e_bf5f) // 82d1a8f0
    }

    fn part_target(
        &self,
        core: &CommonEntityCore,
        skater: bool,
    ) -> Result<[f32; 16], VehicleCollisionError> {
        let model = self
            .model
            .as_ref()
            .ok_or(VehicleCollisionError::MissingModel)?;
        let definition = if skater {
            Some(&model.skater_collision)
        } else {
            model.world_collision.as_ref()
        };
        let from_pipe = match definition {
            Some(definition) => {
                definition
                    .lock()
                    .map_err(|_| VehicleCollisionError::PoisonedDefinition)?
                    .from_pipe
            }
            None => false,
        };
        let target: Transform = collision_part_target(
            core.target_transform,
            core.velocity,
            from_pipe,
            model.dimensions[1],
        );
        Ok(std::array::from_fn(|i| target[i / 4][i % 4]))
    }

    /// AddModels82e5cd40: SetTransform first, DoWorld BEFORE allow-world test,
    /// then DoSkater. Fill transforms is outside Body.AddToWorld's existing-
    /// membership early return, so it still runs for an already admitted body.
    pub(crate) fn add_models_to_world(
        &mut self,
        core: &mut CommonEntityCore,
        world: &mut PhysicalAssemblyWorld,
        allow_world: bool,
        _world_state: u32,
        skater_state: u32,
    ) -> Result<(), VehicleCollisionError> {
        core.set_transform(core.target_transform)
            .map_err(VehicleCollisionError::Core)?;
        if self.do_world_collisions()? && allow_world {
            let _world_target = self.part_target(core, false)?;
            // AddWorldCollisionBodyToWorld82e505d8 returns on null vehicle body.
        }
        if self.do_skater_collisions()? {
            let target = self.part_target(core, true)?;
            if matches!(self.skater_body, Some(Body::InWorld(_))) {
                return Ok(());
            }
            let Some(Body::Constructed(candidate)) = self.skater_body.take() else {
                return Ok(());
            };
            match world.try_admit_vehicle_collision(candidate, &[target], skater_state) {
                Ok(handle) => self.skater_body = Some(Body::InWorld(handle)),
                Err(VehicleAdmissionFailure::Placement { error, candidate }) => {
                    self.skater_body = Some(Body::Constructed(candidate));
                    return Err(VehicleCollisionError::Placement(error));
                }
                Err(VehicleAdmissionFailure::Insert(failure)) => {
                    let failure = *failure;
                    self.skater_body = Some(Body::Constructed(failure.candidate));
                    return Err(VehicleCollisionError::Insert(failure.error));
                }
                Err(VehicleAdmissionFailure::Island { error, handle }) => {
                    self.skater_body = Some(Body::InWorld(handle));
                    return Err(VehicleCollisionError::Island(error));
                }
            }
        }
        Ok(())
    }

    /// UpdateTransforms82e5cf28 fills WORLD then SKATER even though the vehicle
    /// has no world body. Actual attached Parts receive the existing source
    /// drive loop; constructed/unadmitted bodies preserve its no-Island return.
    pub(crate) fn update_collision_transforms(
        &self,
        core: &CommonEntityCore,
        world: &mut PhysicalAssemblyWorld,
        dt: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), VehicleCollisionError> {
        let _world_target = self.part_target(core, false)?;
        let target = self.part_target(core, true)?;
        if let Some(Body::InWorld(handle)) = &self.skater_body {
            world
                .try_drive_living_world_collision_body(
                    LivingWorldBodyKind::Vehicle,
                    handle,
                    &[target],
                    dt,
                    self.ignore_gravity,
                    cvars,
                    &mut HostReckoningNumeric,
                    &mut HostReckoningSinCosNumeric,
                )
                .map_err(VehicleCollisionError::Drive)?;
        }
        Ok(())
    }

    /// RemoveFromWorld retains the Body/Assembly for later re-admission. Its
    /// model and assembly identity survive; only Island membership is removed.
    pub(crate) fn remove_models_from_world(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleCollisionError> {
        if let Some(Body::InWorld(handle)) = &self.skater_body {
            let candidate = world
                .try_remove_vehicle_collision(handle)
                .map_err(VehicleCollisionError::Island)?;
            self.skater_body = Some(Body::Constructed(candidate));
        }
        Ok(())
    }

    /// cLivingWorldCollision destructor82e57220: remove bodies from the world,
    /// destroy their Assemblies, then release the model. Safe-host recovery also
    /// detaches a candidate retained after failed pool insertion; Drop alone
    /// cannot remove its backend bodies.
    pub(crate) fn destroy_bodies(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleCollisionError> {
        self.remove_models_from_world(world)?;
        if let Some(Body::Constructed(candidate)) = &mut self.skater_body {
            if candidate.assembly.parts.is_attached() {
                candidate
                    .try_detach(world.backend_mut())
                    .map_err(VehicleCollisionError::Detach)?;
            }
        }
        self.skater_body = None;
        self.model = None;
        Ok(())
    }
}
