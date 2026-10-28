//! Retained pre-admission `SkateboardBody::Init` producer (0x82b374e8).
//!
//! This constructs the real seven-Part definition/session, not a complete
//! Skateboard, physical player or Island registration. Source descriptor/category
//! allocation and temporary debug Instance lifetime are excluded. Existing
//! component constructors retain their checked-host allocation/numeric policies.
//! A real fresh detached Session precedes initial-placement reads; each completed
//! source Part write survives a later failure. Completed component owners and
//! that Session survive later failures here. A failed attempt is never retried.
//!
//! Vault bindings are immutable. Wheel sampling is the accepted immutable
//! constructor snapshot, not mutable native read timing. Host reciprocal/sqrt,
//! trig and dot reductions are not asserted to reproduce Xenon result bits.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "pre-admission board producer awaits paired construction integration"
    )
)]

#[path = "skateboard_body_output_getters.rs"]
pub(crate) mod output_getters;

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    dynamic_assembly_session::{DynamicAssemblySessionCreateError, DynamicAssemblyTemplateError},
    skateboard_assembly::{
        SkateboardAssemblySession, SkateboardCollisionDefinition, SkateboardRuntimeTemplate,
        try_bind_skateboard_runtime_assembly,
    },
    skateboard_attributes::{
        SkateboardAttributeClass as Class, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
    skateboard_deck::{HostSkateboardDeckNumeric, SkateboardDeckDefinition, SkateboardDeckError},
    skateboard_initial_transforms::{
        HostSkateboardInitialTransformNumeric, SkateboardInitialTransformError,
        initialize_skateboard_culling_table, try_initialize_skateboard_transforms_with_writer,
    },
    skateboard_joints::{
        HostSkateboardJointNumeric, SkateboardJointBuildError, SkateboardJointDefinition,
        try_create_skateboard_joints,
    },
    skateboard_trucks::{SkateboardTruckDefinitionError, SkateboardTruckDefinitions},
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, HostSkateboardWheelMassNumeric, SkateboardWheelAttributes,
        SkateboardWheelDefinitionError, SkateboardWheelDefinitions,
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, fmt, sync::Arc};

#[cfg(test)]
#[path = "skateboard_body_construction_tests.rs"]
mod tests;

#[path = "../collision/skateboard_wheel_lines.rs"]
pub(crate) mod wheel_lines;

#[path = "../runtime/skateboard_wheel_compressions.rs"]
pub(crate) mod wheel_compressions;

#[path = "../runtime/skateboard_debug_colors.rs"]
pub(crate) mod debug_colors;

#[path = "../runtime/skateboard_standard_session.rs"]
pub(crate) mod standard;

#[path = "../runtime/skateboard_wipeout_session.rs"]
pub(crate) mod wipeout;

pub(crate) type SkateboardRetainedWheelLines = wheel_lines::SkateboardWheelLines<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch>;

/// Host provenance only: associates successful real Start/End with a new solved
/// contact frame. This does not add native Sync calls or manufacture hits.
#[derive(Default)]
pub(crate) struct SkateboardWheelSchedule {
    started: bool,
    before: Option<Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>>,
    completed:
        Option<Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>>,
}
impl SkateboardWheelSchedule {
    pub(crate) fn begin(
        &mut self,
        frame: Option<
            &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        >,
    ) {
        self.started = false;
        self.before = frame.cloned();
        self.completed = None;
    }
    pub(crate) fn started(&mut self) {
        self.started = true;
    }
    pub(crate) fn end(
        &mut self,
        frame: Option<
            &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        >,
    ) {
        self.completed = frame
            .filter(|frame| {
                self.started
                    && self
                        .before
                        .as_ref()
                        .is_none_or(|old| !Arc::ptr_eq(old, frame))
            })
            .cloned();
        self.started = false;
    }
    pub(crate) fn matches(
        &self,
        frame: &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
    ) -> bool {
        self.completed
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, frame))
    }
}

/// Selected constructor-backed fields consumed by subsequent contact stages.
pub(crate) struct SkateboardBodyPostFields {
    pub(crate) time_since_wheels_touching: f32,
    pub(crate) grip_enabled: bool,
    pub(crate) free_wheel_enabled: bool,
    pub(crate) grip_vector: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) grip_cosine: f32,
    pub(crate) num_parts: u8,
    pub(crate) num_wheels: u8,
}
impl SkateboardBodyPostFields {
    /// Body::TeleportReset 0x82b2e844..e860 clears only the two represented
    /// high flag bits, then stores positive-zero cosine and full-zero vector.
    pub(crate) fn reset_teleport_grip(&mut self) {
        self.grip_enabled = false;
        self.free_wheel_enabled = false;
        self.grip_cosine = 0.0;
        self.grip_vector = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
    }

    fn from_constructor() -> Self {
        Self {
            time_since_wheels_touching: 0.0, //386d8
            grip_enabled: false,             // Body+2098 bit40 clear387c4
            free_wheel_enabled: false,       // bit80 clear387c4 (387bc loads old byte)
            grip_vector: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]), //383a4
            grip_cosine: 0.0, //383a8
            num_parts: 0,     // CollisionInfo Reset236b0
            num_wheels: 0,    //236bc
        }
    }
    /// Final Body `PostPhysics` timer store374b8, before either truck call.
    pub(crate) fn advance_wheel_contact_timer(&mut self, timestep: f32) {
        self.time_since_wheels_touching = if self.num_wheels == 0 {
            self.time_since_wheels_touching + timestep
        } else {
            0.0
        };
    }
}

/// These are Body-owned records, NOT replacement Part material bindings.
/// Init does not select them; later grind/standard/wipeout setters do that.
#[derive(Debug, PartialEq)]
pub(crate) struct SkateboardBodyMaterials {
    wheel: Arc<AssemblyPartMaterial>,
    deck: Arc<AssemblyPartMaterial>,
    truck: Arc<AssemblyPartMaterial>,
    wheel_grind: Arc<AssemblyPartMaterial>,
    deck_grind: Arc<AssemblyPartMaterial>,
    truck_grind: Arc<AssemblyPartMaterial>,
    wipeout: Arc<AssemblyPartMaterial>,
}

impl SkateboardBodyMaterials {
    /// Seven ctor triples, 0x82b38748..0x82b387a0. These are not Vault defaults.
    fn new() -> Self {
        let initial = AssemblyPartMaterial {
            static_friction: f32::from_bits(0x3f00_0000),
            dynamic_friction: f32::from_bits(0x3e99_999a),
            restitution: f32::from_bits(0x3ecc_cccd),
        };
        Self {
            wheel: Arc::new(initial),
            deck: Arc::new(initial),
            truck: Arc::new(initial),
            wheel_grind: Arc::new(initial),
            deck_grind: Arc::new(initial),
            truck_grind: Arc::new(initial),
            wipeout: Arc::new(initial),
        }
    }

    // Value observations do not expose Arc/Weak ownership before construction.
    pub(crate) fn wheel(&self) -> &AssemblyPartMaterial {
        &self.wheel
    }
    pub(crate) fn deck(&self) -> &AssemblyPartMaterial {
        &self.deck
    }
    pub(crate) fn truck(&self) -> &AssemblyPartMaterial {
        &self.truck
    }
    pub(crate) fn wheel_grind(&self) -> &AssemblyPartMaterial {
        &self.wheel_grind
    }
    pub(crate) fn deck_grind(&self) -> &AssemblyPartMaterial {
        &self.deck_grind
    }
    pub(crate) fn truck_grind(&self) -> &AssemblyPartMaterial {
        &self.truck_grind
    }
    pub(crate) fn wipeout(&self) -> &AssemblyPartMaterial {
        &self.wipeout
    }
}

/// The current operation (also retained on failure), not a resumable instruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardBodyConstructionPhase {
    Fresh,
    EarlyAttributes,
    Wheels,
    Trucks,
    Deck,
    Joints,
    Binding,
    InitialTransforms,
    Session,
    Materials,
    Complete,
    HandedOff,
}

#[derive(Debug)]
pub(crate) enum SkateboardBodyConstructionError {
    WrongPhase(SkateboardBodyConstructionPhase),
    PoisonedWheelLines,
    Attribute(SkateboardAttributeError),
    WheelAttribute(AttributeRuntimeError),
    Wheels(SkateboardWheelDefinitionError),
    Trucks(SkateboardTruckDefinitionError<SkateboardAttributeError>),
    Deck(SkateboardDeckError<SkateboardAttributeError, Infallible>),
    Joints(SkateboardJointBuildError<SkateboardAttributeError, Infallible>),
    Template(DynamicAssemblyTemplateError),
    InitialTransforms(
        SkateboardInitialTransformError<
            SkateboardAttributeError,
            Infallible,
            DynamicAssemblyPartTransformError,
        >,
    ),
    Session(DynamicAssemblySessionCreateError),
    CollisionGroups(
        crate::dynamic_assembly_session::board_collision_groups::BoardCollisionGroupError,
    ),
}

impl fmt::Display for SkateboardBodyConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPhase(phase) => write!(f, "board construction cannot run in {phase:?}"),
            Self::PoisonedWheelLines => f.write_str("board constructor wheel output lock poisoned"),
            Self::Attribute(error) => write!(f, "board constructor attribute: {error}"),
            Self::WheelAttribute(error) => write!(f, "board wheel attribute: {error}"),
            Self::Wheels(error) => write!(f, "board wheel construction: {error:?}"),
            Self::Trucks(error) => write!(f, "board truck construction: {error}"),
            Self::Deck(error) => write!(f, "board deck construction: {error}"),
            Self::Joints(error) => write!(f, "board joints: {error}"),
            Self::Template(error) => write!(f, "board template: {error}"),
            Self::InitialTransforms(error) => write!(f, "board initial placement: {error}"),
            Self::Session(error) => write!(f, "board session: {error}"),
            Self::CollisionGroups(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for SkateboardBodyConstructionError {}

/// Actual construction outputs survive until transferred to the real candidate.
/// After `take_session`, retain THIS owner together with the candidate/registered
/// member: its reader/cache are the same continuing board construction lifetime.
pub(crate) struct SkateboardBodyConstruction {
    wheel_schedule: Arc<std::sync::Mutex<SkateboardWheelSchedule>>,
    post_fields: Option<SkateboardBodyPostFields>,
    debug_colors: Option<Arc<std::sync::Mutex<debug_colors::SkateboardDebugColors>>>,
    wheel_compressions: wheel_compressions::SkateboardWheelCompressions,
    wheel_lines: Arc<std::sync::Mutex<SkateboardRetainedWheelLines>>,
    physical_owner: Option<Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>>,
    island_body_id: Option<u32>,
    attributes: SkateboardAttributes,
    wheel_attributes: SkateboardWheelAttributes,
    materials: SkateboardBodyMaterials,
    joint_count: Option<usize>,
    wheel_distance: Option<f32>,
    remain_on_reset: bool,
    collision_groups:
        Option<Arc<crate::dynamic_assembly_session::board_collision_groups::BoardCollisionGroups>>,
    attempted: bool,
    phase: SkateboardBodyConstructionPhase,
    wheels: Option<SkateboardWheelDefinitions>,
    trucks: Option<SkateboardTruckDefinitions>,
    deck: Option<SkateboardDeckDefinition>,
    joints: Vec<SkateboardJointDefinition>,
    collision: Option<Arc<SkateboardCollisionDefinition>>,
    template: Option<Arc<SkateboardRuntimeTemplate>>,
    session: Option<SkateboardAssemblySession>,
}

impl SkateboardBodyConstruction {
    /// Test probe: each emitted volume's enable bit (wheels x4, trucks x2, deck children).
    #[cfg(test)]
    pub(crate) fn volume_enables_for_test(&self) -> String {
        let Some(collision) = self.collision.as_ref() else { return "no collision".into() };
        let Ok(state) = collision.volume_flags.read() else { return "poisoned".into() };
        (0..6 + state.deck_count())
            .map(|i| state.emission_flags(i).map_or('?', |f| if f & 1 != 0 { '1' } else { '0' }))
            .collect()
    }
}

impl SkateboardBodyConstruction {
    pub(crate) fn wheel_schedule(&self) -> &Arc<std::sync::Mutex<SkateboardWheelSchedule>> {
        &self.wheel_schedule
    }
    pub(crate) fn post_fields(&self) -> Option<&SkateboardBodyPostFields> {
        self.post_fields.as_ref()
    }
    pub(crate) fn post_fields_mut(&mut self) -> Option<&mut SkateboardBodyPostFields> {
        self.post_fields.as_mut()
    }
    pub(crate) fn debug_colors(
        &self,
    ) -> Option<&Arc<std::sync::Mutex<debug_colors::SkateboardDebugColors>>> {
        self.debug_colors.as_ref()
    }

    pub(crate) fn compression_parts(
        &mut self,
    ) -> (
        &Option<f32>,
        &mut wheel_compressions::SkateboardWheelCompressions,
    ) {
        (&self.wheel_distance, &mut self.wheel_compressions)
    }
    pub(crate) fn wheel_compressions(&self) -> &wheel_compressions::SkateboardWheelCompressions {
        &self.wheel_compressions
    }
    pub(crate) fn wheel_lines(&self) -> &Arc<std::sync::Mutex<SkateboardRetainedWheelLines>> {
        &self.wheel_lines
    }
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            wheel_schedule: Arc::new(std::sync::Mutex::new(SkateboardWheelSchedule::default())),
            post_fields: None,
            debug_colors: None,
            wheel_compressions: wheel_compressions::SkateboardWheelCompressions::default(),
            wheel_lines: Arc::new(std::sync::Mutex::new(SkateboardRetainedWheelLines::new())),
            physical_owner: None,
            island_body_id: None,
            attributes: SkateboardAttributes::bind(vault),
            wheel_attributes: SkateboardWheelAttributes::bind(vault),
            materials: SkateboardBodyMaterials::new(),
            joint_count: None,
            wheel_distance: None,
            remain_on_reset: false,
            collision_groups: None,
            attempted: false,
            phase: SkateboardBodyConstructionPhase::Fresh,
            wheels: None,
            trucks: None,
            deck: None,
            joints: Vec::new(),
            collision: None,
            template: None,
            session: None,
        }
    }

    /// Bound by the actual paired construction BEFORE Init begins. The legacy
    /// Vault-only constructor does not acquire Entity provenance retroactively.
    pub(crate) fn new_for_physical_player(
        owner: Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Self {
        let mut result = Self::new(&owner.inputs().vault);
        result.physical_owner = Some(owner);
        result
    }

    pub(crate) fn physical_owner(
        &self,
    ) -> Option<&Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>> {
        self.physical_owner.as_ref()
    }

    /// Source Body ID is not reset by Island removal.
    pub(crate) fn island_body_id(&self) -> Option<u32> {
        self.island_body_id
    }
    pub(crate) fn set_island_body_id(&mut self, id: u32) {
        self.island_body_id = Some(id);
    }

    /// Only the paired constructor calls this after actual board target
    /// registration, at `HiLOD82e04188`. The enclosing manager verifies ownership.
    pub(crate) fn mark_physical_board_completed(&mut self) {
        self.remain_on_reset = true;
    }

    /// One attempt only. A returned error leaves the completed retained prefix;
    /// partial internals of an errored existing leaf retain that leaf's policy.
    pub(crate) fn try_construct(&mut self) -> Result<(), SkateboardBodyConstructionError> {
        use SkateboardBodyConstructionError as Error;
        use SkateboardBodyConstructionPhase as Phase;
        if self.attempted {
            return Err(Error::WrongPhase(self.phase));
        }
        self.attempted = true;
        // Selected ctor fields precede Init387e8. Bind their retained projection
        // at this execution boundary, not when the host projection is allocated.
        self.wheel_lines
            .lock()
            .map_err(|_| Error::PoisonedWheelLines)?
            .reset_constructor_collision_output();
        self.post_fields = Some(SkateboardBodyPostFields::from_constructor());
        self.phase = Phase::EarlyAttributes;
        try_read_early(
            &mut self.attributes,
            &mut self.joint_count,
            &mut self.wheel_distance,
        )
        .map_err(Error::Attribute)?;

        self.phase = Phase::Wheels;
        let tuning = self
            .wheel_attributes
            .try_snapshot()
            .map_err(Error::WheelAttribute)?;
        self.wheels = Some(
            SkateboardWheelDefinitions::try_create(tuning, 0, &mut HostSkateboardWheelMassNumeric)
                .map_err(Error::Wheels)?,
        );
        self.phase = Phase::Trucks;
        self.trucks = Some(
            SkateboardTruckDefinitions::try_create(
                &mut self.attributes,
                self.wheels.as_ref().expect("completed wheels").debug_end(),
                &mut HostSkateboardWheelMassNumeric,
            )
            .map_err(Error::Trucks)?,
        );
        self.phase = Phase::Deck;
        self.deck = Some(
            SkateboardDeckDefinition::try_create(
                &mut self.attributes,
                &mut HostSkateboardDeckNumeric,
                self.trucks.as_ref().expect("completed trucks").debug_end(),
            )
            .map_err(Error::Deck)?,
        );
        self.phase = Phase::Joints;
        if self.joint_count == Some(6) {
            self.joints =
                try_create_skateboard_joints(&mut self.attributes, &mut HostSkateboardJointNumeric)
                    .map_err(Error::Joints)?
                    .to_vec();
        }
        self.phase = Phase::Binding;
        let collision = Arc::new(SkateboardCollisionDefinition::from_components(
            self.wheels.take().expect("completed wheels"),
            self.trucks.take().expect("completed trucks"),
            self.deck.take().expect("completed deck"),
            initialize_skateboard_culling_table(),
        ));
        self.collision = Some(Arc::clone(&collision));
        self.debug_colors = Some(Arc::new(std::sync::Mutex::new(
            debug_colors::SkateboardDebugColors::from_constructed(Arc::clone(&collision)),
        )));
        let template = try_bind_skateboard_runtime_assembly(collision, self.joints.clone())
            .map_err(Error::Template)?;
        self.template = Some(Arc::clone(&template));
        self.phase = Phase::Session;
        self.session = Some(
            SkateboardAssemblySession::try_new(template, vec![FRESH_VOLUME_TRANSFORM; 7])
                .map_err(Error::Session)?,
        );
        self.phase = Phase::InitialTransforms;
        let session = self
            .session
            .as_mut()
            .expect("fresh session exists before placement");
        self.collision_groups = Some(Arc::clone(&session.board_collision_groups));
        try_initialize_skateboard_transforms_with_writer(
            &mut self.attributes,
            &mut HostSkateboardInitialTransformNumeric,
            &mut |index, matrix| session.try_set_detached_part_transform(index, matrix),
        )
        .map_err(Error::InitialTransforms)?;
        // Native group store then all ascending Part userdata stores. The
        // existing board publication consumes this proven board-specific setup.
        session
            .board_collision_groups
            .try_initialize(7)
            .map_err(Error::CollisionGroups)?;
        self.phase = Phase::Materials;
        try_read_materials(&mut self.attributes, &mut self.materials).map_err(Error::Attribute)?;
        self.phase = Phase::Complete;
        Ok(())
    }

    pub(crate) fn phase(&self) -> SkateboardBodyConstructionPhase {
        self.phase
    }
    pub(crate) fn materials(&self) -> &SkateboardBodyMaterials {
        &self.materials
    }
    /// The only material ownership escape. Native Init's stores have finished
    /// in both phases; construction cannot restart after either phase.
    pub(crate) fn standard_material_owners(&self) -> Option<[Arc<AssemblyPartMaterial>; 3]> {
        if !matches!(
            self.phase,
            SkateboardBodyConstructionPhase::Complete | SkateboardBodyConstructionPhase::HandedOff
        ) {
            return None;
        }
        Some([
            Arc::clone(&self.materials.deck),
            Arc::clone(&self.materials.wheel),
            Arc::clone(&self.materials.truck),
        ])
    }
    pub(crate) fn joint_count(&self) -> Option<usize> {
        self.joint_count
    }
    pub(crate) fn wheel_distance(&self) -> Option<f32> {
        self.wheel_distance
    }
    pub(crate) fn remain_on_reset(&self) -> bool {
        self.remain_on_reset
    }
    pub(crate) fn group(&self) -> Option<u32> {
        self.collision_groups
            .as_ref()?
            .try_snapshot()
            .ok()
            .map(|state| state.assembly)
    }
    pub(crate) fn part_user_data(&self) -> Option<[u32; 7]> {
        self.collision_groups
            .as_ref()?
            .try_snapshot()
            .ok()
            .map(|state| state.parts)
    }
    pub(crate) fn session(&self) -> Option<&SkateboardAssemblySession> {
        self.session.as_ref()
    }
    pub(crate) fn attributes_mut(&mut self) -> &mut SkateboardAttributes {
        &mut self.attributes
    }

    /// One-way internal candidate handoff, not admission. The caller transfers
    /// this remaining owner together with the session, including on failures.
    pub(crate) fn take_session(
        &mut self,
    ) -> Result<SkateboardAssemblySession, SkateboardBodyConstructionError> {
        if self.phase != SkateboardBodyConstructionPhase::Complete {
            return Err(SkateboardBodyConstructionError::WrongPhase(self.phase));
        }
        let session = self
            .session
            .take()
            .expect("successful construction retains session");
        self.phase = SkateboardBodyConstructionPhase::HandedOff;
        Ok(session)
    }
}

/// Byte at 0x82b37594, then distance load/store at 0x82b37684/8c. Descriptor,
/// category allocation and the intervening unused debug Instance are excluded.
fn try_read_early<A: SkateboardAttributeRead>(
    attributes: &mut A,
    joint_count: &mut Option<usize>,
    wheel_distance: &mut Option<f32>,
) -> Result<(), A::Error> {
    let pointer = attributes.lookup(Class::Trucks, 0xb1cb_e3e9_29af_c8a0)?;
    *joint_count = Some(if attributes.byte(&pointer)? == 0 {
        0
    } else {
        6
    });
    let pointer = attributes.lookup(Class::Trucks, 0x2be0_8dd7_a920_7622)?;
    *wheel_distance = Some(attributes.float(&pointer)?);
    Ok(())
}

/// Init 0x82b37788..0x82b37b6c: all eleven scalar loads precede all stores.
/// Copies only; no friction arbitration, numeric sanitization or mass operation.
fn try_read_materials<A: SkateboardAttributeRead>(
    attributes: &mut A,
    materials: &mut SkateboardBodyMaterials,
) -> Result<(), A::Error> {
    let wheel_static = attributes.read_float(Class::Wheels, 0x7d77_7041_dbfa_9234)?;
    let wheel_dynamic = attributes.read_float(Class::Wheels, 0x2a10_82dd_feef_3ea7)?;
    let deck_restitution = attributes.read_float(Class::Deck, 0x254d_958a_72d5_9a2d)?;
    let wheel_restitution = attributes.read_float(Class::Wheels, 0x5d09_5e63_6526_0b84)?;
    let truck_restitution = attributes.read_float(Class::Trucks, 0x3298_05a8_2950_ab54)?;
    let deck_static = attributes.read_float(Class::Deck, 0xaae1_43f5_64f2_63aa)?;
    let truck_static = attributes.read_float(Class::Trucks, 0xb34a_6487_0f3f_5944)?;
    let deck_dynamic = attributes.read_float(Class::Deck, 0xac32_e9a7_f005_6c6d)?;
    let truck_dynamic = attributes.read_float(Class::Trucks, 0x7e13_5b94_4834_707e)?;
    let wipeout_friction = attributes.read_float(Class::Wipeout, 0x9904_48fc_2357_51f6)?;
    let wipeout_restitution = attributes.read_float(Class::Wipeout, 0xd0fb_2f7d_77b0_bea4)?;
    // Private Arcs cannot escape until Complete/HandedOff. No make_mut: these
    // must remain the exact constructor-owned records, not copy-on-write ones.
    let wheel =
        Arc::get_mut(&mut materials.wheel).expect("preadmission material exclusively owned");
    wheel.restitution = wheel_restitution;
    wheel.static_friction = wheel_static;
    wheel.dynamic_friction = wheel_dynamic;
    let deck = Arc::get_mut(&mut materials.deck).expect("preadmission material exclusively owned");
    deck.restitution = deck_restitution;
    deck.static_friction = deck_static;
    deck.dynamic_friction = deck_dynamic;
    let truck =
        Arc::get_mut(&mut materials.truck).expect("preadmission material exclusively owned");
    truck.restitution = truck_restitution;
    truck.static_friction = truck_static;
    truck.dynamic_friction = truck_dynamic;
    let wipeout =
        Arc::get_mut(&mut materials.wipeout).expect("preadmission material exclusively owned");
    wipeout.restitution = wipeout_restitution;
    wipeout.static_friction = wipeout_friction;
    wipeout.dynamic_friction = wipeout_friction;
    let wheel_grind =
        Arc::get_mut(&mut materials.wheel_grind).expect("preadmission material exclusively owned");
    wheel_grind.restitution = 0.0;
    wheel_grind.static_friction = 0.0;
    wheel_grind.dynamic_friction = 0.0;
    let deck_grind =
        Arc::get_mut(&mut materials.deck_grind).expect("preadmission material exclusively owned");
    deck_grind.restitution = 0.0;
    deck_grind.static_friction = 0.0;
    deck_grind.dynamic_friction = 0.0;
    let truck_grind =
        Arc::get_mut(&mut materials.truck_grind).expect("preadmission material exclusively owned");
    truck_grind.restitution = 0.0;
    truck_grind.static_friction = 0.0;
    truck_grind.dynamic_friction = 0.0;
    Ok(())
}
