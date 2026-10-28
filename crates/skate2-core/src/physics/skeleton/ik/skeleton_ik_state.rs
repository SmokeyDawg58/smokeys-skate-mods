//! Retained `SkeletonIK` histories: Reset (0x82b51370), status update
//! (0x82b3db20), and blend update (0x82b51d38). The existing target owner
//! remains the only mapping/target-matrix authority. `DataIn`, constructor
//! geometry, and the retained attribute Instance belong to the aggregate.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "active IK aggregate owns retained-state integration"
    )
)]

use crate::{
    skeleton_ik_attributes::SkeletonIkAttributeRead,
    skeleton_ik_geometry::{SkeletonIkGeometry, SkeletonIkGeometryProgress},
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[path = "../../player/wipeout/physical_player_post_wipeout_foot_ik.rs"]
pub(crate) mod post_wipeout_foot;
#[path = "../../player/wipeout/physical_player_post_wipeout_hand_ik.rs"]
pub(crate) mod post_wipeout_hand;
#[path = "../../player/wipeout/physical_player_post_wipeout_ik_update.rs"]
pub(crate) mod post_wipeout_update;

#[cfg(test)]
#[path = "skeleton_ik_state_tests.rs"]
mod tests;

#[path = "skeleton_ik_blend_transforms.rs"]
pub(crate) mod blend_transforms;

#[path = "skeleton_ik_initial.rs"]
pub(crate) mod initial;

#[path = "skeleton_ik_external.rs"]
pub(crate) mod external;

#[path = "skeleton_ik_set_target_normal.rs"]
pub(crate) mod set_target_normal;

#[path = "skeleton_ik_contact.rs"]
pub(crate) mod contact;

#[path = "skeleton_ik_drive_frames.rs"]
pub(crate) mod drive_frames;

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkBlendError<A> {
    Attribute(A),
    Input(crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError),
    GeometryUnavailable(SkeletonIkGeometryProgress),
    ExternalTargetNotSet { entry: usize },
}

/// Not Clone: these arrays are the continuing IK histories, not a snapshot.
/// Raw u32 modes and contact states retain values outside the named enum cases.
pub(crate) struct SkeletonIkState {
    do_ik: bool,
    skateboard_dont_push: bool,
    tell_anim_dont_push_or_brake: bool,
    on_deck_blends: [f32; 4],
    external_blends: [f32; 4],
    modes: [u32; 4],
    external_target_positions_world: [SkeletonRootVector; 4],
    external_target_positions_local: [SkeletonRootVector; 4],
    external_target_local_deltas: [SkeletonRootVector; 4],
    external_target_normals: [SkeletonRootVector; 4],
    external_target_normal_set: [bool; 4],
    external_target_set: [bool; 4],
    local_target_set: [bool; 4],
    target_blends: [f32; 4],
    external_target_normal_blends: [f32; 4],
    part_to_world: [SkeletonRootMatrix; 4],
    part_to_world_external: [SkeletonRootMatrix; 4],
    part_to_deck: [SkeletonRootMatrix; 4],
    part_positions: [SkeletonRootVector; 4],
    parent_to_animation: [SkeletonRootMatrix; 4],
    parent_to_world: [SkeletonRootMatrix; 4],
    parent_to_world_external: [SkeletonRootMatrix; 4],
    parent_to_deck: [SkeletonRootMatrix; 4],
    contact_target_offsets: [f32; 4],
    contact_offsets: [f32; 4],
    contact_positions: [SkeletonRootVector; 4],
    contact_states: [u32; 4],
    parts_on_deck: [u32; 4],
    last_left_foot_position_deck: SkeletonRootVector,
    last_right_foot_position_deck: SkeletonRootVector,
}

impl SkeletonIkState {
    /// Final stores of Skeleton.SetLeft/RightHandIKTargetPosition82b41100/a8.
    /// The caller has already clamped the world target to the animated shoulder.
    pub(crate) fn set_world_hand_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        let entry = if left { 2 } else { 3 };
        self.external_target_positions_world[entry] = position;
        self.external_target_set[entry] = true;
        self.target_blends[entry] = blend;
    }

    /// Skeleton world-space foot setters 0x82b41250/0x82b41270. Unlike
    /// the Local setters, these select external mode 2, without transforming
    /// the position or enabling the local-delta smoothing path.
    pub(crate) fn set_world_foot_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        let entry = usize::from(!left);
        self.external_target_positions_world[entry] = position;
        self.external_target_set[entry] = true;
        self.target_blends[entry] = blend;
    }

    /// `Skeleton::SetLeft/RightFootIKTargetPositionLocal` stores the already
    /// world-to-animation transformed vector at IK+0x260/0x270, then the
    /// corresponding local-set byte and blend scalar.
    pub(crate) fn set_local_foot_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        let entry = usize::from(!left);
        self.external_target_positions_local[entry] = position;
        self.local_target_set[entry] = true;
        self.target_blends[entry] = blend;
    }

    pub(crate) fn new(targets: &mut SkeletonIkTargets) -> Self {
        // Local safe construction storage is not observable before Reset.
        let matrix = SkeletonRootMatrix::known([0.0; 16]);
        let mut result = Self {
            do_ik: false,
            skateboard_dont_push: false,
            tell_anim_dont_push_or_brake: false,
            on_deck_blends: [0.0; 4],
            external_blends: [0.0; 4],
            modes: [0; 4],
            external_target_positions_world: [ZERO; 4],
            external_target_positions_local: [ZERO; 4],
            external_target_local_deltas: [ZERO; 4],
            external_target_normals: [ZERO; 4],
            external_target_normal_set: [false; 4],
            external_target_set: [false; 4],
            local_target_set: [false; 4],
            target_blends: [0.0; 4],
            external_target_normal_blends: [0.0; 4],
            part_to_world: [matrix; 4],
            part_to_world_external: [matrix; 4],
            part_to_deck: [matrix; 4],
            part_positions: [ZERO; 4],
            parent_to_animation: [matrix; 4],
            parent_to_world: [matrix; 4],
            parent_to_world_external: [matrix; 4],
            parent_to_deck: [matrix; 4],
            contact_target_offsets: [0.0; 4],
            contact_offsets: [0.0; 4],
            contact_positions: [ZERO; 4],
            contact_states: [0; 4],
            parts_on_deck: [0; 4],
            last_left_foot_position_deck: ZERO,
            last_right_foot_position_deck: ZERO,
        };
        result.reset(targets);
        result
    }

    /// Full 0x82b51370..0x82b51710 history reset, retaining aggregate-owned
    /// `DataIn`/geometry/attributes and the existing target owner's Init mappings.
    /// Source stores occur under one exclusive borrow; concurrent native
    /// per-column matrix observers are not modeled.
    pub(crate) fn reset(&mut self, targets: &mut SkeletonIkTargets) {
        self.do_ik = true;
        self.skateboard_dont_push = false;
        self.tell_anim_dont_push_or_brake = false;
        for entry in 0..4 {
            self.modes[entry] = 0;
            self.on_deck_blends[entry] = 0.0;
            self.external_blends[entry] = 0.0;
            self.external_target_normal_blends[entry] = 0.0;
            self.contact_positions[entry] = ZERO;
            self.contact_states[entry] = 0;
            self.external_target_positions_world[entry] = ZERO;
            self.external_target_positions_local[entry] = ZERO;
            self.external_target_local_deltas[entry] = ZERO;
            self.external_target_normal_set[entry] = false;
            self.external_target_set[entry] = false;
            self.local_target_set[entry] = false;
            self.target_blends[entry] = 0.0;
            self.parts_on_deck[entry] = 0;
        }
        targets.reset_target_transforms();
        let identity = SkeletonRootMatrix::known(IDENTITY);
        self.parent_to_animation.fill(identity);
        self.parent_to_world.fill(identity);
        self.parent_to_world_external.fill(identity);
        self.parent_to_deck.fill(identity);
        self.part_to_world_external.fill(identity);
        self.part_to_world.fill(identity);
        self.part_to_deck.fill(identity);
        self.part_positions.fill(ZERO);
        self.external_target_normals.fill(ZERO);
        self.contact_target_offsets.fill(0.0);
        self.contact_offsets.fill(0.0);
        self.last_left_foot_position_deck = ZERO;
        self.last_right_foot_position_deck = ZERO;
    }

    /// 0x82b3db20..0x82b3dd44. Desired hand modes use flags+8c8 bits9/10;
    /// only the feet consult `DoIK`. Invalid raw modes are unchanged.
    pub(crate) fn update_status(&mut self, flags_8c8: u32) {
        let desired = [
            u32::from(self.do_ik),
            u32::from(self.do_ik),
            (flags_8c8 >> 9) & 1,
            (flags_8c8 >> 10) & 1,
        ];
        for (entry, desired) in desired.into_iter().enumerate() {
            match self.modes[entry] {
                0 => {
                    if self.external_target_set[entry] {
                        self.modes[entry] = 2;
                        self.external_blends[entry] = self.target_blends[entry];
                        self.on_deck_blends[entry] = 0.0;
                    } else if self.local_target_set[entry] {
                        self.external_target_local_deltas[entry] = ZERO;
                        self.modes[entry] = 3;
                        self.external_blends[entry] = self.target_blends[entry];
                        self.on_deck_blends[entry] = 0.0;
                    } else {
                        self.on_deck_blends[entry] = 0.0;
                        self.external_blends[entry] = 0.0;
                        self.modes[entry] = desired;
                    }
                }
                1 => {
                    if self.external_target_set[entry] {
                        self.modes[entry] = 2;
                        self.external_blends[entry] = self.target_blends[entry];
                    } else if self.local_target_set[entry] {
                        self.external_target_local_deltas[entry] = ZERO;
                        self.modes[entry] = 3;
                        self.external_blends[entry] = self.target_blends[entry];
                    } else if self.on_deck_blends[entry] == 0.0 {
                        self.external_blends[entry] = 0.0;
                        self.modes[entry] = desired;
                    }
                }
                2 => {
                    if !self.external_target_set[entry] {
                        self.modes[entry] = 3;
                    }
                }
                3 => {
                    if self.external_target_set[entry] {
                        self.modes[entry] = 2;
                        self.external_blends[entry] = self.target_blends[entry];
                    }
                    // Independent second test consumes the newly stored blend.
                    if !self.local_target_set[entry] && self.external_blends[entry] == 0.0 {
                        self.external_target_local_deltas[entry] = ZERO;
                        self.modes[entry] = desired;
                    }
                }
                _ => {}
            }
        }
    }

    /// 0x82b51d38..0x82b520cc. Four lazy attribute reads precede all blend
    /// stores, even for disabled/invalid modes. `PartPos` is prior retained state.
    pub(crate) fn try_update_blend_values<A: SkeletonIkAttributeRead>(
        &mut self,
        geometry: &SkeletonIkGeometry,
        flags_8d0: u32,
        attributes: &mut A,
    ) -> Result<(), SkeletonIkBlendError<A::Error>> {
        self.try_update_blend_values_read(geometry, || Ok(flags_8d0 & 0x1000_0000 != 0), attributes)
    }

    pub(crate) fn try_update_blend_values_read<A: SkeletonIkAttributeRead>(
        &mut self,
        geometry: &SkeletonIkGeometry,
        mut read_blend_flag: impl FnMut()->Result<bool, crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
        attributes: &mut A,
    ) -> Result<(), SkeletonIkBlendError<A::Error>> {
        use SkeletonIkBlendError as Error;
        let width = geometry.box_half_width().ok_or(Error::GeometryUnavailable(
            SkeletonIkGeometryProgress::BoxHalfWidth,
        ))?;
        let length = geometry
            .total_half_length()
            .ok_or(Error::GeometryUnavailable(
                SkeletonIkGeometryProgress::TotalHalfLength,
            ))?;
        let base = [width, 0.0, length, 0.0];
        let offset = attributes
            .vector(0xaa37_8f2e_bf7a_a313)
            .map_err(Error::Attribute)?;
        let extent: [f32; 4] = std::array::from_fn(|lane| base[lane] + offset[lane]);
        let range = attributes
            .vector(0x7ce3_2881_4f93_09ae)
            .map_err(Error::Attribute)?;
        let expanded: [f32; 4] = std::array::from_fn(|lane| extent[lane] + range[lane]);
        let on_deck_step = attributes
            .scalar(0x39fa_8c07_e835_6284)
            .map_err(Error::Attribute)?;
        let external_step = attributes
            .scalar(0x094a_ce5d_e068_51fc)
            .map_err(Error::Attribute)?;
        for entry in 0..4 {
            match self.modes[entry] {
                2 => {
                    if !self.external_target_set[entry] {
                        return Err(Error::ExternalTargetNotSet { entry });
                    }
                    let remaining = self.on_deck_blends[entry] - on_deck_step;
                    self.external_blends[entry] = self.target_blends[entry];
                    self.on_deck_blends[entry] = select(-remaining, 0.0, remaining);
                }
                3 => {
                    self.external_blends[entry] = if self.local_target_set[entry] {
                        1.0
                    } else {
                        decay(self.external_blends[entry], external_step)
                    };
                    self.on_deck_blends[entry] = decay(self.on_deck_blends[entry], on_deck_step);
                }
                1 => {
                    let target = if entry < 2 {
                        Some(if read_blend_flag().map_err(Error::Input)? {
                            0.0
                        } else {
                            1.0
                        })
                    } else {
                        hand_target(
                            self.part_positions[entry],
                            self.on_deck_blends[entry],
                            extent,
                            range,
                            expanded,
                        )
                    };
                    if let Some(target) = target {
                        self.on_deck_blends[entry] =
                            approach(self.on_deck_blends[entry], target, on_deck_step);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) const fn do_ik(&self) -> bool {
        self.do_ik
    }
    /// `Skeleton::Init82b62ae0..62b04`: DisableIK's stores, then EnableIK's.
    pub(crate) fn finish_skeleton_init(&mut self) {
        self.disable();
        self.enable();
    }

    /// `Skeleton::EnableIK` 0x82b462e0 stores only this flag.
    pub(crate) fn enable(&mut self) {
        self.do_ik = true;
    }
    /// `Skeleton::DisableIK` 0x82b462f0 clears each entry's on-deck blend, mode
    /// and external blend before the flag, so no part keeps its IK.
    pub(crate) fn disable(&mut self) {
        for entry in 0..4 {
            self.on_deck_blends[entry] = 0.0;
            self.modes[entry] = 0;
            self.external_blends[entry] = 0.0;
        }
        self.do_ik = false;
    }
    /// Skeleton.ResetPushAbort82b3bb80 clears only the retained IK push veto.
    pub(crate) fn reset_push_abort(&mut self) {
        self.skateboard_dont_push = false;
    }

    /// `RequestPushExit82b3bb70` sets only IK+c71.
    pub(crate) fn request_push_exit(&mut self) {
        self.tell_anim_dont_push_or_brake = true;
    }

    pub(crate) const fn skateboard_dont_push(&self) -> bool {
        self.skateboard_dont_push
    }
    pub(crate) const fn tell_anim_dont_push_or_brake(&self) -> bool {
        self.tell_anim_dont_push_or_brake
    }
    pub(crate) const fn modes(&self) -> &[u32; 4] {
        &self.modes
    }
    pub(crate) const fn on_deck_blends(&self) -> &[f32; 4] {
        &self.on_deck_blends
    }
    pub(crate) const fn external_blends(&self) -> &[f32; 4] {
        &self.external_blends
    }
    pub(crate) const fn part_positions(&self) -> &[SkeletonRootVector; 4] {
        &self.part_positions
    }
    pub(crate) const fn external_target_positions_world(&self) -> &[SkeletonRootVector; 4] {
        &self.external_target_positions_world
    }
    pub(crate) const fn external_target_positions_local(&self) -> &[SkeletonRootVector; 4] {
        &self.external_target_positions_local
    }
    pub(crate) const fn external_target_local_deltas(&self) -> &[SkeletonRootVector; 4] {
        &self.external_target_local_deltas
    }
    pub(crate) const fn external_target_normals(&self) -> &[SkeletonRootVector; 4] {
        &self.external_target_normals
    }
    pub(crate) const fn external_target_normal_set(&self) -> &[bool; 4] {
        &self.external_target_normal_set
    }
    pub(crate) const fn external_target_set(&self) -> &[bool; 4] {
        &self.external_target_set
    }
    pub(crate) const fn local_target_set(&self) -> &[bool; 4] {
        &self.local_target_set
    }
    pub(crate) const fn target_blends(&self) -> &[f32; 4] {
        &self.target_blends
    }
    pub(crate) const fn external_target_normal_blends(&self) -> &[f32; 4] {
        &self.external_target_normal_blends
    }
    pub(crate) const fn part_to_world(&self) -> &[SkeletonRootMatrix; 4] {
        &self.part_to_world
    }
    pub(crate) const fn part_to_world_external(&self) -> &[SkeletonRootMatrix; 4] {
        &self.part_to_world_external
    }
    pub(crate) const fn part_to_deck(&self) -> &[SkeletonRootMatrix; 4] {
        &self.part_to_deck
    }
    pub(crate) const fn parent_to_animation(&self) -> &[SkeletonRootMatrix; 4] {
        &self.parent_to_animation
    }
    pub(crate) const fn parent_to_world(&self) -> &[SkeletonRootMatrix; 4] {
        &self.parent_to_world
    }
    pub(crate) const fn parent_to_world_external(&self) -> &[SkeletonRootMatrix; 4] {
        &self.parent_to_world_external
    }
    pub(crate) const fn parent_to_deck(&self) -> &[SkeletonRootMatrix; 4] {
        &self.parent_to_deck
    }
    pub(crate) const fn contact_target_offsets(&self) -> &[f32; 4] {
        &self.contact_target_offsets
    }
    pub(crate) const fn contact_offsets(&self) -> &[f32; 4] {
        &self.contact_offsets
    }
    pub(crate) const fn contact_positions(&self) -> &[SkeletonRootVector; 4] {
        &self.contact_positions
    }
    pub(crate) const fn contact_states(&self) -> &[u32; 4] {
        &self.contact_states
    }
    pub(crate) const fn parts_on_deck(&self) -> &[u32; 4] {
        &self.parts_on_deck
    }
    pub(crate) const fn last_left_foot_position_deck(&self) -> SkeletonRootVector {
        self.last_left_foot_position_deck
    }
    pub(crate) const fn last_right_foot_position_deck(&self) -> SkeletonRootVector {
        self.last_right_foot_position_deck
    }
}

fn select(test: f32, nonnegative: f32, other: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { other }
}

fn decay(value: f32, step: f32) -> f32 {
    let remaining = value - step;
    select(-remaining, 0.0, remaining)
}

fn approach(old: f32, target: f32, step: f32) -> f32 {
    let negative_step = -step;
    let delta = target - old;
    let lower = select(negative_step - delta, negative_step, delta);
    let bounded = select(step - lower, lower, step);
    bounded + old
}

fn hand_target(
    position: SkeletonRootVector,
    old_blend: f32,
    extent: [f32; 4],
    range: [f32; 4],
    expanded: [f32; 4],
) -> Option<f32> {
    let absolute = position
        .xyz
        .map(|value| f32::from_bits(value.to_bits() & 0x7fff_ffff));
    // BO4 !LT rejects unordered, but positive old blend bypasses all box tests.
    if !(old_blend > 0.0
        || (absolute[0] < expanded[0] && absolute[1] < expanded[1] && absolute[2] < expanded[2]))
    {
        return None;
    }
    let mut target = 1.0;
    if absolute[0] > extent[0] {
        target = 1.0 - ((absolute[0] - extent[0]) / range[0]);
    }
    if absolute[2] > extent[2] {
        let z = 1.0 - ((absolute[2] - extent[2]) / range[2]);
        target = select(target - z, z, target);
    }
    if absolute[1] > extent[1] {
        let y = 1.0 - ((absolute[1] - extent[1]) / range[1]);
        target = select(y - target, target, y);
    }
    let lower = select(-target, 0.0, target);
    Some(select(1.0 - lower, lower, 1.0))
}

#[path = "skeleton_ik_fill_phys_out.rs"]
pub(crate) mod fill_phys_out;

impl SkeletonIkState {
    pub(crate) fn begin_wipeout_adjust(&mut self) {
        self.do_ik = true;
    }
    pub(crate) fn seed_wipeout_foot_blends(&mut self) {
        self.on_deck_blends[1] = 1.0;
        self.on_deck_blends[0] = 1.0;
    }
    /// `UpdateWipeoutAdjust` 0x82b59db8..0x82b59ddc ends with DisableIK's stores.
    pub(crate) fn finish_wipeout_adjust(&mut self) {
        self.disable();
    }
}
