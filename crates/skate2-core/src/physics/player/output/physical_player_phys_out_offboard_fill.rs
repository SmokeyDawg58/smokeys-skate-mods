//! PhysState_OffBoard::FillPhysOut 0x82d87358, then FeetIK.Fill 0x82dc8ba8.
use super::{PhysOutAccessError, PhysOutPublicationError, PhysicalPlayerPhysOut};

pub(crate) trait OffBoardFillSource {
    type Error;
    fn grabbing(&mut self) -> Result<bool, Self::Error>;
    fn locomotion_viable(&mut self) -> Result<bool, Self::Error>;
    fn grab_descriptor(&mut self) -> Result<(i32, i32), Self::Error>;
    fn left_foot_down(&mut self) -> Result<bool, Self::Error>;
    fn right_foot_down(&mut self) -> Result<bool, Self::Error>;
}

impl PhysicalPlayerPhysOut {
    /// LandingOnDeckManager.FillPhysOut82dcedb8: guarded OffBoard flag store.
    pub(crate) fn try_publish_can_land_on_skateboard(
        &mut self,
        value: bool,
    ) -> Result<(), PhysOutAccessError> {
        let guard = self.sample_write_guard()?;
        self.check_write_guard(&guard)?;
        self.concrete
            .offboard
            .as_mut()
            .ok_or(PhysOutAccessError::NotConstructed)?
            .can_land_on_skateboard = value;
        Ok(())
    }

    pub(crate) fn try_publish_offboard_fill<S: OffBoardFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PhysOutPublicationError<S::Error>> {
        use PhysOutPublicationError as E;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let grabbing = source.grabbing().map_err(E::Source)?;
        self.concrete
            .offboard
            .as_mut()
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?
            .is_grabbing_object = grabbing;

        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let viable = source.locomotion_viable().map_err(E::Source)?;
        self.concrete
            .offboard
            .as_mut()
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?
            .is_locomotion_viable = viable;

        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let (object_type, object_id) = source.grab_descriptor().map_err(E::Source)?;
        let state = self
            .concrete
            .state
            .as_mut()
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?;
        state.grab_spline_desc.object_type = object_type;
        state.grab_spline_desc.object_id = object_id;

        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let left = source.left_foot_down().map_err(E::Source)?;
        self.concrete
            .offboard
            .as_mut()
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?
            .left_foot_down = left;

        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let right = source.right_foot_down().map_err(E::Source)?;
        self.concrete
            .offboard
            .as_mut()
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?
            .right_foot_down = right;
        Ok(())
    }
}
