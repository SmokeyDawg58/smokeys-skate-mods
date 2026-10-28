//! Actual `Update1_ProcessInput82e03504..36c4`, before `PrepareToolkitInput`.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUNDTRICK, PHYSICS_CATEGORY_OFFBOARD,
};
use super::{
    physical_ground_position::PhysicalPlayerGroundPositionError,
    physical_update1_prefix::Update1AfterTeleportCheck,
};
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_normal::effective_animation_transform::SkeletonEffectiveAnimationUnknownMirror,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, hilod_prefix::PhysicalHiLodPrefix},
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug)]
pub(crate) enum ProcessInputGroundError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignPair,
    Ground(PhysicalPlayerGroundPositionError),
    Effective(SkeletonEffectiveAnimationUnknownMirror),
    Numeric(N),
}

impl<N: std::fmt::Debug> std::fmt::Display for ProcessInputGroundError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "physical ground input missing {field}"),
            Self::Poisoned => f.write_str("physical ground input owner lock poisoned"),
            Self::ForeignPair => {
                f.write_str("physical ground input continuation belongs to another pair")
            }
            Self::Ground(error) => write!(f, "physical ground position: {error}"),
            Self::Effective(error) => write!(f, "physical effective transform: {error}"),
            Self::Numeric(error) => {
                write!(f, "physical ground damping numeric boundary: {error:?}")
            }
        }
    }
}
fn reciprocal_residual(input: f32, reciprocal: f32) -> f32 {
    // VNMSUB3664/367c negates the fused result, including exact-zero sign.
    -reciprocal.mul_add(input, -1.0)
}
fn refine_ground_reciprocal(input: f32, seed: f32) -> f32 {
    let first = seed.mul_add(reciprocal_residual(input, seed), seed);
    first.mul_add(reciprocal_residual(input, first), first)
}
/// Called after source3608's ground store; estimate precedes delta store366c.
fn update_ground_history<N: ReckoningNumeric>(
    prefix: &mut PhysicalHiLodPrefix,
    old: SkeletonRootVector,
    dt: f32,
    frames_since_teleport: i32,
    numeric: &mut N,
) -> Result<(), N::Error> {
    if frames_since_teleport > 1 {
        let seed = numeric.reciprocal_estimate(dt)?;
        let current = prefix.ground_position_all_states;
        let delta = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| current.xyz[lane] - old.xyz[lane]),
            w: current.w.zip(old.w).map(|(new, old)| new - old),
        };
        let damping = f32::from_bits(0x3f73_3333);
        let complement = 1.0 - damping;
        prefix.ground_position_delta = delta; //366c
        let reciprocal = refine_ground_reciprocal(dt, seed);
        let prior_velocity = prefix.ground_velocity_damped;
        let damp = |delta: f32, old: f32| {
            let weighted = delta * complement;
            let velocity = reciprocal * weighted;
            old.mul_add(damping, velocity)
        };
        prefix.ground_velocity_damped = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| damp(delta.xyz[lane], prior_velocity.xyz[lane])),
            w: delta
                .w
                .zip(prior_velocity.w)
                .map(|(delta, old)| damp(delta, old)),
        }; //368c
    }
    Ok(())
}
impl PhysicalAssemblyWorld {
    /// Retains all prior native stores on failure; no completed Reckoning token
    /// is fabricated for constructor-initialized data read by the first frame.
    pub(crate) fn try_continue_player_update1_ground<N: ReckoningNumeric>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        token: &Update1AfterTeleportCheck,
        numeric: &mut N,
    ) -> Result<(), ProcessInputGroundError<N::Error>> {
        use ProcessInputGroundError as E;
        for handle in [board, skeleton] {
            let actual = self
                .physical_body_owner_for_contacts(handle)
                .map_err(|_| E::Missing("registered physical pair"))?;
            if !std::sync::Arc::ptr_eq(actual, &token.owner) {
                return Err(E::ForeignPair);
            }
        }
        let mut prefix_owner = token.owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix_owner.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        let mut processed = token.processed.lock().map_err(|_| E::Poisoned)?;
        if token.prior_category == PHYSICS_CATEGORY_OFFBOARD {
            let position = *self
                .skeleton(skeleton)
                .map_err(|_| E::Missing("Skeleton"))?
                .root_transforms()
                .animation_to_world()
                .columns()
                .last()
                .unwrap();
            prefix.ground_position_all_states = position; //3538
            prefix.time_offboard += f32::from_bits(0x3c88_8889); //3544
        } else {
            let mut position = SkeletonRootVector::known([0.0; 4]);
            self.try_calculate_current_board_ground_position(board, &mut position)
                .map_err(E::Ground)?;
            prefix.ground_position_all_states = position; //356c
            prefix.time_offboard = 0.0;
        }
        processed.ground_position_fields_mut().time_offboard = Some(prefix.time_offboard); //3580
        let effective = self
            .skeleton(skeleton)
            .map_err(|_| E::Missing("Skeleton"))?
            .try_effective_animation_to_world(&processed)
            .map_err(E::Effective)?;
        processed.ground_position_fields_mut().effective_animation = Some(effective); //35b0..c8
        if token.prior_category != PHYSICS_CATEGORY_AIR && token.prior_category != PHYSICS_CATEGORY_GROUNDTRICK {
            let frames = token
                .owner
                .physics_state()
                .lock()
                .map_err(|_| E::Poisoned)?
                .frames_since_jump
                .ok_or(E::Missing("HiLOD frames since jump"))?;
            if frames > 0 {
                let dt = processed
                    .physics_state_fields()
                    .time_delta
                    .ok_or(E::Missing("Processed dt"))?;
                // The ground store3608 precedes the potentially unknown frame count3624.
                let old = prefix.ground_position;
                prefix.ground_position = prefix.ground_position_all_states;
                let frames = processed
                    .input_controls_mut()
                    .frames_since_teleport
                    .ok_or(E::Missing("Processed frames since teleport"))?;
                update_ground_history(prefix, old, dt, frames, numeric).map_err(E::Numeric)?;
            }
        }
        processed.ground_position_fields_mut().ground_position = Some(prefix.ground_position); //36ac
        processed.publish_ground_position_all_states(prefix.ground_position_all_states); //36b8
        processed
            .ground_position_fields_mut()
            .damped_ground_velocity = Some(prefix.ground_velocity_damped); //36c4
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bits(value: SkeletonRootVector) -> [u32; 4] {
        [
            value.xyz[0],
            value.xyz[1],
            value.xyz[2],
            value.w.expect("known test lane"),
        ]
        .map(f32::to_bits)
    }
    #[test]
    fn reciprocal_residual_negates_fused_zero_and_retains_finite_fma_precision() {
        assert_eq!(reciprocal_residual(2.0, 0.5).to_bits(), 0x8000_0000);
        // Algebraic negation of the multiplier has the opposite exact-zero sign.
        assert_eq!((-0.5_f32).mul_add(2.0, 1.0).to_bits(), 0);
        let input = f32::from_bits(0x72bb_3b18);
        let seed = f32::from_bits(0x0c2f_038f);
        assert_eq!(refine_ground_reciprocal(input, seed).to_bits(), 0x0c2f_038f);
        let first = seed * (1.0 - seed * input) + seed;
        let defused = first * (1.0 - first * input) + first;
        assert_eq!(defused.to_bits(), 0x0c2f_0390);
    }
    struct Numeric {
        calls: usize,
        fail: bool,
    }
    impl ReckoningNumeric for Numeric {
        type Error = &'static str;
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            panic!("no dot product in damping")
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("no reciprocal sqrt in damping")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("no conversion in damping")
        }
        fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.calls += 1;
            if self.fail {
                Err("estimate")
            } else {
                Ok(1.0 / value)
            }
        }
    }
    #[test]
    fn recent_teleport_preserves_delta_and_velocity_without_numeric_sampling() {
        let mut prefix = PhysicalHiLodPrefix::construct_in_sim_allocation();
        prefix.ground_position_all_states = SkeletonRootVector::known([8.0; 4]);
        prefix.ground_position_delta = SkeletonRootVector::known([9.0; 4]);
        prefix.ground_velocity_damped = SkeletonRootVector::known([7.0; 4]);
        let mut numeric = Numeric {
            calls: 0,
            fail: true,
        };
        update_ground_history(&mut prefix, [0.0; 4].into(), 0.0, 1, &mut numeric).unwrap();
        assert_eq!(numeric.calls, 0);
        assert_eq!(bits(prefix.ground_position_delta), [9.0_f32.to_bits(); 4]);
        assert_eq!(bits(prefix.ground_velocity_damped), [7.0_f32.to_bits(); 4]);
        assert_eq!(
            update_ground_history(&mut prefix, [0.0; 4].into(), 0.0, 2, &mut numeric),
            Err("estimate")
        );
        assert_eq!(numeric.calls, 1);
        assert_eq!(bits(prefix.ground_position_delta), [9.0_f32.to_bits(); 4]);
    }
    #[test]
    fn damping_updates_four_lanes_with_weight_before_reciprocal_product() {
        let mut prefix = PhysicalHiLodPrefix::construct_in_sim_allocation();
        prefix.ground_position_all_states = SkeletonRootVector::known([5.0, 8.0, 11.0, 14.0]);
        prefix.ground_velocity_damped = SkeletonRootVector::known([-1.0, 2.0, -3.0, 4.0]);
        let mut numeric = Numeric {
            calls: 0,
            fail: false,
        };
        update_ground_history(
            &mut prefix,
            [1.0, 2.0, 3.0, 4.0].into(),
            2.0,
            2,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(numeric.calls, 1);
        assert_eq!(
            bits(prefix.ground_position_delta),
            [4.0_f32, 6.0, 8.0, 10.0].map(f32::to_bits)
        );
        let expected = std::array::from_fn::<_, 4, _>(|i| {
            let delta = [4.0_f32, 6.0, 8.0, 10.0][i];
            let coefficient = f32::from_bits(0x3f73_3333);
            coefficient
                .mul_add(
                    [-1.0, 2.0, -3.0, 4.0][i],
                    (delta * (1.0 - coefficient)) * 0.5,
                )
                .to_bits()
        });
        assert_eq!(bits(prefix.ground_velocity_damped), expected);
    }
    #[test]
    fn unknown_w_does_not_block_xyz_damping_or_become_known() {
        let mut known = PhysicalHiLodPrefix::construct_in_sim_allocation();
        known.ground_position_all_states = [5.0, 8.0, 11.0, 14.0].into();
        known.ground_velocity_damped = [-1.0, 2.0, -3.0, 4.0].into();
        let mut unknown = PhysicalHiLodPrefix::construct_in_sim_allocation();
        unknown.ground_position_all_states = SkeletonRootVector {
            xyz: [5.0, 8.0, 11.0],
            w: None,
        };
        unknown.ground_velocity_damped = known.ground_velocity_damped;
        let mut numeric = Numeric {
            calls: 0,
            fail: false,
        };
        let old = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
        update_ground_history(&mut known, old, 2.0, 2, &mut numeric).unwrap();
        update_ground_history(&mut unknown, old, 2.0, 2, &mut numeric).unwrap();
        assert_eq!(numeric.calls, 2);
        assert_eq!(
            unknown.ground_position_delta.xyz.map(f32::to_bits),
            known.ground_position_delta.xyz.map(f32::to_bits)
        );
        assert_eq!(
            unknown.ground_velocity_damped.xyz.map(f32::to_bits),
            known.ground_velocity_damped.xyz.map(f32::to_bits)
        );
        assert_eq!(unknown.ground_position_delta.w, None);
        assert_eq!(unknown.ground_velocity_damped.w, None);
        // A later known position cannot restore an unknown damped-history lane.
        unknown.ground_position_all_states = known.ground_position_all_states;
        update_ground_history(&mut unknown, old, 2.0, 2, &mut numeric).unwrap();
        assert!(unknown.ground_position_delta.w.is_some());
        assert_eq!(unknown.ground_velocity_damped.w, None);
    }
}
