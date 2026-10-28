//! ReceiveGroundDMOLineTestBatch82e72d70 delivery to the existing entity.
use super::PedestrianConstruction;
use crate::{
    living_world_mesh_store::MeshStore,
    pedestrian_ground_dmo_batches::{GroundDmoRequest, GroundDmoTest},
};

impl PedestrianConstruction {
    /// Caller resolves positioning/taze pedestrian IDs through EntityMan40 and
    /// skips absent IDs, as82e72de8..e70 does. Jump requests instead retain a
    /// native NavRig pointer: its owning entity must remain alive until receive.
    /// This method does not create an entity or replacement navigation owner.
    pub(crate) fn receive_ground_dmo_result(
        &mut self, request: &GroundDmoRequest, mesh: &MeshStore, y_tolerance: f32,
    ) -> Result<(), String> {
        if !request.submitted { return Ok(()); }
        if request.pedestrian_id != self.id {
            return Err("pedestrian default result delivered to a different entity".into());
        }
        match request.test {
            GroundDmoTest::Jump(ray) => {
                let rig = self.nav_rig.as_mut().ok_or("pedestrian jump result lost its navigation rig")?;
                let distance = rig.distances.jump.get_mut(ray)
                    .ok_or("SetJumpDistance native assertion: ray outside 0..2")?;
                *distance = request.hit_distance;
            }
            GroundDmoTest::Positioning => {
                self.spatial_adjust.as_mut().ok_or("pedestrian positioning result missing spatial adjust")?
                    .force_position_to_ground_plane(request.hit_position, request.hit_normal,
                        request.surface_id, request.hit_owner.is_none(), mesh, y_tolerance)?;
            }
            GroundDmoTest::Taze => {
                let state = self.state.as_mut().ok_or("pedestrian taze result missing state storage")?;
                // vcmpgtfp128.82e72e9c -> CR6 EQ (none of XYZ exceeded
                // epsilon), mfocrf/srwi5/insrwi writes byte3244 bit40.
                let clear = !request.taze_obstructed();
                state.flags[4] = (state.flags[4] & !0x40) | (u8::from(clear) << 6);
            }
        }
        Ok(())
    }
}
