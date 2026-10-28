//! CheckForSkeletonCollisionWipeoutAir82dd8c90, distinct from OffBoardAir.
use super::*;
impl PhysicalAssemblyWorld {
    pub(crate) fn try_check_landing_skeleton_air_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(E::Pair)?
        {
            return Err(E::ForeignOwner);
        }
        let overlay = self.debug_overlay.clone();
        let mut s = Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay: &overlay,
        };
        let squash = s.wipeout_hash(0x09ed_0901_2ea6_80e3)?;
        if detector::leaves::squashed(&mut s, squash)? {
            s.request(18)?;
            return Ok(());
        }
        let displacement = s.wipeout_hash(0xaa44_05c5_949d_1845)?;
        if detector::leaves::displacement(&mut s, displacement)? {
            s.request(1)?;
            return Ok(());
        }
        let arms = s.wipeout_pointer(0xd031_7181_ce40_bfbc)?;
        let body = s.wipeout_pointer(0x68e0_e915_d4f3_af66)?;
        let arms = s.wipeout_float(&arms)?;
        let body = s.wipeout_float(&body)?;
        if detector::leaves::contact_force(&mut s, body, arms)? {
            s.request(0)?;
            if s.owner
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
    pub(crate) fn try_landing_heavy_wipeout_threshold<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        let overlay = self.debug_overlay.clone();
        Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay: &overlay,
        }
        .wipeout_hash(0xee81_dd78_506e_4a2d)
    }
}
