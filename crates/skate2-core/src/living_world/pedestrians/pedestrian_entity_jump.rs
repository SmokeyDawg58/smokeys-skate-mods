//! Concrete entity jump callbacks on the retained core/chase/spatial owners.
use super::PedestrianConstruction;
use crate::{input_cvar_runtime::InputCvarReferences, reckoning_numeric::ReckoningNumeric};

impl PedestrianConstruction {
    /// Pedestrian::GetMaxChaseSpeed82e6f378 enters through Chaser+192.
    /// Raw +18b0/+18b4 are entity mChaseParameters+0/+4, not iterator data.
    pub(crate) fn max_chase_speed(&self) -> Result<f32, String> {
        let chase = &self.resources.as_ref().ok_or("pedestrian resources not constructed")?.chase;
        if chase.is_null() { return Err("GetMaxChaseSpeed native assertion: absent chase collection".into()); }
        let data = chase.try_cached_layout_field(0, 4).map_err(|e| e.to_string())?
            .ok_or("GetMaxChaseSpeed chase layout absent")?;
        Ok(f32::from_be_bytes(data.bytes().try_into().map_err(|_| "MaxChaseSpeed field width")?))
    }

    /// SetJumpToPosition82e701f8 reads facing first, position second. Entity
    /// GetFacing8258f128 returns actual transform Z; GetPosition82e94640
    /// returns actual transform translation, not target transform/NavRig cache.
    pub(crate) fn set_jump_to_position(&mut self, landing: [f32; 4], ledge_distance: f32)
        -> Result<(), String>
    {
        let core = self.core.as_ref().ok_or("pedestrian core not constructed")?;
        let facing = core.transform[2];
        let position = core.transform[3];
        self.spatial_adjust.as_mut().ok_or("pedestrian spatial adjust not constructed")?
            .set_jump_to_position(position, facing, landing, ledge_distance);
        Ok(())
    }

    /// JumpNow82e80f90 reads Chaser::GetMaxChaseSpeed BEFORE GetPosition,
    /// then invokes the actual retained SpatialAdjust trajectory producer.
    pub(crate) fn jump_now<N: ReckoningNumeric>(&mut self, cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        let speed = self.max_chase_speed()?;
        let position = self.core.as_ref().ok_or("pedestrian core not constructed")?.transform[3];
        let tuning = cvars.try_pedestrian_jump_tuning().map_err(|e| format!("pedestrian jump cvars: {e:?}"))?;
        self.spatial_adjust.as_mut().ok_or("pedestrian spatial adjust not constructed")?
            .jump_now(position, speed, &tuning, numeric)
    }
}
