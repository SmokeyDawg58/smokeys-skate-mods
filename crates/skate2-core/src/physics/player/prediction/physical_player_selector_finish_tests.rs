#![expect(
    clippy::float_cmp,
    reason = "exact publications and fixture source arithmetic"
)]
use super::*;
use crate::{
    physical_player_trajectory_owners::selector_update::tests::owner,
    reckoning_numeric::HostReckoningNumeric,
};
struct Source {
    calls: Vec<String>,
    threshold: f32,
    maximum: f32,
    fail: Option<usize>,
    debug: bool,
    fail_overlay: bool,
    fail_graph: bool,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            threshold: f32::INFINITY,
            maximum: 100.0,
            fail: None,
            debug: false,
            fail_overlay: false,
            fail_graph: false,
        }
    }
}
impl FinishSource for Source {
    type Error = &'static str;
    type Layout = ();
    fn layout(&mut self) -> Result<(), Self::Error> {
        self.calls.push("layout".into());
        Ok(())
    }
    fn scalar(&mut self, (): &(), offset: usize) -> Result<f32, Self::Error> {
        self.calls.push(format!("scalar{offset:x}"));
        if self.fail == Some(offset) {
            return Err("scalar");
        }
        Ok(match offset {
            0x1f4 => self.maximum,
            0x1f0 => self.threshold,
            0x244 => 2.0,
            0x23c => 3.0,
            _ => panic!("offset"),
        })
    }
    fn graph(&mut self, (): &(), offset: usize, _: f32) -> Result<f32, Self::Error> {
        assert_eq!(offset, 0x60);
        self.calls.push("graph".into());
        if self.fail_graph {
            Err("graph")
        } else {
            Ok(1.0)
        }
    }
    fn up(&mut self) -> Result<V, Self::Error> {
        self.calls.push("up".into());
        Ok(V::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn ground_normal(&mut self) -> Result<V, Self::Error> {
        self.calls.push("ground".into());
        Ok(V::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn show_wall(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("wall".into());
        Ok(false)
    }
    fn show_trajectories(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("trajectories".into());
        Ok(false)
    }
    fn show_select(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("select".into());
        Ok(self.debug)
    }
    fn overlay(&mut self) -> Result<(), Self::Error> {
        self.calls.push("overlay".into());
        if self.fail_overlay {
            Err("overlay")
        } else {
            Ok(())
        }
    }
}
fn winner() -> OnboardTrajectoryOwner {
    let mut s = owner();
    s.winning_result = Some(0);
    s.winning_buffer = s.queries[0].result;
    s.winning_data.landing_normal = V::known([0.0, 1.0, 0.0, 0.0]);
    s.winning_data.velocity = V::known([7.0, 8.0, 9.0, 10.0]);
    s.launch.com_position = V::known([0.0, 2.0, 0.0, 0.0]);
    s.pass = 1;
    s
}
#[test]
fn com_copies_winner_and_reads_maximum_even_at_frame_zero() {
    let mut s = winner();
    s.winning_buffer.frame = 0;
    let mut src = Source {
        fail: Some(0x1f4),
        ..Source::default()
    };
    assert!(
        s.calculate_com(&mut src, &mut HostReckoningNumeric)
            .is_err()
    );
    assert_eq!(s.winning_com.position, s.winning_buffer.trajectory.position);
    assert_eq!(src.calls, ["layout", "graph", "up", "layout", "scalar1f4"]);
}
#[test]
fn graph_failure_precedes_com_copy() {
    let mut s = winner();
    s.winning_com.position = V::known([9.0; 4]);
    let mut src = Source {
        fail_graph: true,
        ..Source::default()
    };
    assert!(
        s.calculate_com(&mut src, &mut HostReckoningNumeric)
            .is_err()
    );
    assert_eq!(s.winning_com.position, V::known([9.0; 4]));
}
#[test]
fn com_position_preserves_subtract_add_rounding() {
    let mut s = winner();
    s.winning_buffer.trajectory.position.xyz[0] = 1.0e20;
    s.launch.com_position.xyz[0] = 1.0;
    let mut src = Source {
        maximum: f32::NAN,
        ..Source::default()
    };
    s.calculate_com(&mut src, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(s.winning_com.position.xyz[0], 0.0);
    assert_ne!(s.winning_com.position.xyz[0], s.launch.com_position.xyz[0]);
}
#[test]
fn com_unknown_w_and_time_are_retained() {
    let mut s = winner();
    s.winning_buffer.trajectory.position.w = None;
    s.winning_buffer.trajectory.time = [3.0, 4.0, 5.0, 6.0];
    s.calculate_com(&mut Source::default(), &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(s.winning_com.position.xyz, [0.0, 2.0, 0.0]);
    assert_eq!(s.winning_com.position.w, None);
    assert_eq!(s.winning_com.velocity.w, None);
    assert_eq!(s.winning_com.time, [3.0, 4.0, 5.0, 6.0]);
}
#[test]
fn ai_second_pass_return_preserves_targeting_and_skips_attributes() {
    let mut s = winner();
    s.ai_trajectory_following = true;
    s.targeting_grind = true;
    let mut src = Source::default();
    assert!(!s.second_pass(&mut src, &mut HostReckoningNumeric).unwrap());
    assert!(s.targeting_grind);
    assert!(src.calls.is_empty());
}
#[test]
fn unordered_angle_threshold_requests_second_pass() {
    let mut s = winner();
    let mut src = Source {
        threshold: f32::NAN,
        ..Source::default()
    };
    assert!(
        s.try_finish_completed_query(&mut src, &mut HostReckoningNumeric)
            .unwrap()
    );
    assert!(s.ready);
    assert_eq!(s.launch.start_velocity, s.winning_data.velocity);
    assert_eq!(s.launch.cone_angle_x, 2.0);
    assert_eq!(s.launch.cone_angle_z, 3.0);
    assert_eq!(s.pass, 1);
}
#[test]
fn launch_attribute_failure_leaves_ready_and_start_velocity_published() {
    let mut s = winner();
    let mut src = Source {
        threshold: f32::NAN,
        fail: Some(0x244),
        ..Source::default()
    };
    assert!(
        s.try_finish_completed_query(&mut src, &mut HostReckoningNumeric)
            .is_err()
    );
    assert!(s.ready);
    assert_eq!(s.launch.start_velocity, s.winning_data.velocity);
    assert_eq!(s.pass, 1);
    assert_eq!(src.calls.last().unwrap(), "scalar244");
}
#[test]
fn debug_overlay_failure_precedes_ready_publication() {
    let mut s = winner();
    let mut src = Source {
        debug: true,
        fail_overlay: true,
        ..Source::default()
    };
    assert!(matches!(
        s.try_finish_completed_query(&mut src, &mut HostReckoningNumeric),
        Err(SelectorError::Source("overlay"))
    ));
    assert!(!s.ready);
    assert_eq!(s.winning_com.position.xyz[1], 2.0);
}
#[test]
fn debug_body_keeps_each_overlay_lookup() {
    let mut s = winner();
    let mut src = Source {
        debug: true,
        ..Source::default()
    };
    assert!(
        !s.try_finish_completed_query(&mut src, &mut HostReckoningNumeric)
            .unwrap()
    );
    assert!(s.ready);
    assert_eq!(src.calls.iter().filter(|s| *s == "overlay").count(), 4);
}
