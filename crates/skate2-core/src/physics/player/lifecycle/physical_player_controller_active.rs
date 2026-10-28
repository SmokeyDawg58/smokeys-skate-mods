//! Active UpdateState82dd18e0 / Update82dd1d68 on the retained board/controller.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_length},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_controller::{
        ControllerRetrieval, SkateboardController, inactive_update::ControllerUpdateError,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum ControllerActiveError {
    Poisoned,
    Missing(&'static str),
    ForeignOwner,
    Service(&'static str, String),
}
use ControllerActiveError as E;
fn service(site: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(site, format!("{error:?}"))
}
fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, E> {
    Ok(Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    ))
}
fn state(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<u32, E> {
    owner
        .skateboard_controller()
        .lock()
        .map_err(|_| E::Poisoned)?
        .state()
        .ok_or(E::Missing("controller state"))
}
fn store_state(owner: &Arc<PhysicalPlayerAssemblyOwner>, state: u32) -> Result<(), E> {
    owner
        .skateboard_controller()
        .lock()
        .map_err(|_| E::Poisoned)?
        .try_store_active_state::<()>(state)
        .map_err(|e| service("state store", e))
}
fn flags(owner: &Arc<PhysicalPlayerAssemblyOwner>, word8d0: bool) -> Result<u32, E> {
    let p = processed(owner)?;
    let p = p.lock().map_err(|_| E::Poisoned)?;
    let f = p.physics_state_fields();
    if word8d0 { f.flags_8d0 } else { f.flags_8cc }.ok_or(E::Missing("Processed controller flags"))
}
fn requested(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<bool, E> {
    Ok(flags(owner, true)? & 0x200000 != 0 || flags(owner, false)? & 0x1000 != 0)
}
pub(super) fn scalar(controller: &SkateboardController, key: u64) -> Result<f32, E> {
    Ok(f32::from_bits(
        controller
            .attributes()
            .ok_or(E::Missing("controller attributes"))?
            .try_scalar_word(key, 0)
            .map_err(|e| service("scalar attribute", e))?
            .unwrap_or(0),
    ))
}
pub(super) fn curve(controller: &SkateboardController, key: u64, input: f32) -> Result<f32, E> {
    let data = controller
        .attributes()
        .ok_or(E::Missing("controller attributes"))?
        .try_get_data(key, 0)
        .map_err(|e| service("curve attribute", e))?;
    if data.as_ref().is_some_and(|d| d.bytes().len() < 80) {
        return Err(E::Missing("PointGraph8 attribute extent"));
    }
    let read = |i: usize| {
        data.as_ref().map_or(0.0, |d| {
            f32::from_be_bytes(
                d.bytes()[i * 4..i * 4 + 4]
                    .try_into()
                    .expect("checked graph extent"),
            )
        })
    };
    Ok(crate::physics::PointGraph::<8> {
        x: std::array::from_fn(|i| read(i + 4)),
        y: std::array::from_fn(|i| read(i + 12)),
    }
    .evaluate(input))
}
fn retrieval(controller: &mut SkateboardController) -> Result<&mut ControllerRetrieval, E> {
    controller
        .try_active_retrieval_mut::<()>()
        .map_err(|e| service("retrieval data", e))
}
fn progress(time: f32, total: f32) -> f32 {
    let ratio = time / total;
    if 1.0 - ratio >= 0.0 { ratio } else { 1.0 }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_active_controller(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        trig: &mut HostReckoningSinCosNumeric,
    ) -> Result<(), E> {
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(|e| service("actual pair", e))?
        {
            return Err(E::ForeignOwner);
        }
        match owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_inactive()
        {
            Ok(()) => return Ok(()),
            Err(ControllerUpdateError::PendingUpdateState) => {}
            Err(e) => return Err(service("controller admission", e)),
        }
        match state(owner)? {
            1 => {
                let prior = self
                    .try_update_pair_controller_grip(owner, board, skeleton)
                    .map_err(|e| service("UpdateGrip", e))?;
                if (flags(owner, false)? & 0x4000 != 0 || prior)
                    && flags(owner, true)? & 0x200000 == 0
                    && state(owner)? != 2
                {
                    self.active_let_go(owner, board, skeleton, refs)?;
                    store_state(owner, 2)?;
                }
            }
            2 => {
                let wanted = requested(owner)?;
                // Both calls occur even if the request already selects RETURNING.
                let automatic = self.active_auto_retrieve(owner, numeric)?;
                let contacts = self
                    .try_with_board_force_runtime(board, |r| Arc::clone(&r.contact_surface))
                    .map_err(|e| service("board contacts", e))?;
                let unrideable = contacts
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .overall()
                    .ok_or(E::Missing("overall physics surface"))?
                    == 6;
                if wanted {
                    if state(owner)? != 4 {
                        self.active_init_retrieval(owner, board, skeleton, refs, numeric)?;
                        store_state(owner, 4)?;
                    }
                } else if (automatic || unrideable) && state(owner)? != 3 {
                    let p = processed(owner)?;
                    let mut c = owner
                        .skateboard_controller()
                        .lock()
                        .map_err(|_| E::Poisoned)?;
                    self.try_initialize_board_hiding(board, refs, &p, &mut c, numeric)
                        .map_err(|e| service("InitSkateboardHiding", e))?;
                    c.try_store_active_state::<()>(3)
                        .map_err(|e| service("hiding state", e))?;
                }
            }
            3 => {
                if requested(owner)? {
                    self.active_init_retrieval(owner, board, skeleton, refs, numeric)?;
                    store_state(owner, 4)?;
                }
            }
            4 => {
                let param = owner
                    .skateboard_controller()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .retrieval()
                    .ok_or(E::Missing("retrieval"))?
                    .parametric_time;
                if param < 1.0 {
                    if refs
                        .try_skc_grab_cheat()
                        .map_err(|e| service("GrabCheat", e))?
                        && flags(owner, true)? & 0x200000 == 0
                    {
                        self.active_let_go(owner, board, skeleton, refs)?;
                        store_state(owner, 2)?;
                    }
                } else {
                    self.try_set_board_velocity(board, &V::known([0.0; 4]))
                        .map_err(|e| service("return complete velocity", e))?;
                    if state(owner)? != 1 {
                        self.try_hold_pair_controller(owner, board, skeleton, refs, numeric)
                            .map_err(|e| service("HoldSkateboard", e))?;
                        store_state(owner, 1)?;
                    }
                }
            }
            _ => {}
        }
        // Update reloads the state after UpdateState; a transition's child runs now.
        match state(owner)? {
            1 => self
                .try_update_pair_controller_drive_frames(owner, skeleton, refs, numeric)
                .map_err(|e| service("UpdateDriveFrames", e)),
            3 => self.active_update_hiding(owner, board, refs),
            4 => self.active_update_retrieval(owner, board, skeleton, numeric, trig),
            _ => Ok(()),
        }
    }
    fn active_let_go(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        let p = processed(owner)?;
        let mut c = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?;
        self.try_let_go_retained_controller::<()>(board, skeleton, refs, &p, &mut c)
            .map_err(|e| service("LetGoOfSkateboard", e))
    }
    fn active_auto_retrieve(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<bool, E> {
        let threshold = scalar(
            &*owner
                .skateboard_controller()
                .lock()
                .map_err(|_| E::Poisoned)?,
            0x893d_a1b9_6623_1d76,
        )?;
        let p = processed(owner)?;
        let p = p.lock().map_err(|_| E::Poisoned)?;
        let deck = p
            .deck_transform()
            .ok_or(E::Missing("deck transform"))?
            .columns()[3];
        let com = p
            .prior_output()
            .centre_of_mass()
            .ok_or(E::Missing("COM"))?
            .value;
        let delta = deck.zip(com, |a, b| a - b);
        drop(p);
        Ok(numeric
            .dot_vector(delta, delta)
            .map_err(|e| service("auto retrieval dot", e))?
            > threshold * threshold)
    }
    pub(super) fn active_animation_root(&self, skeleton: &Handle) -> Result<M, E> {
        let normal = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?;
        let local = normal
            .animation_targets()
            .volume_row(0)
            .ok_or(E::Missing("current animation board root"))?;
        Ok(normal
            .root_transforms()
            .animation_to_world()
            .compose(&M::known(*local)))
    }
    fn active_init_retrieval(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), E> {
        self.try_disable_board_collision_volumes(board)
            .map_err(|e| service("retrieval collision disable", e))?;
        self.try_clear_board_grip_enabled(board)
            .map_err(|e| service("retrieval clear grip", e))?;
        self.try_enable_board_angular_drive_only(board, 1.0, refs, numeric)
            .map_err(|e| service("retrieval angular drive", e))?;
        let mut c = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?;
        let old_position = retrieval(&mut c)?.start.columns()[3];
        c.try_reset_active_retrieval::<()>()
            .map_err(|e| service("Retrieval.Reset", e))?;
        let p = processed(owner)?;
        retrieval(&mut c)?.start = *p
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::Missing("deck transform"))?;
        if c.state() == Some(3) {
            retrieval(&mut c)?.start.set_column(3, old_position);
        }
        retrieval(&mut c)?.target = self.active_animation_root(skeleton)?;
        let r = retrieval(&mut c)?;
        let mut delta = r.start.columns()[3].zip(r.target.columns()[3], |a, b| a - b);
        let mut distance =
            try_length(delta, numeric).map_err(|e| service("retrieval distance", e))?;
        if distance < 0.001 {
            distance = 0.001;
            delta = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .ground_position_fields()
                .effective_animation
                .as_ref()
                .ok_or(E::Missing("effective animation"))?
                .columns()[2]
                .map(|a| a * 0.001);
        }
        let flag = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::Missing("Processed8d0"))?
            & 0x200000
            != 0;
        let maximum = scalar(
            &c,
            if flag {
                0x3076_cd60_6165_bbec
            } else {
                0xea33_d941_8435_8327
            },
        )?;
        if distance > maximum || c.state() == Some(3) {
            let estimate = numeric
                .reciprocal_estimate(distance)
                .map_err(|e| service("retrieval reciprocal", e))?;
            let reciprocal =
                crate::simulation_drive_jacobian::refine_drive_reciprocal(distance, estimate);
            let r = retrieval(&mut c)?;
            let position = delta
                .map(|a| a * maximum)
                .zip(r.target.columns()[3], |a, b| a.mul_add(reciprocal, b));
            r.start.set_column(3, position);
            self.try_set_controller_board_position(board, &position, refs)
                .map_err(|e| service("retrieval SetPosition", e))?;
            distance = maximum;
        }
        let time = curve(&c, 0x34e2_f893_31f2_eb83, distance)?;
        retrieval(&mut c)?.total_time = time;
        retrieval(&mut c)?.time = 0.0;
        if p.lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::Missing("Processed8d0"))?
            & 0x200000
            != 0
        {
            let cap = scalar(&c, 0x4bf4_4e39_2c38_4895)?;
            let r = retrieval(&mut c)?;
            r.total_time = if cap - r.total_time >= 0.0 {
                r.total_time
            } else {
                cap
            };
        }
        Ok(())
    }
    fn active_update_hiding(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        let mut c = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?;
        let r = retrieval(&mut c)?;
        r.time += f32::from_bits(0x3c88_8889);
        r.parametric_time = 0.5;
        r.warped_parametric_time = 0.5;
        let p = processed(owner)?;
        let animation_position = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .ground_position_fields()
            .effective_animation
            .as_ref()
            .ok_or(E::Missing("effective animation"))?
            .columns()[3];
        let offset = scalar(&c, 0x6cff_6f2c_43f0_ad41)?;
        let normal = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .ground_normal()
            .ok_or(E::Missing("ground normal"))?
            .value;
        let r = retrieval(&mut c)?;
        r.target = r.start;
        r.target.set_column(
            3,
            normal.zip(animation_position, |a, b| a.mul_add(offset, b)),
        );
        r.current = r.target;
        self.try_set_controller_board_target_orientation(board, &r.current)
            .map_err(|e| service("hiding orientation", e))?;
        self.try_set_controller_board_position(board, &r.current.columns()[3], refs)
            .map_err(|e| service("hiding position", e))?;
        self.try_set_board_velocity(board, &V::known([0.0; 4]))
            .map_err(|e| service("hiding velocity", e))?;
        Ok(())
    }
    fn active_update_retrieval(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        numeric: &mut HostReckoningNumeric,
        trig: &mut HostReckoningSinCosNumeric,
    ) -> Result<(), E> {
        let mut c = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?;
        let r = retrieval(&mut c)?;
        r.time += f32::from_bits(0x3c88_8889);
        r.parametric_time = progress(r.time, r.total_time);
        let param = r.parametric_time;
        let warped = curve(&c, 0xbb14_b9c2_9d2f_6803, param)?;
        let r = retrieval(&mut c)?;
        r.warped_parametric_time = warped;
        r.target = self.active_animation_root(skeleton)?;
        r.current = crate::skeleton_affine_interpolation::try_interpolate_affine_typed(
            &r.start, &r.target, warped, numeric, trig,
        )
        .map_err(|e| service("retrieval interpolation", e))?;
        self.try_set_controller_board_target_orientation(board, &r.current)
            .map_err(|e| service("retrieval orientation", e))?;
        self.try_target_controller_board_position_with_velocity(board, &r.current.columns()[3])
            .map_err(|e| service("retrieval position velocity", e))?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieval_progress_keeps_native_fsel_and_no_lower_clamp() {
        assert_eq!(progress(-0.5, 1.0), -0.5);
        assert_eq!(progress(2.0, 1.0), 1.0);
        assert_eq!(progress(0.0, 0.0), 1.0);
        assert_eq!(progress(f32::NAN, 1.0), 1.0);
        assert_eq!(progress(0.5, 1.0), 0.5);
    }
}
