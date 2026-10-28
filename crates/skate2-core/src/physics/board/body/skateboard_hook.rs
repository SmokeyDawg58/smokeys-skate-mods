//! Retained payload and definition prefix of `CreateHookDrive`, 0x82b2e4b8.
//! No `HookPool` admission, cross-assembly Drive, spawn transform, allocator
//! singleton, Vault fetch or complete `SkateboardBody` constructor is implied.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "hook prefix awaits live skater publication")
)]

use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyBackendTemplate, DynamicAssemblyPartAttachmentMetadata,
        DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload,
        DynamicAssemblyRuntimeTemplate, DynamicAssemblyTemplateError,
    },
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    simulation_rigid_body_pool::SimulationInertia,
    simulation_volume_integral::fat_box_unit_mass,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_truck_drives::SkateboardTruckDriveNumeric,
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, SkateboardWheelMassNumeric, apply_unit_density_mass_to_inertia,
    },
    world::try_simulation_quaternion_candidate,
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[cfg(test)]
#[path = "skateboard_hook_tests.rs"]
mod tests;

const HALF_EXTENT: f32 = f32::from_bits(0x3dcc_cccd);
const MASS: f32 = f32::from_bits(0x42c8_0000);
const ZERO_PARAMS: SimulationDriveDynamicsParams = SimulationDriveDynamicsParams {
    spring: 0.0,
    damping: 0.0,
    strength: 0.0,
    raw_type: 0,
};

pub(crate) type SkateboardHookRuntimeTemplate =
    DynamicAssemblyBackendTemplate<SkateboardSimulationBackend, SkateboardHookDefinition>;

/// One source-created Box and its `PartDefinition` data. These values remain
/// owned even though later Static admission does not retain live body inertia.
#[derive(Debug)]
pub(crate) struct SkateboardHookDefinition {
    volume: Volume,
    inertia: Arc<SimulationInertia>,
    material: AssemblyPartMaterial,
    inverse_body_transform: Option<[f32; 16]>,
    is_static_body: bool,
}
impl SkateboardHookDefinition {
    pub(crate) const fn volume(&self) -> &Volume {
        &self.volume
    }
    pub(crate) fn inertia(&self) -> &Arc<SimulationInertia> {
        &self.inertia
    }
    pub(crate) fn inverse_tensor_w(&self) -> f32 {
        self.inertia.inverse_tensor[0]
    }
    pub(crate) const fn material(&self) -> &AssemblyPartMaterial {
        &self.material
    }
    pub(crate) const fn inverse_body_transform(&self) -> Option<&[f32; 16]> {
        self.inverse_body_transform.as_ref()
    }
    /// `PartDefinition`'s word stays zero; `STATIC_BODY1` is a later admission input.
    pub(crate) const fn is_static_body(&self) -> bool {
        self.is_static_body
    }
}

/// Except Complete, stages identify the currently attempted operation. A host
/// error leaves all earlier stores intact and never restores Fresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardHookStage {
    Fresh,
    Positions,
    ChildFrame,
    ParentFrame,
    Dynamics,
    Definition,
    Complete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardHookPayload {
    Frames,
    Dynamics,
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardHookError<E> {
    NotFresh { stage: SkateboardHookStage },
    Numeric(E),
    Poisoned { payload: SkateboardHookPayload },
    Template(DynamicAssemblyTemplateError),
}
impl<E: fmt::Display> fmt::Display for SkateboardHookError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFresh { stage } => {
                write!(formatter, "hook preparation already attempted: {stage:?}")
            }
            Self::Numeric(error) => write!(formatter, "hook quaternion numeric: {error}"),
            Self::Poisoned { payload } => write!(formatter, "hook {payload:?} lock poisoned"),
            Self::Template(error) => write!(formatter, "hook template: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SkateboardHookError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Numeric(error) => Some(error),
            Self::Template(error) => Some(error),
            Self::NotFresh { .. } | Self::Poisoned { .. } => None,
        }
    }
}

/// Mutable payload owners preexist callbacks, so observers and subsequent pool
/// admission retain these exact Arcs. `RwLocks` are host lifetime/synchronization
/// policy, not native fields. Preparation requires exclusive logical mutation:
/// numeric callbacks may observe snapshots but must not mutate/reenter owners.
/// Callbacks run without a payload lock; each following write checks poisoning
/// at that write site. No rollback, poisoned-lock recovery or implicit cleanup.
/// Arc/Vec allocation follows Rust policy, not the native resource allocator.
/// The immutable definition is materialized after mass calculation; this does
/// not model intermediate native allocator publication or inspectable raw
/// `PartDefinition` storage during that calculation. Numeric errors are confined
/// to the two quaternion calls; the reused mass estimate service is infallible.
pub(crate) struct SkateboardHookConstruction {
    frames: Arc<RwLock<SimulationDriveFrames>>,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
    // Absence means not yet produced in this host owner, NOT an assertion that
    // the native uninitialized definition-pointer storage contained null.
    definition: Option<Arc<SkateboardHookDefinition>>,
    template: Option<Arc<SkateboardHookRuntimeTemplate>>,
    stage: SkateboardHookStage,
}
impl SkateboardHookConstruction {
    /// Native retained frames/dynamics initialization, 0x82b38308..0x82b38398.
    pub(crate) fn new() -> Self {
        Self {
            frames: Arc::new(RwLock::new(SimulationDriveFrames {
                quaternion_a: [0.0, 0.0, 0.0, 1.0],
                position_a: [0.0; 4],
                quaternion_b: [0.0, 0.0, 0.0, 1.0],
                position_b: [0.0; 4],
            })),
            dynamics: Arc::new(RwLock::new(SimulationDriveDynamics {
                linear: ZERO_PARAMS,
                angular: ZERO_PARAMS,
            })),
            definition: None,
            template: None,
            stage: SkateboardHookStage::Fresh,
        }
    }
    pub(crate) fn frames(&self) -> &Arc<RwLock<SimulationDriveFrames>> {
        &self.frames
    }
    pub(crate) fn dynamics(&self) -> &Arc<RwLock<SimulationDriveDynamics>> {
        &self.dynamics
    }
    pub(crate) const fn definition(&self) -> Option<&Arc<SkateboardHookDefinition>> {
        self.definition.as_ref()
    }
    pub(crate) const fn template(&self) -> Option<&Arc<SkateboardHookRuntimeTemplate>> {
        self.template.as_ref()
    }
    pub(crate) const fn stage(&self) -> SkateboardHookStage {
        self.stage
    }

    /// `CreateHookDrive` 0x82b2e504..0x82b2e644, ending at the retained typed
    /// one-Part definition/template boundary. Both setters call the existing
    /// unnormalized candidate graph; the direct Box mass path never goes
    /// through aggregate integration/diagonalization or enabled-state culling.
    /// Host providers retain their existing estimate/libm policy; these calls
    /// do not establish Xenon estimate bits or floating-point status parity.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered constructor prefix with explicit retained failure stages"
    )]
    pub(crate) fn try_prepare<N: SkateboardTruckDriveNumeric, M: SkateboardWheelMassNumeric>(
        &mut self,
        quaternion: &mut N,
        mass: &mut M,
    ) -> Result<(), SkateboardHookError<N::Error>> {
        use SkateboardHookError::{NotFresh, Numeric, Poisoned, Template};
        use SkateboardHookStage as Stage;
        if self.stage != Stage::Fresh {
            return Err(NotFresh { stage: self.stage });
        }
        self.stage = Stage::Positions;
        {
            let mut frames = self.frames.write().map_err(|_| Poisoned {
                payload: SkateboardHookPayload::Frames,
            })?;
            frames.position_a = [0.0; 4];
            frames.position_b = [0.0; 4];
        }
        self.stage = Stage::ChildFrame;
        let child = try_simulation_quaternion_candidate(FRESH_VOLUME_TRANSFORM, |input| {
            quaternion.quaternion_reciprocal_square_root_estimate(input)
        })
        .map_err(Numeric)?;
        self.frames
            .write()
            .map_err(|_| Poisoned {
                payload: SkateboardHookPayload::Frames,
            })?
            .quaternion_a = child;
        self.stage = Stage::ParentFrame;
        let parent = try_simulation_quaternion_candidate(FRESH_VOLUME_TRANSFORM, |input| {
            quaternion.quaternion_reciprocal_square_root_estimate(input)
        })
        .map_err(Numeric)?;
        self.frames
            .write()
            .map_err(|_| Poisoned {
                payload: SkateboardHookPayload::Frames,
            })?
            .quaternion_b = parent;
        self.stage = Stage::Dynamics;
        {
            let mut dynamics = self.dynamics.write().map_err(|_| Poisoned {
                payload: SkateboardHookPayload::Dynamics,
            })?;
            dynamics.angular.raw_type = 2;
            dynamics.angular.spring = 0.0;
            dynamics.angular.damping = 0.0;
            dynamics.angular.strength = 0.0;
            dynamics.linear.raw_type = 2;
            dynamics.linear.spring = 0.0;
            dynamics.linear.damping = 0.0;
            dynamics.linear.strength = 0.0;
        }
        self.stage = Stage::Definition;
        let mut volume = Volume {
            transform: FRESH_VOLUME_TRANSFORM,
            vtable_offset: VolumeType::Box as u32,
            shape_words: [HALF_EXTENT.to_bits(); 3],
            radius: 0.0,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        };
        volume.flags &= !1;
        let (diagonal, amount) =
            fat_box_unit_mass(volume.shape_words.map(f32::from_bits), volume.radius);
        // Fresh COM dot is +0 and all identity-axis deviations are zero. The
        // original !LT arm includes unordered; fixed box constants yield LT.
        let threshold = ((diagonal[1] + diagonal[2]) + diagonal[0]) * f32::from_bits(0x3586_37be);
        let inverse_body_transform = if 0.0 < threshold {
            None
        } else {
            Some(FRESH_VOLUME_TRANSFORM)
        };
        let mut inertia = SimulationInertia::native_default();
        apply_unit_density_mass_to_inertia(diagonal, amount, MASS, &mut inertia, mass);
        let definition = Arc::new(SkateboardHookDefinition {
            volume,
            inertia: Arc::new(inertia),
            inverse_body_transform,
            is_static_body: false,
            material: AssemblyPartMaterial {
                static_friction: f32::from_bits(0x3f00_0000),
                dynamic_friction: f32::from_bits(0x3e99_999a),
                restitution: f32::from_bits(0x3ecc_cccd),
            },
        });
        self.definition = Some(Arc::clone(&definition));
        let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(
            vec![DynamicAssemblyPartAttachmentMetadata {
                // Absent native matrix is uninitialized; canonical host storage
                // below is inert because inverse-present remains zero.
                inverse_body_transform: definition
                    .inverse_body_transform
                    .unwrap_or(FRESH_VOLUME_TRANSFORM),
                has_inverse_body_transform: u32::from(definition.inverse_body_transform.is_some()),
                is_static_body: u32::from(definition.is_static_body()),
            }],
            vec![DynamicAssemblyPartPayload {
                inertia: Arc::clone(&definition.inertia).into(),
                material: definition.material,
            }],
        )
        .expect("one source hook Part metadata record and payload");
        self.template = Some(Arc::new(
            DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
                definition,
                Arc::new(attachment),
                Vec::new(),
                Vec::new(),
            )
            .map_err(Template)?,
        ));
        self.stage = Stage::Complete;
        Ok(())
    }
}
