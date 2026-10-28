//! SAME board hook `EnableAngularDriveOnly`82b2aca8 / soft angular82b24ad8.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SkateboardEntrySessionError,
};
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    simulation_drive_pool::SimulationDriveDynamics,
    skateboard_assembly::SkateboardSimulationBackend,
};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub(crate) enum AirDriveError<N> {
    Entry(SkateboardEntrySessionError),
    Cvar(InputCvarError),
    Numeric(N),
    Poisoned,
}
impl<N: std::fmt::Debug> std::fmt::Display for AirDriveError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entry(e) => write!(f, "Air drive owner: {e}"),
            Self::Cvar(e) => write!(f, "Air drive cvar: {e:?}"),
            Self::Numeric(e) => write!(f, "Air drive numeric: {e:?}"),
            Self::Poisoned => f.write_str("Air drive dynamics lock poisoned"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    fn try_retained_board_hook(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<Arc<RwLock<SimulationDriveDynamics>>, SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let runtime = candidate
            .force_runtime
            .as_ref()
            .ok_or(E::MissingConstruction)?;
        runtime
            .hook_dynamics
            .as_ref()
            .map(Arc::clone)
            .ok_or(E::MissingHook)
    }

    /// `HookDisableLinearDrive` 0x82b24cb8 writes HARD with zero payload.
    pub(crate) fn try_disable_board_linear_drive(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), AirDriveError<std::convert::Infallible>> {
        let hook = self
            .try_retained_board_hook(backend, board)
            .map_err(AirDriveError::Entry)?;
        let mut dynamics = hook.write().map_err(|_| AirDriveError::Poisoned)?;
        write_board_linear_disabled(&mut dynamics);
        Ok(())
    }

    /// `HookEnableLinearDriveSoft` 0x82b24bb8, on the retained Body hook.
    /// Hold's soft animation calls this before the angular leaf.
    pub(crate) fn try_enable_board_linear_soft_drive<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        strength: f32,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AirDriveError<N::Error>> {
        let hook = self
            .try_retained_board_hook(backend, board)
            .map_err(AirDriveError::Entry)?;
        write_board_linear_soft(&hook, strength, refs, numeric)
    }

    /// `HookEnableAngularDriveSoft` 0x82b24ad8 without the distinct
    /// `EnableAngularDriveOnly` caller's trailing linear-hard-zero stores.
    pub(crate) fn try_enable_board_angular_soft_drive<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        strength: f32,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AirDriveError<N::Error>> {
        let hook = self
            .try_retained_board_hook(backend, board)
            .map_err(AirDriveError::Entry)?;
        write_board_angular_soft(&hook, strength, refs, numeric)
    }

    pub(crate) fn try_enable_board_angular_drive_only<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        strength: f32,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AirDriveError<N::Error>> {
        use AirDriveError as E;
        let hook = self
            .try_retained_board_hook(backend, board)
            .map_err(E::Entry)?;
        write_board_angular_soft(&hook, strength, refs, numeric)?;
        // Caller reacquires the same Body and writes hard linear zero payload.
        let mut dynamics = hook.write().map_err(|_| E::Poisoned)?;
        write_board_linear_disabled(&mut dynamics);
        Ok(())
    }
}

pub(super) fn write_board_linear_soft<N: ReckoningNumeric>(
    hook: &Arc<RwLock<SimulationDriveDynamics>>,
    strength: f32,
    refs: &InputCvarReferences,
    numeric: &mut N,
) -> Result<(), AirDriveError<N::Error>> {
    use AirDriveError as E;
    let spring = refs
        .try_skateboard_linear_soft_drive_disp()
        .map_err(E::Cvar)?
        * strength;
    // The native estimate is between the two distinct linked-target reads.
    let seed = numeric
        .reciprocal_sqrt_estimate(spring)
        .map_err(E::Numeric)?;
    let force = refs
        .try_skateboard_linear_soft_drive_strength()
        .map_err(E::Cvar)?;
    let mut dynamics = hook.write().map_err(|_| E::Poisoned)?;
    dynamics.linear.raw_type = 1;
    dynamics.linear.strength = force * f32::from_bits(0x4560_fffe);
    dynamics.linear.spring = spring;
    dynamics.linear.damping = soft_drive_damping(spring, seed);
    Ok(())
}

pub(super) fn write_board_angular_soft<N: ReckoningNumeric>(
    hook: &Arc<RwLock<SimulationDriveDynamics>>,
    strength: f32,
    refs: &InputCvarReferences,
    numeric: &mut N,
) -> Result<(), AirDriveError<N::Error>> {
    use AirDriveError as E;
    let spring = refs.try_skateboard_soft_drive_disp().map_err(E::Cvar)? * strength;
    let seed = numeric
        .reciprocal_sqrt_estimate(spring)
        .map_err(E::Numeric)?;
    let force = refs.try_skateboard_soft_drive_strength().map_err(E::Cvar)?;
    let mut dynamics = hook.write().map_err(|_| E::Poisoned)?;
    dynamics.angular.raw_type = 1;
    dynamics.angular.strength = force * f32::from_bits(0x4560_fffe);
    dynamics.angular.spring = spring;
    dynamics.angular.damping = soft_drive_damping(spring, seed);
    Ok(())
}

fn soft_drive_damping(spring: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let half = seed * 0.5;
    let residual = -spring.mul_add(square, -1.0);
    let once = half.mul_add(residual, seed);
    let square = once * once;
    let half = once * 0.5;
    let residual = -spring.mul_add(square, -1.0);
    let twice = half.mul_add(residual, once);
    let root = spring * twice;
    (if spring == 0.0 { 0.0 } else { root }) * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        input_cvar_runtime::InputCvarManager, reckoning_numeric::HostReckoningNumeric,
        simulation_drive_pool::SimulationDriveDynamicsParams,
    };

    #[test]
    fn linear_soft_leaf_reads_linked_values_and_preserves_angular_drive() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkSkateboardLinSoftDriveDisp=7\r\nkSkateboardLinSoftDriveStr=3\r\n",
            ))
            .unwrap();
        let angular = SimulationDriveDynamicsParams {
            spring: 9.0,
            damping: 8.0,
            strength: 7.0,
            raw_type: 2,
        };
        let hook = Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: SimulationDriveDynamicsParams {
                spring: 0.0,
                damping: 0.0,
                strength: 0.0,
                raw_type: 0,
            },
            angular,
        }));
        write_board_linear_soft(&hook, 0.5, &refs, &mut HostReckoningNumeric).unwrap();
        let dynamics = hook.read().unwrap();
        assert_eq!(dynamics.linear.raw_type, 1);
        assert_eq!(dynamics.linear.spring.to_bits(), 3.5f32.to_bits());
        assert_eq!(
            dynamics.linear.strength.to_bits(),
            (3.0 * f32::from_bits(0x4560_fffe)).to_bits()
        );
        assert_eq!(
            dynamics.linear.damping.to_bits(),
            soft_drive_damping(3.5, 1.0 / 3.5f32.sqrt()).to_bits()
        );
        assert_eq!(dynamics.angular, angular);
    }
}

/// Shared source hard-zero leaf, leaving angular dynamics untouched.
fn write_board_linear_disabled(dynamics: &mut SimulationDriveDynamics) {
    dynamics.linear.raw_type = 2;
    dynamics.linear.spring = 0.0;
    dynamics.linear.damping = 0.0;
    dynamics.linear.strength = 0.0;
}

#[cfg(test)]
#[test]
fn disable_linear_is_hard_zero_and_preserves_angular() {
    use crate::simulation_drive_pool::SimulationDriveDynamicsParams;
    let params = SimulationDriveDynamicsParams {
        raw_type: 1,
        spring: 3.0,
        damping: 4.0,
        strength: 5.0,
    };
    let mut dynamics = SimulationDriveDynamics {
        linear: params,
        angular: params,
    };
    write_board_linear_disabled(&mut dynamics);
    assert_eq!(dynamics.angular, params);
    assert_eq!(dynamics.linear.raw_type, 2);
    assert_eq!(
        [
            dynamics.linear.spring,
            dynamics.linear.damping,
            dynamics.linear.strength
        ]
        .map(f32::to_bits),
        [0; 3]
    );
}
