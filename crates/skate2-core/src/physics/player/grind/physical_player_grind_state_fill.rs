//! Grind outputs: `PhysState_Grind_Common::FillPhysOut` 0x82d90460 with each
//! type's override (Boardslide 0x82d90d78, 5050 0x82d90e38, Tipslide
//! 0x82d90e98, 5_O 0x82d90f20, Backslash 0x82d90fa8), Nonspecific
//! 0x82d86820, and `GrindManager::FillPhysOut` 0x82dd3970 with
//! `GrindJumper::FillPhysOut` 0x82dc7140. Every store follows the PhysOut
//! write-lock check the native asserts before it.
use super::{PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    input_cvar_runtime::grind_states::GrindCvar as C,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_grind_manager::{
        types::{Investigation, LEDGE},
        view::{grind_type_from_raw, root},
    },
    physical_player_physics_state::GrindKind,
    simulation_assembly_backend::constraint_session::grind_states::{
        GrindFill, GrindStateError, error, grind_fill,
    },
};
use std::sync::Arc;

impl PhysicalPlayerPhysOut {
    fn grind_guard(&self, component: PhysOutComponent) -> Result<(), GrindStateError> {
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(component).map_err(error)
    }
    fn fill_grind_common(
        &mut self,
        fill: &GrindFill,
        inv: &Investigation,
    ) -> Result<(), GrindStateError> {
        use PhysOutComponent::{Air, Grinds};
        let ledge = inv.geometry.geometry_type == LEDGE;
        let ids = inv.primitive.spline_ids().map_err(error)?;
        let audio = i32::try_from(inv.geometry.audio_surface).map_err(error)?;
        self.grind_guard(Grinds)?;
        self.concrete.grinds.as_mut().expect("checked").grind_crouch = fill.crouch;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .is_leaving_grind = fill.leaving;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .grind_primitive_dir = root(fill.dir);
        self.grind_guard(Grinds)?;
        self.concrete.grinds.as_mut().expect("checked").grind_normal = root(fill.normal);
        self.grind_guard(Grinds)?;
        self.concrete.grinds.as_mut().expect("checked").grind_skew = root(fill.skew);
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .grind_position = root(inv.primitive_location);
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .primitive_start_pos = root(inv.primitive.start);
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .primitive_end_pos = root(inv.primitive.end);
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .grind_impact_speed = inv.impact_speed;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .grind_audio_surface_type = audio;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .grind_substate = fill.substate;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .is_grinding_ledge = ledge;
        self.grind_guard(Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .is_grinding_curb = inv.geometry.is_curb;
        // mpSplineHeader: its two leading words, when the pointer is set.
        if let Some((pathed, local)) = ids {
            self.grind_guard(Grinds)?;
            self.concrete
                .grinds
                .as_mut()
                .expect("checked")
                .grind_pathed_guid = pathed;
            self.grind_guard(Grinds)?;
            self.concrete
                .grinds
                .as_mut()
                .expect("checked")
                .grind_local_guid = local;
        }
        if fill.jumped_this_frame {
            self.grind_guard(Air)?;
            self.concrete
                .air
                .as_mut()
                .expect("checked")
                .state_triggered_jump = true;
            self.grind_guard(Air)?;
            self.concrete
                .air
                .as_mut()
                .expect("checked")
                .state_intended_jump_vel = root(fill.jump_vel);
        }
        if ledge {
            self.grind_guard(Grinds)?;
            self.concrete
                .grinds
                .as_mut()
                .expect("checked")
                .ledge_high_dir = root(inv.geometry.high_side_dir);
        }
        Ok(())
    }
    /// The type's override after the common fill.
    fn fill_grind_type(
        &mut self,
        fill: &GrindFill,
        inv: &Investigation,
    ) -> Result<(), GrindStateError> {
        use PhysOutComponent::{Grinds, Physics};
        self.grind_guard(Grinds)?;
        self.concrete.grinds.as_mut().expect("checked").grind_type =
            grind_type_from_raw(fill.kind.grind_type()).map_err(error)?;
        match fill.kind {
            GrindKind::Boardslide => {
                if let Some(impulse) = fill.wipeout_impulse {
                    self.grind_guard(Physics)?;
                    self.concrete
                        .physics
                        .as_mut()
                        .expect("checked")
                        .is_impulse_for_wipeout = true;
                    self.grind_guard(Physics)?;
                    self.concrete
                        .physics
                        .as_mut()
                        .expect("checked")
                        .wipeout_impulse = root(impulse);
                }
            }
            GrindKind::FiftyFifty => {}
            GrindKind::Tipslide | GrindKind::FiveO | GrindKind::Backslash => {
                self.grind_guard(Grinds)?;
                self.concrete
                    .grinds
                    .as_mut()
                    .expect("checked")
                    .is_grind_on_front = inv.grind_on_front;
            }
        }
        Ok(())
    }
    fn fill_nonspecific(&mut self, jump: Option<[f32; 4]>) -> Result<(), GrindStateError> {
        if let Some(velocity) = jump {
            self.grind_guard(PhysOutComponent::Air)?;
            self.concrete
                .air
                .as_mut()
                .expect("checked")
                .state_triggered_jump = true;
            self.grind_guard(PhysOutComponent::Air)?;
            self.concrete
                .air
                .as_mut()
                .expect("checked")
                .state_intended_jump_vel = root(velocity);
        }
        Ok(())
    }
    /// `GrindManager::FillPhysOut`: the jumper's `mDidJump`, then the
    /// investigation's hang-primitive bit.
    pub(crate) fn try_fill_grind_manager(
        &mut self,
        did_jump: bool,
        hang_primitive_exists: bool,
    ) -> Result<(), GrindStateError> {
        self.grind_guard(PhysOutComponent::Grinds)?;
        self.concrete.grinds.as_mut().expect("checked").did_jump = did_jump;
        self.grind_guard(PhysOutComponent::Grinds)?;
        self.concrete
            .grinds
            .as_mut()
            .expect("checked")
            .hang_primitive_exists = hang_primitive_exists;
        Ok(())
    }
}

fn investigation(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Investigation, GrindStateError> {
    let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
    let p = processed.lock().map_err(error)?;
    p.grind_investigation()
        .cloned()
        .ok_or_else(|| GrindStateError("Processed grind investigation".into()))
}

/// The selected grind type's `FillPhysOut`.
pub(crate) fn try_fill_current_grind(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    kind: GrindKind,
    overlay: &DebugOverlaySlot,
) -> Result<(), GrindStateError> {
    let fill = grind_fill(owner, board, skeleton, kind)?;
    let inv = investigation(owner)?;
    owner
        .phys_out()
        .lock()
        .map_err(error)?
        .fill_grind_common(&fill, &inv)?;
    if owner
        .inputs()
        .cvars
        .try_grind_bool(C::kDisplayPhysOutGrinds)
        .map_err(error)?
    {
        // 82d90838: SingletonHolder<DebugOverlay> assertion closing the common fill.
        overlay.require().map_err(error)?;
    }
    owner
        .phys_out()
        .lock()
        .map_err(error)?
        .fill_grind_type(&fill, &inv)
}

/// Nonspecific `FillPhysOut`.
pub(crate) fn try_fill_current_nonspecific(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
) -> Result<(), GrindStateError> {
    let jump = PhysicalAssemblyWorld::nonspecific_fill(owner, board, skeleton)?;
    owner
        .phys_out()
        .lock()
        .map_err(error)?
        .fill_nonspecific(jump)
}
