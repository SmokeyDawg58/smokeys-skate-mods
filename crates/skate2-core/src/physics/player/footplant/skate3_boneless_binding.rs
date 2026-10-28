//! Source S3 boneless/fastplant602 calculation on the actual S2 state and bodies.
use super::{E, processed, pushing_right_foot, service, state};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_footplant::skate3::SourceFootplant,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::{Arc, Mutex};

fn lanes(v: V) -> [f32; 4] {
    // Adapter-owned geometric input. Unknown native W knowledge stays unknown
    // in its original owner; source skating calculations use XYZ geometry.
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.)]
}

impl PhysicalAssemblyWorld {
    pub(super) fn try_enter_source_boneless(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        let mut source = SourceFootplant::new_boneless()
            .map_err(|e| service("embedded S3 boneless recipe", e))?;
        self.try_enable_source_plant_drive(board, true)
            .map_err(|e| service("S3 boneless board drive", e))?;
        let processed = processed(owner)?;
        let right = pushing_right_foot(&processed)?;
        let toe = if right { 19 } else { 15 };
        let transform = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_query_part_transform(toe)
            })
            .map_err(|e| service("S3 boneless Skeleton pair", e))?
            .map_err(|e| service("S3 boneless physical toe", e))?;
        source.begin(right, lanes(transform.columns()[3]));
        crate::diagnostics::test_log::event(
            "skate3_boneless_start",
            format_args!(
                "toe={} anchor={:?} native_s2_solver=true",
                source.toe, source.anchor
            ),
        );
        state(owner, board, skeleton, |state| {
            state.source3 = Some(Box::new(source));
            Ok(())
        })
    }
}

/// No S2 launch attributes/cvars are consulted when this actual state owns the
/// source replacement. The caller still launches its existing native selector.
pub(super) fn launch(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    p: &Mutex<SkeletonProcessedContact>,
) -> Result<Option<(V, V)>, E> {
    if !state(owner, board, skeleton, |state| Ok(state.source3.is_some()))? {
        return Ok(None);
    }
    let input = {
        let p = p.lock().map_err(|_| E::Poisoned)?;
        let prior = p.prior_output();
        crate::skate3_native::physics::boneless::LaunchInput {
            up: lanes(
                prior
                    .reckoning_up()
                    .ok_or(E::Missing("boneless up+220"))?
                    .value,
            ),
            current_velocity: lanes(
                prior
                    .centre_of_mass_velocity()
                    .ok_or(E::Missing("boneless COM velocity+260"))?
                    .value,
            ),
            prepared_velocity: lanes(
                p.prepare_output()
                    .velocity()
                    .ok_or(E::Missing("boneless prepared velocity+2c0"))?
                    .value,
            ),
            forward: lanes(
                p.ground_position_fields()
                    .effective_animation
                    .as_ref()
                    .ok_or(E::Missing("boneless effective animation+c0"))?
                    .columns()[2],
            ),
            centre_of_mass: lanes(
                prior
                    .centre_of_mass()
                    .ok_or(E::Missing("boneless COM+250"))?
                    .value,
            ),
        }
    };
    let input_facts = (
        input.up,
        input.current_velocity,
        input.prepared_velocity,
        input.forward,
        input.centre_of_mass,
    );
    let output = state(owner, board, skeleton, |state| {
        Ok(state
            .source3
            .as_ref()
            .ok_or(E::Missing("entered source boneless"))?
            .calculate_launch(input))
    })?;
    crate::diagnostics::test_log::event(
        "skate3_boneless_launch",
        format_args!(
            "velocity={:?} position={:?} input_up_current_prepared_forward_com={input_facts:?} native_s2_solver=true",
            output.velocity, output.position
        ),
    );
    Ok(Some((V::known(output.velocity), V::known(output.position))))
}
