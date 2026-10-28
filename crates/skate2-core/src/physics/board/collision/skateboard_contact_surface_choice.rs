//! Actual `CollisionInfo` reads for `ChoosePhysicsSurfaceType` 82b24878..49fc.
use super::{SkateboardContactSurface, choose_count_winner};

#[derive(Debug, PartialEq)]
pub(crate) enum BoardSurfaceChoiceError<C> {
    UnknownContact(usize),
    UnknownPhysicsType(usize),
    /// Native code indexes outside its scratch array; host refuses safely.
    InvalidVoteType(u32),
    Cvar(C),
}
impl SkateboardContactSurface {
    pub(crate) fn try_choose_physics_surface<C>(
        &mut self,
        forced: impl FnOnce() -> Result<u32, C>,
    ) -> Result<(), BoardSurfaceChoiceError<C>> {
        use BoardSurfaceChoiceError as E;
        let mut counts = [0_u32; 12];
        for wheel in 0..4 {
            if self.collision_state.in_contact[wheel].ok_or(E::UnknownContact(wheel))? {
                let kind = self
                    .collision_state
                    .surfaces
                    .physics_type(wheel)
                    .ok_or(E::UnknownPhysicsType(wheel))?;
                let count = counts
                    .get_mut(usize::try_from(kind).map_err(|_| E::InvalidVoteType(kind))?)
                    .ok_or(E::InvalidVoteType(kind))?;
                *count += 1;
            }
        }
        let voted = choose_count_winner(&counts);
        let forced = forced().map_err(E::Cvar)?;
        self.overall = Some(if forced != 0 { forced } else { voted });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_contact_fails_before_cvar_but_false_does_not_read_unknown_type() {
        let mut state = SkateboardContactSurface::new();
        assert_eq!(
            state.try_choose_physics_surface(|| -> Result<u32, ()> { panic!() }),
            Err(BoardSurfaceChoiceError::UnknownContact(0))
        );
        state.collision_state.in_contact[..4].fill(Some(false));
        state.try_choose_physics_surface(|| Ok::<_, ()>(0)).unwrap();
        assert_eq!(state.overall(), Some(1));
    }
    #[test]
    fn actual_collision_flags_and_types_win_over_unused_diagnostic_parts() {
        let mut state = SkateboardContactSurface::new();
        for (wheel, kind) in [3, 2, 3, 2].into_iter().enumerate() {
            state.collision_state.in_contact[wheel] = Some(true);
            state.collision_state.surfaces.set_physics_type(wheel, kind);
        }
        state.try_choose_physics_surface(|| Ok::<_, ()>(0)).unwrap();
        assert_eq!(state.overall(), Some(2));
        assert_eq!(
            state.try_choose_physics_surface(|| Err("late target")),
            Err(BoardSurfaceChoiceError::Cvar("late target"))
        );
        assert_eq!(state.overall(), Some(2));
        state
            .try_choose_physics_surface(|| Ok::<_, ()>(u32::MAX))
            .unwrap();
        assert_eq!(state.overall(), Some(u32::MAX));
    }
    #[test]
    fn native_scratch_types_ten_eleven_are_counted_but_not_candidates() {
        let mut state = SkateboardContactSurface::new();
        state.collision_state.in_contact[..4].fill(Some(false));
        state.collision_state.in_contact[0] = Some(true);
        state.collision_state.surfaces.set_physics_type(0, 11);
        state.try_choose_physics_surface(|| Ok::<_, ()>(0)).unwrap();
        assert_eq!(state.overall(), Some(1));
        state.collision_state.surfaces.set_physics_type(0, 12);
        assert_eq!(
            state.try_choose_physics_surface(|| -> Result<u32, ()> { panic!() }),
            Err(BoardSurfaceChoiceError::InvalidVoteType(12))
        );
        assert_eq!(state.overall(), Some(1));
    }
}
