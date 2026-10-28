//! SecurityGuard82e98778/82e70330 and original WorldPainter patrol-zone queries.
use crate::world_painter_layer::{WorldPainterLayerError, WorldPainterLayerMan};

#[derive(Debug)]
pub(crate) enum SecurityGuardError {
    Layer(WorldPainterLayerError),
    InvalidPatrolZone,
    UnterminatedZoneName,
}
impl std::fmt::Display for SecurityGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "security guard: {self:?}")
    }
}
impl std::error::Error for SecurityGuardError {}

#[derive(Default, Debug)]
pub(crate) struct WorldPainterPatrolZone {
    name: String,
}
impl WorldPainterPatrolZone {
    pub(crate) fn is_valid(&self) -> bool {
        !self.name.is_empty()
    }
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    /// SetZoneByPosition82e57c38 truncates at the first literal Area.
    pub(crate) fn set_by_position(
        &mut self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
    ) -> Result<(), SecurityGuardError> {
        let name = world
            .try_region_name(
                crate::session_marker::SECURITY_GUARD_AREAS,
                [position[0], position[2]],
            )
            .map_err(SecurityGuardError::Layer)?
            .unwrap_or("");
        let name = name.split_once("Area").map_or(name, |(prefix, _)| prefix);
        // Native strncpy capacity64 must leave a terminated name for strcmp.
        if name.len() >= 64 {
            return Err(SecurityGuardError::UnterminatedZoneName);
        }
        self.name = name.to_owned();
        Ok(())
    }
    fn query(
        &self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
        buffer: bool,
        area: bool,
    ) -> Result<bool, SecurityGuardError> {
        if !self.is_valid() {
            return Err(SecurityGuardError::InvalidPatrolZone);
        }
        let Some(region) = world
            .try_region_name(
                crate::session_marker::SECURITY_GUARD_AREAS,
                [position[0], position[2]],
            )
            .map_err(SecurityGuardError::Layer)?
        else {
            return Ok(false);
        };
        let suffix = region.strip_prefix(&self.name);
        Ok((area && suffix == Some("Area")) || (buffer && suffix == Some("Buffer")))
    }
    pub(crate) fn contains(
        &self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
    ) -> Result<bool, SecurityGuardError> {
        self.query(world, position, false, true)
    }
    pub(crate) fn contains_buffer(
        &self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
    ) -> Result<bool, SecurityGuardError> {
        self.query(world, position, true, false)
    }
    pub(crate) fn contains_boundary_or_buffer(
        &self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
    ) -> Result<bool, SecurityGuardError> {
        self.query(world, position, true, true)
    }
}

/// Stored on the existing Pedestrian; the guard is not a separate simulation entity.
#[derive(Default, Debug)]
pub(crate) struct SecurityGuardData {
    pub(crate) zone: WorldPainterPatrolZone,
    pub(crate) previous_guardee_position: [f32; 3],
    pub(crate) guardee_closing_velocity: f32,
    pub(crate) outside_cull_radius: bool,
    pub(crate) break_off_forced: bool,
}
impl SecurityGuardData {
    /// PostInitialize82e70330; caller also sets Pedestrian AI bit8 at+3240.
    pub(crate) fn post_initialize(
        &mut self,
        world: &WorldPainterLayerMan,
        position: [f32; 3],
    ) -> Result<(), SecurityGuardError> {
        if !self.zone.is_valid() {
            self.zone.set_by_position(world, position)?;
        }
        self.guardee_closing_velocity = 0.0;
        self.previous_guardee_position = [0.0; 3];
        Ok(())
    }
    /// PreStateGraphUpdate82e9f650..718 samples target displacement / dt
    /// Caller separately samples/stores prior position after this computation,
    /// preserving native's second GetPosition call82e9f708.
    pub(crate) fn update_guardee_velocity<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        guardee: Option<[f32; 3]>,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        if let Some(position) = guardee {
            let previous = self.previous_guardee_position;
            let delta = [
                previous[0] - position[0],
                previous[1] - position[1],
                previous[2] - position[2],
                0.0,
            ];
            self.guardee_closing_velocity =
                crate::reckoning_numeric::try_length(delta, numeric)? * (1.0 / dt);
        }
        Ok(())
    }
    /// SetIsOutsideCullRadius82e96680, Census non-cullable guard callback.
    pub(crate) fn set_is_outside_cull_radius(&mut self, outside: bool) {
        self.outside_cull_radius = outside;
    }
    /// ChallengeSystemCanCull82e70420 queries the actual chasee interface.
    pub(crate) fn challenge_system_can_cull(&self, chasee_exists: bool) -> bool {
        !chasee_exists && self.outside_cull_radius
    }
    /// ForceBreakOffChase82e96698. This latch is not cleared by range changes.
    pub(crate) fn force_break_off_chase(&mut self) {
        self.break_off_forced = true;
    }
    pub(crate) fn is_break_off_chase_forced(&self) -> bool {
        self.break_off_forced
    }
}
