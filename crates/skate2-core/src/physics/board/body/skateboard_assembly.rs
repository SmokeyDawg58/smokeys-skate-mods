//! Retained authored board definitions into the existing Assembly transaction.
//!
//! `Assembly::InitializeComponentsFromDefinition` (0x82ae0bf8) binds ordered
//! part payloads and joint records; `Assembly::AddToSimulation` (0x82ae0df0)
//! consumes them. This bridge retains the actual constructed collision owner,
//! not a synthetic serialized mesh. It does not implement `SkateboardBody::Init`,
//! island/SkaterPool publication, collision serialization or post-admission
//! hook drives. Initial transforms must come from their own source owner.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored board assembly is awaiting live skater publication"
    )
)]

use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyBackendTemplate, DynamicAssemblyJointDefinition,
        DynamicAssemblyPartAttachmentMetadata, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblyPartPayload, DynamicAssemblyPostDriveError, DynamicAssemblyRuntimeTemplate,
        DynamicAssemblySession, DynamicAssemblyTemplateError,
    },
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_assembly_payload::SimulationAssemblyInertia,
    simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames},
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_deck::SkateboardDeckDefinition,
    skateboard_joints::SkateboardJointDefinition,
    skateboard_truck_drives::SkateboardTruckDriveAdmission,
    skateboard_trucks::SkateboardTruckDefinitions,
    skateboard_wheels::{FRESH_VOLUME_TRANSFORM, SkateboardWheelDefinitions},
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable};
use std::sync::{Arc, RwLock};

pub(crate) type SkateboardSimulationBackend =
    SimulationAssemblyBackend<AssemblyPartMaterial, SimulationJointFrames, SimulationJointLimits>;
pub(crate) type SkateboardRuntimeTemplate =
    DynamicAssemblyBackendTemplate<SkateboardSimulationBackend, SkateboardCollisionDefinition>;
pub(crate) type SkateboardAssemblySession =
    DynamicAssemblySession<SkateboardSimulationBackend, SkateboardCollisionDefinition>;

#[path = "skateboard_volume_flags.rs"]
pub(crate) mod volume_flags;

/// Short-lived admission borrow; accepted handles stay in the persistent
/// assembly session even after this borrow ends or a later constructor step
/// fails. No body or drive token escapes the existing authority domain.
pub(crate) struct SkateboardAssemblyDriveAdmission<'a> {
    pub(crate) session: &'a mut SkateboardAssemblySession,
    pub(crate) backend: &'a mut SkateboardSimulationBackend,
}

impl SkateboardTruckDriveAdmission for SkateboardAssemblyDriveAdmission<'_> {
    type Error = DynamicAssemblyPostDriveError;

    fn try_add_drive(
        &mut self,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, Self::Error> {
        self.session
            .try_add_post_attachment_drive(self.backend, parent, child, frames, dynamics)
    }
}

/// Exact component owners survive attachment, removal and source-handle drops.
/// Individual wheels/trucks preserve their shared volume allocations; the deck
/// retains its real mapped-array children and source-calculated mass results.
#[derive(Debug)]
pub(crate) struct SkateboardCollisionDefinition {
    pub(crate) wheels: SkateboardWheelDefinitions,
    pub(crate) trucks: SkateboardTruckDefinitions,
    pub(crate) deck: SkateboardDeckDefinition,
    pub(crate) culling: BitTable,
    pub(crate) volume_flags: volume_flags::SkateboardVolumeFlags,
}

impl SkateboardCollisionDefinition {
    pub(crate) fn from_components(
        wheels: SkateboardWheelDefinitions,
        trucks: SkateboardTruckDefinitions,
        deck: SkateboardDeckDefinition,
        culling: BitTable,
    ) -> Self {
        let volume_flags =
            volume_flags::SkateboardVolumeFlags::from_components(&wheels, &trucks, &deck);
        Self {
            wheels,
            trucks,
            deck,
            culling,
            volume_flags,
        }
    }
}

/// Composes source-created component definitions. This is a host ownership
/// bridge, not a second attribute sampling path or a complete native Init.
/// Native definition starts with zero drives; truck/hook drives are created
/// after island admission and are not silently substituted by empty behavior.
pub(crate) fn try_bind_skateboard_assembly(
    collision: Arc<SkateboardCollisionDefinition>,
    joints: Vec<SkateboardJointDefinition>,
) -> Result<Arc<SkateboardRuntimeTemplate>, DynamicAssemblyTemplateError> {
    try_bind_parts(collision, joints, false)
}

/// Runtime Body setters mutate the same Part inertia later read by the solver.
/// Create that owner once, before admission; authored component definitions
/// remain immutable construction evidence rather than live inertia storage.
pub(crate) fn try_bind_skateboard_runtime_assembly(
    collision: Arc<SkateboardCollisionDefinition>,
    joints: Vec<SkateboardJointDefinition>,
) -> Result<Arc<SkateboardRuntimeTemplate>, DynamicAssemblyTemplateError> {
    try_bind_parts(collision, joints, true)
}

fn try_bind_parts(
    collision: Arc<SkateboardCollisionDefinition>,
    joints: Vec<SkateboardJointDefinition>,
    mutable_inertia: bool,
) -> Result<Arc<SkateboardRuntimeTemplate>, DynamicAssemblyTemplateError> {
    let mut metadata = Vec::with_capacity(7);
    let mut payloads = Vec::with_capacity(7);
    for wheel in collision.wheels.parts() {
        append_part(
            &mut metadata,
            &mut payloads,
            wheel.inertia(),
            wheel.material(),
            wheel.inverse_body_transform(),
            wheel.is_static_body(),
            mutable_inertia,
        );
    }
    for truck in collision.trucks.parts() {
        append_part(
            &mut metadata,
            &mut payloads,
            truck.inertia(),
            truck.material(),
            truck.inverse_body_transform(),
            truck.is_static_body(),
            mutable_inertia,
        );
    }
    let deck = &collision.deck;
    append_part(
        &mut metadata,
        &mut payloads,
        deck.inertia(),
        *deck.material(),
        deck.inverse_body_transform(),
        deck.is_static_body(),
        mutable_inertia,
    );
    let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads)
        .expect("one metadata/payload pair is appended for each of seven owned parts");
    DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        collision,
        Arc::new(attachment),
        joints
            .into_iter()
            .map(|joint| DynamicAssemblyJointDefinition {
                parent_part: joint.parent_part,
                child_part: joint.child_part,
                frames: joint.frames,
                // Authored constructor output stays immutable. Move the same Arc
                // into its admission binding; no cloning or mutable replacement.
                limits: joint.limits.into(),
            })
            .collect(),
        Vec::new(),
    )
    .map(Arc::new)
}

fn append_part(
    metadata: &mut Vec<DynamicAssemblyPartAttachmentMetadata>,
    payloads: &mut Vec<DynamicAssemblyPartPayload<SimulationAssemblyInertia, AssemblyPartMaterial>>,
    inertia: &SimulationInertia,
    material: AssemblyPartMaterial,
    inverse_body_transform: Option<&[f32; 16]>,
    is_static_body: bool,
    mutable_inertia: bool,
) {
    metadata.push(DynamicAssemblyPartAttachmentMetadata {
        // Inactive absent-frame storage is canonical host data, never read by
        // Part attachment and not a native initialized-memory claim.
        inverse_body_transform: inverse_body_transform
            .copied()
            .unwrap_or(FRESH_VOLUME_TRANSFORM),
        has_inverse_body_transform: u32::from(inverse_body_transform.is_some()),
        is_static_body: u32::from(is_static_body),
    });
    payloads.push(DynamicAssemblyPartPayload {
        inertia: if mutable_inertia {
            SimulationAssemblyInertia::mutable(
                crate::simulation_rigid_body_pool::SimulationMutableInertia::new(*inertia),
            )
        } else {
            Arc::new(*inertia).into()
        },
        material,
    });
}
