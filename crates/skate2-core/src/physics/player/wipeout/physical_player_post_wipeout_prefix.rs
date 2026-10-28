//! Child of `SkeletonNormalAssembly`. Real82b578b8..579f0 prefix only.
//! Caller must next execute active root adjustment, foot/hand IK, IK postupdate,
//! `HeadTracking` and pose publication. Success is NOT completed Update4.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_KNOWN_AIR, PHYSICS_STATE_PHYSICS_AIR,
    PHYSICS_STATE_TELEPORTING,
};
use super::SkeletonNormalAssembly;
use crate::{
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    reckoning_numeric::ReckoningNumeric,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_body_construction::SkateboardBodyConstruction,
    skeleton_assembly_construction::constructor::SkeletonOuterRemainder,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootMatrix,
    skeleton_skateboard_wobble::{
        SkeletonSkateboardWobble,
        post_wipeout_update::{WobbleAdjustment, WobbleUpdateDebug, WobbleUpdateError},
    },
};

/// Every reference comes from the SAME retained world pair and its constructor.
/// Root validates that identity before borrowing; none is a detached snapshot.
pub(crate) struct PostWipeoutPrefixOwners<'a> {
    pub(crate) processed: &'a SkeletonProcessedContact,
    pub(crate) board: &'a SkateboardAssemblySession,
    pub(crate) board_construction: &'a SkateboardBodyConstruction,
    pub(crate) backend: &'a SkateboardSimulationBackend,
    pub(crate) outer: &'a mut SkeletonOuterRemainder,
    pub(crate) wobble: &'a mut SkeletonSkateboardWobble,
}

/// Captured native locals for the MANDATORY continuation. In particular,
/// `do_adjust=true` requires579f4..57b94 before using deck for IK.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent captured native branch flags"
)]
pub(crate) struct PostWipeoutBeforeRootAdjustment {
    pub(crate) deck: SkeletonRootMatrix,
    pub(crate) wobble: WobbleAdjustment,
    pub(crate) airborne: bool,
    pub(crate) offboard: bool,
    pub(crate) board_contact: bool,
    pub(crate) teleporting: bool,
    pub(crate) wipeout_starting: bool,
}

#[derive(Debug)]
pub(crate) enum PostWipeoutPrefixError<N, D> {
    Missing(&'static str),
    Numeric(N),
    Transform(DynamicAssemblyPartTransformError),
    Wobble(WobbleUpdateError<D>),
}
impl<N: std::fmt::Debug, D: std::fmt::Debug> std::fmt::Display for PostWipeoutPrefixError<N, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "PostWipeout missing {v}"),
            Self::Numeric(e) => write!(f, "PostWipeout numeric: {e:?}"),
            Self::Transform(e) => write!(f, "PostWipeout deck: {e:?}"),
            Self::Wobble(e) => write!(f, "PostWipeout {e}"),
        }
    }
}

impl SkeletonNormalAssembly {
    /// Source input is the REAL `IsWipeoutRequested` result at82e01d68. Update4
    /// then reads dt93c at1d7c before calling1d94. dt is unused in this function
    /// but that caller read must occur; the prefix does not perform a second query.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume exclusive retained owner borrows"
    )]
    pub(crate) fn try_post_wipeout_prefix<N: ReckoningNumeric, D: WobbleUpdateDebug>(
        &mut self,
        owners: PostWipeoutPrefixOwners<'_>,
        wipeout_starting: bool,
        numeric: &mut N,
        debug: &mut D,
    ) -> Result<PostWipeoutBeforeRootAdjustment, PostWipeoutPrefixError<N::Error, D::Error>> {
        use PostWipeoutPrefixError as E;
        let p = owners.processed;
        if !wipeout_starting {
            let (flags, known) = p.known_flags_8c4();
            if known & 0x40000 == 0 {
                return Err(E::Missing("Processed8c4 bit40000"));
            }
            if flags & 0x40000 == 0 && self.general_update.add_skateboard_error() {
                // UseSkateboardError82b4d918 up BEFORE error82b4d920.
                let up = p
                    .prior_output()
                    .reckoning_up()
                    .ok_or(E::Missing("Processed220 up"))?
                    .value;
                let error = owners.outer.skateboard_error;
                let dot = numeric
                    .dot3(
                        [error.xyz[0], error.xyz[1], error.xyz[2], 0.0],
                        [up.xyz[0], up.xyz[1], up.xyz[2], 0.0],
                    )
                    .map_err(E::Numeric)?;
                let projection = up.map(|v| v * dot);
                // VMX min exceptional selection remains the existing host boundary.
                let negative_projection = 0.0_f32.min(dot);
                let horizontal = error.zip(projection, |a, b| a - b);
                let offset = up.zip(horizontal, |u, h| u.mul_add(negative_projection, h));
                self.physics_history
                    .add_post_wipeout_skateboard_error_typed(offset);
                self.general_update.clear_add_skateboard_error();
            }
        }
        let physics = p
            .physics_state_fields()
            .physics
            .ok_or(E::Missing("Processed8e4 physics"))?;
        let category = p
            .physics_state_fields()
            .category
            .ok_or(E::Missing("Processed8ec category"))?;
        let airborne = physics == PHYSICS_STATE_PHYSICS_AIR || physics == PHYSICS_STATE_KNOWN_AIR;
        let offboard = category == PHYSICS_CATEGORY_OFFBOARD;
        let contact = owners
            .board_construction
            .post_fields()
            .ok_or(E::Missing("board360 contact"))?
            .num_parts
            != 0;
        let teleporting = p
            .physics_state_fields()
            .current
            .ok_or(E::Missing("Processed8e8 current"))?
            == PHYSICS_STATE_TELEPORTING;
        let deck = owners
            .board
            .try_live_part_transform_typed(owners.backend, 6)
            .map_err(E::Transform)?;
        // Local output initialization has no live observer; actual callee writes
        // every output on success and retains prefix writes on debug failure.
        let mut wobble = WobbleAdjustment {
            do_adjust: false,
            tilt: 0.0,
            squish: 0.0,
        };
        owners
            .wobble
            .try_update_retained(&mut wobble, debug)
            .map_err(E::Wobble)?;
        owners.outer.deck_wobble_tilt = 0.0;
        owners.outer.deck_wobble_squish = 0.0;
        Ok(PostWipeoutBeforeRootAdjustment {
            deck,
            wobble,
            airborne,
            offboard,
            board_contact: contact,
            teleporting,
            wipeout_starting,
        })
    }
}

/// Mandatory continuation starts with the original foot/hand IK conditions.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent captured native branch flags"
)]
pub(crate) struct PostWipeoutBeforeIk {
    pub(crate) deck: SkeletonRootMatrix,
    pub(crate) airborne: bool,
    pub(crate) offboard: bool,
    pub(crate) board_contact: bool,
    pub(crate) teleporting: bool,
    pub(crate) wipeout_starting: bool,
}
#[derive(Debug)]
pub(crate) enum RootAdjustmentError<S> {
    SinCos(S),
    Record(
        crate::skeleton_physics_history::SkeletonPhysicsHistoryError<
            std::convert::Infallible,
            std::convert::Infallible,
        >,
    ),
}
impl<S: std::fmt::Debug> std::fmt::Display for RootAdjustmentError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SinCos(e) => write!(f, "root wobble SinCos: {e:?}"),
            Self::Record(e) => write!(f, "root wobble SetTransform: {e:?}"),
        }
    }
}
fn wobble_root<S: crate::reckoning_sin_cos::ReckoningSinCosNumeric>(
    record: SkeletonRootMatrix,
    adjustment: &WobbleAdjustment,
    rounding: &mut S,
) -> Result<SkeletonRootMatrix, S::Error> {
    let (sin, cos) = crate::reckoning_sin_cos::try_sin_cos(adjustment.tilt, rounding)?;
    let squish = adjustment.squish; // native a5c follows the SinCos call
    let rotation = SkeletonRootMatrix::known([
        cos, sin, 0.0, cos, -sin, cos, 0.0, -sin, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ]);
    let mut result = record.compose(&rotation);
    let p = result.columns()[1].zip(result.columns()[3], |y, p| y.mul_add(squish, p));
    result.set_column(3, p);
    Ok(result)
}
impl SkeletonNormalAssembly {
    /// Native82b579f4..57b94: root copy, `SinCos`, checked `SetTransform`, then
    /// retained tilt/squish and captured-deck replacement in that order.
    pub(crate) fn try_post_wipeout_root_adjustment<
        S: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
    >(
        &mut self,
        before: PostWipeoutBeforeRootAdjustment,
        outer: &mut SkeletonOuterRemainder,
        rounding: &mut S,
    ) -> Result<PostWipeoutBeforeIk, RootAdjustmentError<S::Error>> {
        apply_root_adjustment(
            &mut self.physics_history,
            &mut outer.deck_wobble_tilt,
            &mut outer.deck_wobble_squish,
            before,
            rounding,
        )
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "consume the preceding native continuation"
)]
fn apply_root_adjustment<S: crate::reckoning_sin_cos::ReckoningSinCosNumeric>(
    history: &mut crate::skeleton_physics_history::SkeletonPhysicsHistory,
    tilt: &mut f32,
    squish: &mut f32,
    before: PostWipeoutBeforeRootAdjustment,
    rounding: &mut S,
) -> Result<PostWipeoutBeforeIk, RootAdjustmentError<S::Error>> {
    let mut deck = before.deck;
    if before.wobble.do_adjust {
        let record = history.post_wipeout_record_transform(0);
        let adjusted =
            wobble_root(record, &before.wobble, rounding).map_err(RootAdjustmentError::SinCos)?;
        history
            .try_set_post_wipeout_record_transform(0, &adjusted)
            .map_err(RootAdjustmentError::Record)?;
        *tilt = before.wobble.tilt;
        *squish = before.wobble.squish;
        deck = adjusted;
    }
    Ok(PostWipeoutBeforeIk {
        deck,
        airborne: before.airborne,
        offboard: before.offboard,
        board_contact: before.board_contact,
        teleporting: before.teleporting,
        wipeout_starting: before.wipeout_starting,
    })
}

#[cfg(test)]
mod root_tests {
    use super::*;
    use crate::{
        reckoning_sin_cos::{HostReckoningSinCosNumeric, ReckoningSinCosNumeric},
        skeleton_physics_history::SkeletonPhysicsHistory,
        skeleton_root_transforms::SkeletonRootVector,
    };
    fn before(squish: f32) -> PostWipeoutBeforeRootAdjustment {
        PostWipeoutBeforeRootAdjustment {
            deck: SkeletonRootMatrix::translation(SkeletonRootVector::known([
                40.0, 50.0, 60.0, 0.0,
            ])),
            wobble: WobbleAdjustment {
                do_adjust: true,
                tilt: 0.3,
                squish,
            },
            airborne: true,
            offboard: false,
            board_contact: true,
            teleporting: false,
            wipeout_starting: false,
        }
    }
    fn history() -> SkeletonPhysicsHistory {
        let mut h = SkeletonPhysicsHistory::new([1.0; 24]);
        let mut m =
            SkeletonRootMatrix::translation(SkeletonRootVector::known([10.0, 20.0, 30.0, 0.0]));
        for axis in 0..4 {
            let mut v = m.columns()[axis];
            v.w = None;
            m.set_column(axis, v);
        }
        h.try_set_post_wipeout_record_transform(0, &m).unwrap();
        h
    }
    struct Failing;
    impl ReckoningSinCosNumeric for Failing {
        type Error = &'static str;
        fn round_nearest_even(&mut self, _: f32) -> Result<f32, Self::Error> {
            Err("rounding")
        }
    }
    #[test]
    fn active_root_rotates_squishes_and_replaces_only_root_and_captured_deck() {
        let mut h = history();
        let (mut tilt, mut squish) = (0.0, 0.0);
        let other = h.post_wipeout_record_transform(1);
        let result = apply_root_adjustment(
            &mut h,
            &mut tilt,
            &mut squish,
            before(2.0),
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
        let (sin, cos) = 0.3f64.sin_cos();
        for (actual, expected) in
            result.deck.columns()[3]
                .xyz
                .into_iter()
                .zip([10.0 - 2.0 * sin, 20.0 + 2.0 * cos, 30.0])
        {
            assert!((f64::from(actual) - expected).abs() < 0.00001);
        }
        assert!((f64::from(result.deck.columns()[0].xyz[0]) - cos).abs() < 0.00001);
        assert!((f64::from(result.deck.columns()[0].xyz[1]) - sin).abs() < 0.00001);
        assert_eq!(result.deck, h.post_wipeout_record_transform(0));
        assert!(result.deck.columns().iter().all(|c| c.w.is_none()));
        assert_eq!(h.post_wipeout_record_transform(1), other);
        assert_eq!(h.record().positions()[0].map(f32::to_bits), [0; 4]);
        assert_eq!(tilt.to_bits(), 0.3f32.to_bits());
        assert_eq!(squish.to_bits(), 2.0f32.to_bits());
    }
    #[test]
    fn numeric_and_nan_assertion_failures_preserve_root_and_prior_outer_stores() {
        let mut h = history();
        let original = h.post_wipeout_record_transform(0);
        let (mut tilt, mut squish) = (0.0, 0.0);
        assert!(matches!(
            apply_root_adjustment(&mut h, &mut tilt, &mut squish, before(2.0), &mut Failing),
            Err(RootAdjustmentError::SinCos("rounding"))
        ));
        assert_eq!(h.post_wipeout_record_transform(0), original);
        assert!(matches!(
            apply_root_adjustment(
                &mut h,
                &mut tilt,
                &mut squish,
                before(f32::NAN),
                &mut HostReckoningSinCosNumeric
            ),
            Err(RootAdjustmentError::Record(_))
        ));
        assert_eq!(h.post_wipeout_record_transform(0), original);
        assert_eq!(tilt.to_bits(), 0);
        assert_eq!(squish.to_bits(), 0);
    }
}
