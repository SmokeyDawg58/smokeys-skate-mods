//! Bounded processed Contact inputs: hierarchy ID and the physical toolkit
//! mass/deck/effective prefix. Actual Actor scheduling and attribute dispatch
//! remain outside this producer; a ready projection does not establish them.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "processed input producer awaits actual Actor scheduling"
    )
)]
use crate::{
    skeleton_data::SkeletonData, skeleton_ik_state::contact::SkeletonIkContactProcessed,
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[path = "skeleton_processed_anim_attributes.rs"]
pub(crate) mod anim_attributes;
#[path = "skeleton_processed_animation_input.rs"]
pub(crate) mod animation_input;
#[path = "skeleton_processed_ground_force_input.rs"]
pub(crate) mod ground_force_input;
#[path = "skeleton_processed_ground_position.rs"]
pub(crate) mod ground_position;
#[path = "skeleton_processed_jump_events.rs"]
pub(crate) mod jump_events;
#[path = "skeleton_processed_physics_state.rs"]
pub(crate) mod physics_state;
#[path = "skeleton_processed_prepare.rs"]
pub(crate) mod prepare;
#[path = "skeleton_processed_prior_output.rs"]
pub(crate) mod prior_output;
#[path = "skeleton_processed_process_data_output.rs"]
pub(crate) mod process_data_output;
#[path = "skeleton_processed_toolkit.rs"]
pub(crate) mod toolkit;

#[path = "skeleton_processed_line_surfaces.rs"]
pub(crate) mod line_surfaces;

#[path = "skeleton_processed_input_controls.rs"]
pub(crate) mod input_controls;

#[cfg(test)]
#[path = "skeleton_processed_contact_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "skeleton_processed_flags_tests.rs"]
mod flags_tests;

/// The implementation must borrow the actual board and its retained processed
/// flags. Both transform calls read physical Part 6 independently.
pub(crate) trait SkeletonProcessedContactBoard {
    type Error;
    fn mass(&mut self) -> Result<f32, Self::Error>;
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn flags_8c4(&self) -> u32;
}

#[path = "skeleton_processed_grounded_runtime.rs"]
pub(crate) mod grounded_runtime;

pub(crate) struct SkeletonProcessedContact {
    hang_flags: hang_flags::ProcessedHangFlags,
    primary_grab: Option<crate::physical_player_grab_spline::GrabSpline>,
    alternate_grab: Option<crate::physical_player_grab_spline::GrabSpline>,
    skateboard_orientation: Option<SkeletonRootMatrix>,
    anim_attributes: anim_attributes::ProcessedAnimAttributeFields,
    process_data_output: process_data_output::ProcessDataOutput,
    controls: input_controls::ProcessedInputControls,
    line_surfaces: line_surfaces::SkeletonProcessedLineSurfaces,
    constructor_fields_initialized: bool,
    flags_8c8: Option<u32>,
    physics_state: physics_state::ProcessedPhysicsStateFields,
    ground_position: ground_position::ProcessedGroundPosition,
    animation_input: animation_input::ProcessedAnimationInput,
    contact_bone_id: Option<usize>,
    physical_mass: Option<f32>,
    deck: Option<SkeletonRootMatrix>,
    effective: Option<SkeletonRootMatrix>,
    toolkit_prefix_complete: bool,
    toolkit: toolkit::SkeletonProcessedToolkitState,
    prior_output: prior_output::SkeletonProcessedPriorOutputState,
    surface_projection: crate::skateboard_processed_surface::SkateboardSurfaceProjection,
    surface_ready: bool,
    jump_events: jump_events::SkeletonProcessedJumpEvents,
    prepare_output: prepare::SkeletonProcessedPrepareOutput,
    gravity: Option<crate::physical_player_phys_out::PhysOutValue<f32>>,
    /// +420 `GrindInvestigation`, published by `GrindManager::Update` 82de1924.
    grind_investigation: Option<Box<crate::physical_player_grind_manager::types::Investigation>>,
}
impl SkeletonProcessedContact {
    pub(crate) fn grab_spline(
        &self,
        alternate: bool,
    ) -> Option<&crate::physical_player_grab_spline::GrabSpline> {
        if alternate {
            self.alternate_grab.as_ref()
        } else {
            self.primary_grab.as_ref()
        }
    }
    /// UpdateProcessedPhysIn82dcbb18 copies only a ready, nonnull source spline.
    /// Keep the already constructed destination occurrence and its lifetime.
    pub(crate) fn publish_grab_spline(
        &mut self,
        spline: &crate::physical_player_grab_spline::GrabSpline,
        alternate: bool,
    ) -> bool {
        let target = if alternate {
            self.alternate_grab.as_mut()
        } else {
            self.primary_grab.as_mut()
        };
        let Some(target) = target else {
            return false;
        };
        target.clone_from(spline);
        true
    }
    /// Actual Processed+9c8, loaded by board `FillPhysOut82b3070c`.
    pub(crate) fn animation_wheel_hardness(&self) -> Option<f32> {
        self.animation_input.wheel_hardness
    }
    /// Actual `HiLOD` Sim allocation82e03ee4 before Processed ctor03ef0.
    /// Category55's OR100 clears constructor-unwritten selected payload bytes.
    pub(crate) fn try_initialize_constructor_fields_in_sim_allocation(
        &mut self,
    ) -> Result<(), SkeletonProcessedConstructorAlreadyInitialized> {
        if self.constructor_fields_initialized {
            return Err(SkeletonProcessedConstructorAlreadyInitialized);
        }
        self.animation_input.initialize_sim_residuals();
        self.prior_output.initialize_tail_sim_residuals();
        self.physics_state.ai_flags = (0, u32::MAX);
        self.try_initialize_constructor_fields()
    }
    /// Same native second bitfield word. Reset82b39430 establishes all zero;
    /// no complete contact producer is implied by knowing this reset value.
    pub(crate) const fn known_flags_8c8(&self) -> Option<u32> {
        self.flags_8c8
    }
    /// Selected represented constructor fields: `HiLOD` alloc03ee4/ctor03ef0
    /// publishes this same occurrence at03efc. Processed ctor82e00de0 calls
    /// Reset82b39098 explicitly at82e00f78, after its member constructors.
    /// Both embedded grab members keep their original constructor/reset lifetime.
    pub(crate) fn try_initialize_constructor_fields(
        &mut self,
    ) -> Result<(), SkeletonProcessedConstructorAlreadyInitialized> {
        if self.constructor_fields_initialized {
            return Err(SkeletonProcessedConstructorAlreadyInitialized);
        }
        self.constructor_fields_initialized = true;
        self.controls.initialize_constructor_fields();
        self.hang_flags.initialize_constructor(); //82e00ef4, before GrabSpline
        // GrindGeometry ctor82e00eec then GrindInvestigation::Reset82e00ef4.
        self.grind_investigation = Some(Box::new(
            crate::physical_player_grind_manager::types::Investigation::reset(),
        ));
        // Actual embedded ctor82e00f64 before Processed.Reset82e00f78.
        self.primary_grab =
            Some(crate::physical_player_grab_spline::GrabSpline::in_uninitialized_allocation());
        // Alternate GrabSpline at+7c0, ctor82e00f6c.
        self.alternate_grab =
            Some(crate::physical_player_grab_spline::GrabSpline::in_uninitialized_allocation());
        self.reset();
        Ok(())
    }
    pub(crate) fn physics_state_fields(&self) -> &physics_state::ProcessedPhysicsStateFields {
        &self.physics_state
    }
    pub(crate) fn physics_state_fields_mut(
        &mut self,
    ) -> &mut physics_state::ProcessedPhysicsStateFields {
        &mut self.physics_state
    }
    /// Actual retained 8c4 word, shared with the selected jump-event producer.
    /// Unknown bits are not silently returned as source zeros.
    pub(crate) const fn known_flags_8c4(&self) -> (u32, u32) {
        self.jump_events.known_word()
    }

    /// Native Update1 82e03aa4 inserts only the returned selector ready bit.
    pub(crate) fn publish_trajectory_selector_ready(&mut self, ready: bool) {
        if (self.jump_events.known_word().0 & 0x400 != 0) != ready
            && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some()
        {
            eprintln!("known air: selector ready -> {ready}");
        }
        self.jump_events.process_animation_flag(0x400, ready);
    }

    /// Native Update1 82e0398c clears only the consumed jump event.
    pub(crate) fn clear_consumed_jump_event(&mut self) {
        self.jump_events.process_animation_bits(0x0040_0000, 0);
    }

    pub(crate) const fn board_flipped(&self) -> Option<bool> {
        let (value, known) = self.known_flags_8c4();
        if known & 0x0010_0000 == 0 {
            None
        } else {
            Some(value & 0x0010_0000 != 0)
        }
    }

    /// Selected `HiLOD::ProcessAnimOut` 82e00030..58. The input is the actual
    /// AnimOutPhysIn.mBoardIsFlipped byte at +2880, not a derived direction.
    /// Only its low bit is inserted. Full animation-input scheduling is separate.
    pub(crate) fn process_anim_out_board_flip_byte(&mut self, raw_byte: u8) {
        self.jump_events.process_board_flip_byte(raw_byte);
    }

    pub(crate) const fn new() -> Self {
        Self {
            skateboard_orientation: None,
            anim_attributes: anim_attributes::ProcessedAnimAttributeFields::new(),
            process_data_output: process_data_output::ProcessDataOutput::new(),
            controls: input_controls::ProcessedInputControls::new(),
            line_surfaces: line_surfaces::SkeletonProcessedLineSurfaces::new(),
            constructor_fields_initialized: false,
            primary_grab: None,
            alternate_grab: None,
            hang_flags: hang_flags::ProcessedHangFlags(None),
            flags_8c8: None,
            physics_state: physics_state::ProcessedPhysicsStateFields::new(),
            ground_position: ground_position::ProcessedGroundPosition::new(),
            animation_input: animation_input::ProcessedAnimationInput::new(),
            contact_bone_id: Some(0),
            physical_mass: None,
            deck: None,
            effective: None,
            toolkit_prefix_complete: false,
            toolkit: toolkit::SkeletonProcessedToolkitState::new(),
            prior_output: prior_output::SkeletonProcessedPriorOutputState::new(),
            surface_projection:
                crate::skateboard_processed_surface::SkateboardSurfaceProjection::new(),
            surface_ready: false,
            jump_events: jump_events::SkeletonProcessedJumpEvents::uninitialized(),
            prepare_output: prepare::SkeletonProcessedPrepareOutput::uninitialized(),
            gravity: None,
            grind_investigation: None,
        }
    }

    /// `ProcessedPhysIn::Reset` ID store 0x82b39500 is zero, not -1. Invalidating
    /// readiness is host bookkeeping; it does not invent writes to native lanes.
    pub(crate) fn reset(&mut self) {
        // Reset390f0/390fc/39100/39108 establishes Y/X/Z/P0 on the SAME +0 matrix.
        self.skateboard_orientation = Some(SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]));
        self.process_data_output.reset();
        self.anim_attributes.reset();
        self.controls.reset();
        self.line_surfaces.reset();
        self.prior_output.reset_spatial_fields();
        // 82b39444 stores 820100b8, distinct from positive prepared decay.
        self.gravity = Some(crate::physical_player_phys_out::PhysOutValue {
            value: f32::from_bits(0xc11c_cccd),
            origin: crate::physical_player_phys_out::PhysOutOrigin::Reset,
        });
        self.physics_state.reset();
        // Reset394f0 clears only GrindInvestigation+170 (mIsGrindPossible).
        if let Some(investigation) = &mut self.grind_investigation {
            investigation.possible = false;
        }
        // Actual primary reset follows AI flag clearing. Generic reset does not
        // create an embedded occurrence whose constructor has not run.
        if let Some(primary) = &mut self.primary_grab {
            primary.reset();
        }
        // Reset82b393fc follows the primary reset at82b393f4.
        if let Some(alternate) = &mut self.alternate_grab {
            alternate.reset();
        }
        self.ground_position.reset();
        self.animation_input.reset();
        self.prepare_output.reset_source_fields();
        self.jump_events.reset_source_fields();
        self.flags_8c8 = Some(0);
        self.contact_bone_id = Some(0);
        self.prior_output.invalidate();
        self.surface_ready = false;
        self.toolkit_prefix_complete = false;
        self.toolkit.invalidate();
    }
    pub(crate) const fn gravity(
        &self,
    ) -> Option<crate::physical_player_phys_out::PhysOutValue<f32>> {
        self.gravity
    }

    /// `ProcessAnimAttributes` 0x82b5a418..0x82b5a4b8, stopping immediately after
    /// the hierarchy-ID store. The caller retains raw record order. Unknown
    /// attribute names still store the lookup result; push/brake flag dispatch
    /// is a later source operation and is deliberately not inferred here.
    pub(crate) fn apply_raw_attribute(&mut self, data: &SkeletonData, raw: &[u8; 0x40]) -> bool {
        if raw[0x35] != 3 || raw[0x34] & 0x0c == 0 {
            return false;
        }
        let words: [u32; 5] = std::array::from_fn(|index| {
            u32::from_be_bytes(
                raw[index * 4..index * 4 + 4]
                    .try_into()
                    .expect("fixed record"),
            )
        });
        self.contact_bone_id = data
            .hierarchy()
            .packed_bone_names
            .iter()
            .position(|name| name.words == words);
        true
    }

    /// `PrepareToolkitInput` 0x82b26920..0x82b269e0, including the independent
    /// `GetEffectiveTransform` read at 0x82b2379c and late flags at 0x82b237a8.
    /// A failure retains each already committed field and prevents readiness.
    pub(crate) fn try_sample_toolkit_prefix<B: SkeletonProcessedContactBoard>(
        &mut self,
        board: &mut B,
    ) -> Result<(), B::Error> {
        try_sample_toolkit_prefix_fields(
            &mut self.physical_mass,
            &mut self.deck,
            &mut self.effective,
            &mut self.toolkit_prefix_complete,
            &mut self.toolkit,
            board,
            |board| Ok(board.flags_8c4()),
        )
    }

    /// Flags are the current retained processed +8c4 projection supplied by
    /// the caller after its attribute operations, not the earlier board-flip
    /// read. This immutable snapshot excludes callback alias mutation. It is
    /// not a replacement for the unreconstructed native phase scheduler.
    pub(crate) fn ready(&self, flags_8c4: u32) -> Option<SkeletonProcessedContactReady> {
        self.toolkit_prefix_complete
            .then(|| SkeletonProcessedContactReady {
                flags_8c4,
                contact_bone_id: self.contact_bone_id,
                effective: self.effective.expect("completed effective store"),
            })
    }

    pub(crate) const fn physical_mass(&self) -> Option<f32> {
        self.physical_mass
    }
    pub(crate) const fn deck_transform(&self) -> Option<&SkeletonRootMatrix> {
        self.deck.as_ref()
    }
    pub(crate) fn completed_deck_transform(&self) -> Option<&SkeletonRootMatrix> {
        self.toolkit_prefix_complete
            .then_some(self.deck.as_ref())
            .flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkeletonProcessedConstructorAlreadyInitialized;

fn flip_sign(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

/// Only the source-backed producer can create this complete Contact snapshot.
pub(crate) struct SkeletonProcessedContactReady {
    flags_8c4: u32,
    contact_bone_id: Option<usize>,
    effective: SkeletonRootMatrix,
}
impl SkeletonIkContactProcessed for SkeletonProcessedContactReady {
    fn flags_8c4(&self) -> u32 {
        self.flags_8c4
    }
    fn contact_bone_id(&self) -> Option<usize> {
        self.contact_bone_id
    }
    fn effective_transform(&self) -> &SkeletonRootMatrix {
        &self.effective
    }
}

impl SkeletonProcessedContact {
    pub(crate) fn begin_surface_dispatch(
        &mut self,
        output: &crate::physical_player_phys_out::PhysicalPlayerPhysOut,
    ) -> bool {
        self.surface_ready = false;
        self.toolkit_prefix_complete = false;
        self.toolkit.invalidate();
        self.prior_output.invalidate();
        self.prior_output.bind_output_occurrence(output)
    }
    pub(crate) fn selected_surface(
        &self,
    ) -> Option<&crate::skateboard_processed_surface::SkateboardSurfaceProjection> {
        self.surface_ready.then_some(&self.surface_projection)
    }
}
impl crate::skateboard_processed_surface::SkateboardSurfacePublication
    for SkeletonProcessedContact
{
    type Error = std::convert::Infallible;
    fn publish_physics_type(&mut self, value: u32) -> Result<(), Self::Error> {
        self.surface_ready = false;
        self.surface_projection.physics_type = Some(value);
        Ok(())
    }
    fn publish_collection(
        &mut self,
        value: crate::skateboard_slide_collision_attributes::SkateboardSelectedSurface,
    ) -> Result<(), Self::Error> {
        self.surface_projection.selected = Some(value);
        Ok(())
    }
    fn publish_material(
        &mut self,
        value: crate::skateboard_processed_surface::SkateboardSelectedMaterial,
    ) -> Result<(), Self::Error> {
        self.surface_projection.material = Some(value);
        self.surface_ready = true;
        Ok(())
    }
}

/// Shared prefix destinations permit late reads from the same processed owner.
fn try_sample_toolkit_prefix_fields<B: SkeletonProcessedContactBoard>(
    physical_mass: &mut Option<f32>,
    deck: &mut Option<SkeletonRootMatrix>,
    effective: &mut Option<SkeletonRootMatrix>,
    prefix_complete: &mut bool,
    toolkit: &mut toolkit::SkeletonProcessedToolkitState,
    board: &mut B,
    flags: impl FnOnce(&B) -> Result<u32, B::Error>,
) -> Result<(), B::Error> {
    *prefix_complete = false;
    toolkit.invalidate();
    *physical_mass = Some(board.mass()?);
    *deck = Some(board.deck_transform()?);
    let mut matrix = board.deck_transform()?;
    if flags(board)? & 0x0010_0000 != 0 {
        let mut columns = *matrix.columns();
        for index in [0, 2] {
            columns[index].xyz = columns[index].xyz.map(flip_sign);
            columns[index].w = columns[index].w.map(flip_sign);
        }
        matrix = SkeletonRootMatrix::from_columns(columns);
    }
    *effective = Some(matrix);
    *prefix_complete = true;
    Ok(())
}

#[path = "skeleton_processed_grind_investigation.rs"]
pub(crate) mod grind_investigation;
#[path = "skeleton_processed_grind_permission.rs"]
pub(crate) mod grind_permission;

#[path = "skeleton_processed_reckoning_input.rs"]
mod reckoning_input;

#[path = "skeleton_processed_steering_calculate.rs"]
pub(crate) mod steering_calculate;

#[path = "skeleton_processed_speed_model_input.rs"]
pub(crate) mod speed_model_input;

#[path = "skeleton_processed_speed_model_source.rs"]
pub(crate) mod speed_model_source;

#[path = "skeleton_processed_hang_flags.rs"]
pub(crate) mod hang_flags;
