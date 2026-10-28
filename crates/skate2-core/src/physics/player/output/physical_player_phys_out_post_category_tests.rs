use super::*;
use crate::physical_player_phys_out::{PhysOutMotionScalar, construct::PhysOutAllocation};

#[derive(Default)]
struct Source {
    calls: Vec<&'static str>,
    left: u32,
    flags: u32,
    exit: i32,
    count: i32,
    pushing: bool,
    fail_dt: bool,
    mount_invalid: bool,
}
impl PostCategorySource for Source {
    type Error = &'static str;
    fn left_foot_surface_id(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("left");
        self.left += 1;
        Ok(self.left)
    }
    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("flags");
        Ok(self.flags)
    }
    fn board_mount_invalid(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("mount");
        Ok(self.mount_invalid)
    }
    fn interactable_object_id(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("object");
        Ok(91)
    }
    fn teleport_exit_state(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("exit");
        Ok(self.exit)
    }
    fn dismount_count(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("count");
        Ok(self.count)
    }
    fn set_dismount_count(&mut self, value: i32) -> Result<(), Self::Error> {
        self.calls.push("set_count");
        self.count = value;
        Ok(())
    }
    fn physically_pushing(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("pushing");
        Ok(self.pushing)
    }
    fn animation_push_speed(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("speed");
        Ok(5.0)
    }
    fn animation_time_delta(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("dt");
        if self.fail_dt { Err("dt") } else { Ok(0.25) }
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let guard = out.sample_write_guard().unwrap();
    out.publish_motion_scalar(&guard, PhysOutMotionScalar::ZSpeed, 2.0)
        .unwrap();
    out
}
#[test]
fn duplicate_left_reads_and_false_push_preserve_latches() {
    let mut out = output();
    out.concrete
        .state
        .as_mut()
        .unwrap()
        .physics_requests_dismount = true;
    out.concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .contact_error_dist = 7.0;
    let mut source = Source {
        flags: 0x0050_0800,
        ..Source::default()
    };
    out.try_fill_post_category(&mut source).unwrap();
    let board = out.concrete.offboard.as_ref().unwrap();
    assert_eq!(
        (board.left_foot_surface_id, board.right_foot_surface_id),
        (1, 2)
    );
    assert!(!board.is_mounting);
    assert!(board.is_unmounting && board.is_air_dismount && board.has_valid_interactable_object);
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requests_dismount
    );
    assert_eq!(
        out.concrete.state.as_ref().unwrap().interactable_object_id,
        91
    );
    assert_eq!(
        out.concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .contact_error_dist
            .to_bits(),
        0
    );
    assert_eq!(
        source.calls,
        [
            "left", "left", "flags", "flags", "flags", "flags", "flags", "object", "exit", "count",
            "pushing"
        ]
    );
}
#[test]
fn mount_validation_sets_invalid_without_clearing_an_existing_latch() {
    let mut out = output();
    let mut source = Source {
        flags: 0x0024_0000,
        ..Source::default()
    };
    out.try_fill_post_category(&mut source).unwrap();
    assert!(
        !out.concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_board_mount_invalid
    );
    assert_eq!(source.calls[6], "mount");
    source.mount_invalid = true;
    out.try_fill_post_category(&mut source).unwrap();
    assert!(
        out.concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_board_mount_invalid
    );
    source.mount_invalid = false;
    out.try_fill_post_category(&mut source).unwrap();
    assert!(
        out.concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_board_mount_invalid
    );
}
#[test]
fn teleport_refreshes_and_decrements_actual_count_before_push_failure() {
    let mut out = output();
    out.concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .contact_error_dist = 7.0;
    let mut source = Source {
        exit: 500,
        count: 8,
        pushing: true,
        fail_dt: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_fill_post_category(&mut source),
        Err(PostCategoryError::Source("dt"))
    ));
    assert_eq!(source.count, 2);
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requests_dismount
    );
    assert_eq!(
        out.concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .contact_error_dist
            .to_bits(),
        7.0_f32.to_bits()
    );
    assert_eq!(
        &source.calls[8..],
        [
            "exit",
            "set_count",
            "count",
            "set_count",
            "pushing",
            "speed",
            "dt"
        ]
    );
}
#[test]
fn push_distance_uses_published_z_speed_and_actual_animation_dt() {
    let mut out = output();
    let mut source = Source {
        count: 1,
        pushing: true,
        ..Source::default()
    };
    out.try_fill_post_category(&mut source).unwrap();
    assert_eq!(source.count, 0);
    assert_eq!(
        out.concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .contact_error_dist
            .to_bits(),
        0.75_f32.to_bits()
    );
}
