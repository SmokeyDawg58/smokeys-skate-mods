//! Draft child of `SkeletonNormalAssembly`: actual post-physics preparation.
//! Registration requires the narrowly documented typed outer-field amendments.
use super::{SkeletonNormalAssembly, reset_owner::SkeletonResetError};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        part_transform::DynamicAssemblyPartTransformError,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skeleton_assembly_construction::constructor::SkeletonOuterRemainder,
    skeleton_body_attributes::{
        SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute, SkeletonBodyAttributes,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(crate) trait PostPhysicsPreparationCvars {
    type Error;
    fn draw_skateboard_error(&mut self) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PreparationError<C, N> {
    Attribute(AttributeRuntimeError),
    Body(DynamicAssemblyPartContactSpyError),
    Motion(SimulationRigidBodyPoolError),
    Transform(DynamicAssemblyPartTransformError),
    Numeric(N),
    Reset(SkeletonResetError),
    MissingIk,
    Cvar(C),
}
impl<C: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for PreparationError<C, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => e.fmt(f),
            Self::Body(e) => e.fmt(f),
            Self::Motion(e) => e.fmt(f),
            Self::Transform(e) => e.fmt(f),
            Self::Numeric(e) => write!(f, "Skeleton preparation numeric: {e:?}"),
            Self::Reset(e) => e.fmt(f),
            Self::MissingIk => f.write_str("Skeleton preparation requires actual IK"),
            Self::Cvar(e) => write!(f, "Skeleton preparation cvar: {e:?}"),
        }
    }
}

/// Standard VMADD82b4ccbc/ccc0/ccc4 and ccdc/cce0/cce4. Input W is not a coefficient.
/// Each result W requires all four actual matrix W lanes, even for zero XYZ.
pub(crate) fn transform_point(
    matrix: &SkeletonRootMatrix,
    position: [f32; 3],
) -> SkeletonRootVector {
    let [i, j, k, p] = *matrix.columns();
    let first = transform_point_stage(p, i, position[0]);
    let second = transform_point_stage(first, j, position[1]);
    transform_point_stage(second, k, position[2])
}

/// One VMADD stage, shared with OffBoard's interleaved COM/length graph.
pub(crate) fn transform_point_stage(
    prior: SkeletonRootVector,
    axis: SkeletonRootVector,
    coefficient: f32,
) -> SkeletonRootVector {
    axis.zip(prior, |axis, prior| axis.mul_add(coefficient, prior))
}

impl SkeletonNormalAssembly {
    /// `IsBadData82b472e8` reads the actual body-local skeleton Instance first.
    /// Speed threshold applies to squared Part23 linear speed (no square root).
    /// Each subsequent Part reads velocity, angular velocity, then `GetTransform`.
    /// Only XYZ NaN is bad; infinity and packed unknown W do not fail checks.
    pub(crate) fn try_postphysics_bad_data<C, N: ReckoningNumeric>(
        &self,
        backend: &SkateboardSimulationBackend,
        attributes: &mut SkeletonBodyAttributes,
        numeric: &mut N,
    ) -> Result<bool, PreparationError<C, N::Error>> {
        use PreparationError as E;
        let threshold = attributes
            .float(
                SkeletonBodyAttributeRoute::LocalSkeleton,
                0xd2d7_4e3f_daf8_ec6d,
            )
            .map_err(E::Attribute)?;
        let body = self
            .session
            .try_part_contact_spy_observation(backend, 23)
            .map_err(E::Body)?;
        let velocity = backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .map_err(E::Motion)?;
        // Dot3 ignores the adjacent packed W; scratch is never published.
        let v = [velocity[0], velocity[1], velocity[2], 0.0];
        if numeric.dot3(v, v).map_err(E::Numeric)? > threshold {
            return Ok(true);
        }
        for part in 0..24 {
            let body = self
                .session
                .try_part_contact_spy_observation(backend, part)
                .map_err(E::Body)?;
            let velocity = backend
                .collision_body_pool()
                .try_contact_spy_linear_velocity(body)
                .map_err(E::Motion)?;
            if velocity.iter().any(|v| v.is_nan()) {
                return Ok(true);
            }
            // Native reloads this Part's body pointer before its angular read.
            let body = self
                .session
                .try_part_contact_spy_observation(backend, part)
                .map_err(E::Body)?;
            let angular = backend
                .collision_body_pool()
                .try_phys_out_angular_velocity(body)
                .map_err(E::Motion)?;
            if angular.iter().any(|v| v.is_nan()) {
                return Ok(true);
            }
            let transform = self
                .session
                .try_live_part_transform_typed(backend, part)
                .map_err(E::Transform)?;
            if transform.has_nan_xyz() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Actual575fc bad-data branch, then fresh board Part6 read57684.
    /// Narrow required general-owner amendment: unconditional respawn store,
    /// distinct from the state-gated failed-hook reset helper.
    pub(crate) fn try_prepare_postphysics<C: PostPhysicsPreparationCvars, N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SkateboardAssemblySession,
        outer: &mut SkeletonOuterRemainder,
        attributes: &mut SkeletonBodyAttributes,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<(), PreparationError<C::Error, N::Error>> {
        use PreparationError as E;
        if self.try_postphysics_bad_data(backend, attributes, numeric)? {
            self.try_reset_skeleton(backend).map_err(E::Reset)?;
            self.current_animation
                .try_finish_skeleton_init()
                .map_err(|_| E::MissingIk)?;
            self.general_update.mark_want_respawn_after_bad_data();
        }
        let deck = board
            .try_live_part_transform_typed(backend, 6)
            .map_err(E::Transform)?;
        let prediction = self.root_transforms.predicted_board_position();
        outer.skateboard_error = deck.columns()[3].zip(prediction, |position, old| position - old);
        // Native576d0's enabled target825187c0 is BLR; no overlay read.
        // The live cvar read itself remains mandatory even though both paths
        // have identical observable state after that proven empty callee.
        cvars.draw_skateboard_error().map_err(E::Cvar)?;
        Ok(())
    }

    /// Invoke immediately after the existing `UpdatePhysicsRecords` prefix,
    /// supplying its captured cc4c board Part6 P, never a fresh getter.
    /// OLD retained COM remains untouched until later `UpdateRecords5789c`.
    pub(crate) fn finish_postphysics_history_local(
        &self,
        outer: &mut SkeletonOuterRemainder,
        captured_deck_position: SkeletonRootVector,
    ) {
        let old_com = self.post_physics_records.centre_of_mass();
        outer.centre_of_mass_local = transform_point(
            self.root_transforms.world_to_animation(),
            [old_com[0], old_com[1], old_com[2]],
        );
        // Native reloads the matrix columns after publishing COM local.
        outer.deck_local = transform_point(
            self.root_transforms.world_to_animation(),
            captured_deck_position.xyz,
        );
    }
}

#[cfg(test)]
#[path = "skeleton_postphysics_preparation_live_tests.rs"]
mod tests;
