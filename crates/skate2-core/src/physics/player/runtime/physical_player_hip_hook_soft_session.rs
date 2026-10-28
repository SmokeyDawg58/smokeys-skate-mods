//! Actual same-pair hip hooks used by selected `WipeoutGround.Enter`.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, hip_hook_soft::PairHipHookSoftError},
        skeleton_normal::drive_owner::hook_soft::HipHookSoftKind,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
};

#[derive(Debug)]
pub(crate) enum PhysicalHipHookSoftError<N> {
    MissingPool,
    Pair(PairHipHookSoftError<N>),
}
impl<N: std::fmt::Display> std::fmt::Display for PhysicalHipHookSoftError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("hip soft skater pool unavailable"),
            Self::Pair(e) => write!(f, "{e}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_pair_hip_hook_soft<N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        kind: HipHookSoftKind,
        strength: f32,
        estimate: &mut impl FnMut(f32) -> Result<f32, N>,
    ) -> Result<(), PhysicalHipHookSoftError<N>> {
        self.skaters
            .as_mut()
            .ok_or(PhysicalHipHookSoftError::MissingPool)?
            .try_pair_hip_hook_soft(&self.backend, board, skeleton, kind, strength, estimate)
            .map_err(PhysicalHipHookSoftError::Pair)
    }
}

#[cfg(test)]
pub(crate) fn check_original(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let before_bodies = world.backend.rigid_body_snapshot();
    let payloads = world.skeleton(skeleton).unwrap().hip_hooks_for_test();
    let before = [*payloads[0].read().unwrap(), *payloads[1].read().unwrap()];
    let mut estimates = Vec::new();
    let failed = world
        .try_pair_hip_hook_soft(
            board,
            skeleton,
            HipHookSoftKind::Linear,
            0.0,
            &mut |spring| {
                estimates.push(spring);
                Err::<f32, _>("estimate discriminator")
            },
        )
        .unwrap_err();
    assert!(failed.to_string().contains("estimate discriminator"));
    assert_eq!(*payloads[0].read().unwrap(), before[0]);
    assert_eq!(estimates.len(), 1);
    world
        .try_pair_hip_hook_soft(
            board,
            skeleton,
            HipHookSoftKind::Angular,
            0.5,
            &mut |_| -> Result<f32, std::convert::Infallible> {
                panic!("angular hook has no estimate")
            },
        )
        .unwrap();
    let angular = payloads[0].read().unwrap().angular;
    for scalar in [1.0, 0.0] {
        world
            .try_pair_hip_hook_soft(
                board,
                skeleton,
                HipHookSoftKind::Linear,
                scalar,
                &mut |spring| {
                    assert_eq!(spring.to_bits(), estimates[0].to_bits());
                    Ok::<f32, std::convert::Infallible>(spring.sqrt().recip())
                },
            )
            .unwrap();
        let current = *payloads[0].read().unwrap();
        assert_eq!(current.angular, angular);
        assert_eq!(current.linear.raw_type, 1);
        if scalar.to_bits() == 1.0_f32.to_bits() {
            estimates.push(current.linear.damping);
        } else {
            assert_eq!(current.linear.spring.to_bits(), 0.0_f32.to_bits());
            assert_eq!(current.linear.strength.to_bits(), 0.0_f32.to_bits());
            assert_eq!(current.linear.damping.to_bits(), estimates[1].to_bits());
        }
    }
    let same = world.skeleton(skeleton).unwrap().hip_hooks_for_test();
    for index in 0..2 {
        assert!(std::sync::Arc::ptr_eq(&same[index], &payloads[index]));
    }
    assert_eq!(*payloads[1].read().unwrap(), before[1]);
    assert_eq!(world.backend.rigid_body_snapshot(), before_bodies);
}
