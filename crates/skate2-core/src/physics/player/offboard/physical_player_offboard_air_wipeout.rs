//! CheckForOffBoardAirWipeout82dd8ef0, on the existing collision/request owners.
use super::*;

impl PhysicalAssemblyWorld {
    pub(crate) fn try_check_offboard_air_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let overlay = self.debug_overlay.clone();
        let mut source = Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay: &overlay,
        };
        if source.state()? != 4 {
            source.set_state(4)?;
        }
        let velocity = source
            .world
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .general_update()
            .animation_world_velocity();
        let speed_squared = source
            .numeric
            .dot_vector(velocity, velocity)
            .map_err(E::Numeric)?;
        // Raw insrdi82dd8f78: r8 inserts the HIGH word, r4 retains the LOW word.
        let threshold = source.wipeout_hash(0x1215_5448_d54e_f4db)?;
        let threshold_squared = threshold * threshold;
        if owner
            .inputs()
            .cvars
            .try_wipeout_display_offboard_info()
            .map_err(E::Cvar)?
        {
            overlay.require().map_err(E::Overlay)?;
        }
        // Raw82dd8fdc BO4/BI25 branches when GT is false, including unordered.
        if speed_squared.partial_cmp(&threshold_squared) != Some(std::cmp::Ordering::Greater) {
            return Ok(());
        }
        let squashed = source.wipeout_hash(0xc1d6_4bcc_60f0_8410)?;
        if detector::leaves::squashed(&mut source, squashed)? {
            source.request(18)?;
            return Ok(());
        }
        let displacement = source.wipeout_hash(0x50c9_61ac_b033_757d)?;
        if detector::leaves::displacement(&mut source, displacement)? {
            source.request(1)?;
            return Ok(());
        }
        // Both native attribute pointers are captured before either float loads.
        let arms = source.wipeout_pointer(0x6167_b209_2124_3e65)?;
        let body = source.wipeout_pointer(0xd513_aec1_7267_304a)?;
        let arms = source.wipeout_float(&arms)?;
        let body = source.wipeout_float(&body)?;
        if detector::leaves::contact_force(&mut source, body, arms)? {
            source.request(0)?;
            if owner
                .inputs()
                .cvars
                .try_wipeout_display_debug_info()
                .map_err(E::Cvar)?
            {
                overlay.require().map_err(E::Overlay)?;
            }
        }
        Ok(())
    }
}
