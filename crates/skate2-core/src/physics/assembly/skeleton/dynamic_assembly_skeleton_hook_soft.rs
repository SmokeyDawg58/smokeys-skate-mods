//! Same retained hip-hook dynamics: angular82b47e70, linear82b48000.
use super::{SkeletonDriveOwnerError, SkeletonDriveOwnerStage, SkeletonNormalAssembly};
use crate::{
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_body_attributes::{SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute},
};

const SPRING: u64 = 0x7cda_0800_cbb4_2d9b;
const STRENGTH: u64 = 0x0121_f19d_ba38_eb13;
const ANGULAR_DAMPING: u64 = 0xbc2b_b9a0_9633_5327;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HipHookSoftKind {
    Angular,
    Linear,
}

#[derive(Debug)]
pub(crate) enum HipHookSoftError<A, N> {
    Owner(SkeletonDriveOwnerError),
    Attribute(A),
    Estimate(N),
    Poisoned,
}

impl<A: std::fmt::Display, N: std::fmt::Display> std::fmt::Display for HipHookSoftError<A, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "hip soft hook owner: {e}"),
            Self::Attribute(e) => write!(f, "hip soft hook attribute: {e}"),
            Self::Estimate(e) => write!(f, "hip soft hook estimate: {e}"),
            Self::Poisoned => f.write_str("hip soft hook dynamics lock poisoned"),
        }
    }
}

impl SkeletonNormalAssembly {
    #[cfg(test)]
    pub(crate) fn hip_hooks_for_test(
        &self,
    ) -> [std::sync::Arc<std::sync::RwLock<crate::simulation_drive_pool::SimulationDriveDynamics>>; 2]
    {
        let runtime = self.drives.as_ref().unwrap();
        std::array::from_fn(|index| std::sync::Arc::clone(runtime.prepared.hook_dynamics(index)))
    }

    /// Caller passes the SAME construction body's retained global-animation
    /// reader. The raw hip-hook payload is sampled later by actual drive build;
    /// changing these scalars does not itself refresh body or collision caches.
    pub(crate) fn try_set_hip_hook_soft<A: SkeletonBodyAttributeRead, N>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        attributes: &mut A,
        kind: HipHookSoftKind,
        strength: f32,
        estimate: &mut impl FnMut(f32) -> Result<f32, N>,
    ) -> Result<(), HipHookSoftError<A::Error, N>> {
        use HipHookSoftError as E;
        self.check_drive_backend(backend).map_err(E::Owner)?;
        let runtime = self
            .drives
            .as_mut()
            .ok_or(E::Owner(SkeletonDriveOwnerError::Missing))?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete {
            return Err(E::Owner(SkeletonDriveOwnerError::Incomplete));
        }
        // Both native fsel operations preserve the unordered-input selection.
        let low = if -strength >= 0.0 { 0.0 } else { strength };
        let scalar = if 1.0 - low >= 0.0 { low } else { 1.0 };
        let spring = attributes
            .float(SkeletonBodyAttributeRoute::GlobalAnimation, SPRING)
            .map_err(E::Attribute)?;
        let force = attributes
            .float(SkeletonBodyAttributeRoute::GlobalAnimation, STRENGTH)
            .map_err(E::Attribute)?;
        let force = force * scalar;
        match kind {
            HipHookSoftKind::Angular => {
                let damping = attributes
                    .float(SkeletonBodyAttributeRoute::GlobalAnimation, ANGULAR_DAMPING)
                    .map_err(E::Attribute)?;
                let mut payload = runtime
                    .prepared
                    .hook_dynamics(0)
                    .write()
                    .map_err(|_| E::Poisoned)?;
                payload.angular.spring = spring * scalar;
                payload.angular.damping = damping;
                payload.angular.raw_type = 1;
                payload.angular.strength = force * f32::from_bits(0x4560_fffe);
            }
            HipHookSoftKind::Linear => {
                let damping_subtraction = spring * f32::from_bits(0x3c83_126f);
                // Source estimate8144 precedes every dynamics store, even for0.
                let initial = estimate(spring).map_err(E::Estimate)?;
                let mut payload = runtime
                    .prepared
                    .hook_dynamics(0)
                    .write()
                    .map_err(|_| E::Poisoned)?;
                payload.linear.spring = spring * scalar;
                payload.linear.raw_type = 1;
                payload.linear.strength = force * f32::from_bits(0x4560_fffe);
                payload.linear.damping = linear_damping(spring, initial, damping_subtraction);
            }
        }
        Ok(())
    }
}

fn linear_damping(spring: f32, initial: f32, subtraction: f32) -> f32 {
    let square = initial * initial;
    let half = initial * 0.5;
    let residual = refinement_residual(spring, square);
    let once = half.mul_add(residual, initial);
    let square = once * once;
    let half = once * 0.5;
    let residual = refinement_residual(spring, square);
    let twice = half.mul_add(residual, once);
    let root = spring * twice;
    let root = if spring == 0.0 { 0.0 } else { root };
    (root * 2.0) - subtraction
}

/// VNMSUB81a4/81c0 rounds the product-minus-one before negating its result.
fn refinement_residual(spring: f32, square: f32) -> f32 {
    -spring.mul_add(square, -1.0)
}

#[cfg(test)]
mod tests {
    use super::refinement_residual;

    #[test]
    fn exact_cancellation_preserves_the_postnegated_residual_zero() {
        // The two algebraic expressions differ at this native intermediate.
        // Subsequent positive-estimate refinement can hide that sign difference.
        assert_eq!(
            refinement_residual(1.0, 1.0).to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!((-1.0_f32).mul_add(1.0, 1.0).to_bits(), 0.0_f32.to_bits());
    }
}
