//! `SkeletonIK::CalculateInitialPartTransforms`, 0x82b51710..0x82b51d38.
//! Child of the existing retained state; the actual previous record and target
//! owner remain borrowed. No current pose or replacement mapping is produced.

use super::{SkeletonIkState, external::SkeletonIkExternalError};
use crate::{
    animation::AnimationMatrix, reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_spawn_math::skateboard_spawn_rigid_inverse, skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_attributes::SkeletonIkAttributeRead, skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[cfg(test)]
#[path = "skeleton_ik_initial_tests.rs"]
pub(crate) mod tests;

const DECK_EXTENT_KEY: u64 = 0x121e_67e2_b409_6a41;

/// Each read occurs at its native stage, including repeated debug samples.
pub(crate) trait SkeletonIkInitialCvars {
    type Error;
    fn draw_feet_on_deck(&mut self) -> Result<u8, Self::Error>;
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn maximum_external_normal_angle(&mut self) -> Result<f32, Self::Error>;
    fn target_normal_blend_speed(&mut self) -> Result<f32, Self::Error>;
    fn maximum_local_delta(&mut self) -> Result<f32, Self::Error>;
    fn maximum_world_offset(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkRecordRead {
    Deck,
    Primary { entry: usize, part: usize },
    Parent { entry: usize, part: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkInitialError<A, C, N, R> {
    Attribute(A),
    Cvar(C),
    MissingPrimaryPart { entry: usize },
    RecordUnavailable(SkeletonIkRecordRead),
    External(SkeletonIkExternalError<C, N, R>),
}

impl SkeletonIkState {
    /// 0x82b51710: inverse old deck, extent, debug, then four ascending entries.
    /// Checked reads replace invalid native pointers at their actual stage.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_calculate_initial_transforms<
        A: SkeletonIkAttributeRead,
        C: SkeletonIkInitialCvars,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        data: &SkeletonIkDataIn,
        targets: &SkeletonIkTargets,
        previous: &[AnimationMatrix],
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
        trigonometry: &mut R,
    ) -> Result<(), SkeletonIkInitialError<A::Error, C::Error, N::Error, R::Error>> {
        use SkeletonIkInitialError as Error;
        let deck = previous
            .first()
            .ok_or(Error::RecordUnavailable(SkeletonIkRecordRead::Deck))?;
        let inverse_deck = SkeletonRootMatrix::known(skateboard_spawn_rigid_inverse(deck));
        let extent = attributes
            .vector(DECK_EXTENT_KEY)
            .map_err(Error::Attribute)?;
        // The first enabled text call and the later text/color calls are BLR.
        // Only the later color call's singleton acquisition can refuse.
        cvars.draw_feet_on_deck().map_err(Error::Cvar)?;
        for entry in 0..4 {
            let mode = self.modes[entry];
            self.parts_on_deck[entry] = 0;
            if mode == 2 || mode == 3 {
                self.try_calculate_external_offset(
                    entry,
                    data,
                    targets,
                    previous,
                    cvars,
                    numeric,
                    trigonometry,
                )
                .map_err(Error::External)?;
            }
            if !(self.modes[entry] == 1
                || self.on_deck_blends[entry] > 0.0
                || self.external_blends[entry] < 1.0)
            {
                continue;
            }
            let part = targets.parts()[entry].ok_or(Error::MissingPrimaryPart { entry })?;
            // Native target full4 loads precede previous-record primary loads.
            let target = SkeletonRootMatrix::known(targets.targets()[entry]);
            let primary = previous.get(part.index()).ok_or(Error::RecordUnavailable(
                SkeletonIkRecordRead::Primary {
                    entry,
                    part: part.index(),
                },
            ))?;
            let deck_target = inverse_deck.compose(&target);
            let world = data
                .animation_to_world
                .compose(&SkeletonRootMatrix::known(*primary));
            self.part_positions[entry] = deck_target.columns()[3];
            // Native deck Z/P/X/Y then world X/Y/Z/P. Exclusive access has no
            // intervening service; each complete matrix publishes together.
            self.part_to_deck[entry] = deck_target;
            self.part_to_world[entry] = world;
            self.parts_on_deck[entry] = u32::from((0..3).all(|axis| {
                let lower = f32::from_bits(extent[axis].to_bits() ^ 0x8000_0000);
                let position = self.part_positions[entry].xyz[axis];
                !(lower > position || position > extent[axis])
            }));
            if cvars.draw_feet_on_deck().map_err(Error::Cvar)? != 0 {
                cvars.require_debug_overlay().map_err(Error::Cvar)?;
            }
            if let Some(parent) = targets.parents()[entry] {
                let parent_row = previous
                    .get(parent.index())
                    .ok_or(Error::RecordUnavailable(SkeletonIkRecordRead::Parent {
                        entry,
                        part: parent.index(),
                    }))?;
                let inverse_primary =
                    SkeletonRootMatrix::known(skateboard_spawn_rigid_inverse(primary));
                let relation = inverse_primary.compose(&SkeletonRootMatrix::known(*parent_row));
                self.parent_to_deck[entry] = deck_target.compose(&relation);
                self.parent_to_world[entry] = world.compose(&relation);
            }
        }
        Ok(())
    }
}
