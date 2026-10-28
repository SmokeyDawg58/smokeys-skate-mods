//! Actual Update5 stores579c..5b5c, stopping before current-state Fill5b74.
use crate::physics_state_ids::PHYSICS_STATE_OFFBOARD;
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};

pub(crate) trait PostCategorySource {
    type Error;
    fn left_foot_surface_id(&mut self) -> Result<u32, Self::Error>;
    fn flags_8d0(&mut self) -> Result<u32, Self::Error>;
    fn board_mount_invalid(&mut self) -> Result<bool, Self::Error>;
    fn interactable_object_id(&mut self) -> Result<u32, Self::Error>;
    fn teleport_exit_state(&mut self) -> Result<i32, Self::Error>;
    fn dismount_count(&mut self) -> Result<i32, Self::Error>;
    fn set_dismount_count(&mut self, value: i32) -> Result<(), Self::Error>;
    fn physically_pushing(&mut self) -> Result<bool, Self::Error>;
    fn animation_push_speed(&mut self) -> Result<f32, Self::Error>;
    fn animation_time_delta(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum PostCategoryError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl<S: std::fmt::Debug> std::fmt::Display for PostCategoryError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "post-category source: {e:?}"),
            Self::Access(e) => write!(f, "post-category output: {e:?}"),
            Self::Component(e) => write!(f, "post-category component: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    /// Fresh reads and guarded stores, not an atomic publication transaction.
    pub(crate) fn try_fill_post_category<S: PostCategorySource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PostCategoryError<S::Error>> {
        use PostCategoryError as E;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .left_foot_surface_id = source.left_foot_surface_id().map_err(E::Source)?; //57cc
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        // Source5800 really repeats LEFT934, not RIGHT938.
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .right_foot_surface_id = source.left_foot_surface_id().map_err(E::Source)?;
        for mask in [0x0020_0000, 0x0010_0000, 0x0000_0800] {
            let flags = source.flags_8d0().map_err(E::Source)?;
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let value = flags & mask != 0;
            self.check_write_guard(&guard).map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(E::Component)?;
            let output = self.concrete.offboard.as_mut().unwrap();
            match mask {
                0x0020_0000 => output.is_mounting = value,
                0x0010_0000 => output.is_unmounting = value,
                _ => output.is_air_dismount = value,
            }
        }
        let mount = source.flags_8d0().map_err(E::Source)?;
        if mount & 0x0020_0000 != 0 && mount & 0x0004_0000 != 0 {
            if source.board_mount_invalid().map_err(E::Source)? {
                self.check_current_guard().map_err(E::Access)?;
                self.require_complete_component(PhysOutComponent::OffBoard)
                    .map_err(E::Component)?;
                self.concrete
                    .offboard
                    .as_mut()
                    .unwrap()
                    .is_board_mount_invalid = true;
            }
        }
        let flags = source.flags_8d0().map_err(E::Source)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let valid = flags & 0x0040_0000 != 0;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .has_valid_interactable_object = valid;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let id = source.interactable_object_id().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(E::Component)?;
        self.concrete.state.as_mut().unwrap().interactable_object_id = id;
        if source.teleport_exit_state().map_err(E::Source)? == PHYSICS_STATE_OFFBOARD {
            source.set_dismount_count(3).map_err(E::Source)?;
        }
        let count = source.dismount_count().map_err(E::Source)?;
        if count > 0 {
            source
                .set_dismount_count(count.wrapping_sub(1))
                .map_err(E::Source)?;
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::State)
                .map_err(E::Component)?;
            self.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requests_dismount = true;
        }
        let pushing = source.physically_pushing().map_err(E::Source)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let distance = if pushing {
            self.check_write_guard(&guard).map_err(E::Access)?;
            let z_speed = self.try_motion_z_speed().map_err(E::Component)?;
            let push_speed = source.animation_push_speed().map_err(E::Source)?;
            let difference = push_speed - z_speed;
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let dt = source.animation_time_delta().map_err(E::Source)?;
            let value = difference * dt;
            self.check_write_guard(&guard).map_err(E::Access)?;
            value
        } else {
            self.check_write_guard(&guard).map_err(E::Access)?;
            0.0
        };
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Component)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .unwrap()
            .contact_error_dist = distance;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_post_category_tests.rs"]
mod tests;
