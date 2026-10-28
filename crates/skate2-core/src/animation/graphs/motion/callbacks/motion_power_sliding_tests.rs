#![expect(clippy::float_cmp, reason = "exact authored branch and timer outputs")]
use super::*;
struct Source {
    ground: bool,
    speed: f32,
    inputs: [Option<f32>; 2],
    starts: [bool; 2],
    start_output: [bool; 2],
    end_output: [bool; 2],
    times: [f32; 2],
    graph_result: [f32; 3],
    fail_graph: bool,
    events: Vec<&'static str>,
}
fn source() -> Source {
    Source {
        ground: true,
        speed: 5.0,
        inputs: [None; 2],
        starts: [false; 2],
        start_output: [true; 2],
        end_output: [false; 2],
        times: [0.0; 2],
        graph_result: [1.0, 0.0, 1.0],
        fail_graph: false,
        events: vec![],
    }
}
impl SlidingSource for Source {
    type Error = &'static str;
    fn resolve_interfaces_and_sliding_state(&mut self) -> Result<(), Self::Error> {
        self.events.push("resolve");
        Ok(())
    }
    fn intent(&mut self, side: Side) -> Result<Option<f32>, Self::Error> {
        self.events.push(match side {
            Side::Right => "right",
            Side::Left => "left",
        });
        Ok(self.inputs[side.index()])
    }
    fn is_ground(&mut self) -> Result<bool, Self::Error> {
        self.events.push("ground");
        Ok(self.ground)
    }
    fn missing_instance(&mut self) -> Self::Error {
        "instance"
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.events.push("speed");
        Ok(self.speed)
    }
    fn speed_limit(&mut self) -> Result<f32, Self::Error> {
        self.events.push("limit");
        Ok(1.0)
    }
    fn start_intent(&mut self, side: Side) -> Result<bool, Self::Error> {
        Ok(self.starts[side.index()])
    }
    fn write_start(&mut self, side: Side, value: bool) -> Result<(), Self::Error> {
        self.start_output[side.index()] = value;
        Ok(())
    }
    fn debug(&mut self, calls: usize) -> Result<(), Self::Error> {
        self.events
            .push(if calls == 2 { "debug2" } else { "debug3" });
        Ok(())
    }
    fn heading_cosine(&mut self) -> Result<f32, Self::Error> {
        self.events.push("heading");
        Ok(1.0)
    }
    fn graph(&mut self, offset: usize, _value: f32) -> Result<f32, Self::Error> {
        let i = match offset {
            0x4a0 => {
                self.events.push("speedgraph");
                0
            }
            0x500 => {
                self.events.push("headinggraph");
                1
            }
            0x4d0 => {
                self.events.push("durationgraph");
                2
            }
            _ => panic!("graph"),
        };
        if self.fail_graph {
            Err("graph")
        } else {
            Ok(self.graph_result[i])
        }
    }
    fn dt(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(0.25)
    }
    fn minimum_time(&mut self) -> Result<f32, Self::Error> {
        self.events.push("minimum");
        Ok(0.1)
    }
    fn increment_time(&mut self, side: Side, dt: f32) -> Result<f32, Self::Error> {
        self.times[side.index()] += dt;
        Ok(self.times[side.index()])
    }
    fn write_end(&mut self, side: Side, value: bool) -> Result<(), Self::Error> {
        self.end_output[side.index()] = value;
        Ok(())
    }
}
#[test]
fn neutral_ground_consumes_live_graphs_advances_both_timers_and_sets_end_flags() {
    let mut s = source();
    let mut i = SlidingInstance::after_allocation(true).unwrap();
    update(&mut s, Some(&mut i)).unwrap();
    assert_eq!(s.times, [0.25; 2]);
    assert_eq!(s.start_output, [false; 2]);
    assert_eq!(s.end_output, [true; 2]);
    assert_eq!(
        s.events,
        [
            "resolve",
            "right",
            "left",
            "ground",
            "speed",
            "limit",
            "debug2",
            "heading",
            "speedgraph",
            "headinggraph",
            "durationgraph",
            "minimum",
            "dt",
            "dt",
            "debug3"
        ]
    );
}
#[test]
fn first_ground_suppresses_start_and_late_graph_failure_preserves_history_and_stores() {
    let mut s = source();
    s.inputs = [Some(-1.0), Some(1.0)];
    s.starts = [true; 2];
    s.fail_graph = true;
    let mut i = SlidingInstance::after_allocation(true).unwrap();
    assert_eq!(update(&mut s, Some(&mut i)), Err("graph"));
    assert!(i.was_ground && i.right_started && i.left_started);
    assert_eq!((i.previous_left, i.previous_right), (-1.0, 1.0));
    assert_eq!(s.start_output, [false; 2]);
    assert_eq!(s.times, [0.0; 2]);
    i.right_started = false;
    i.left_started = false;
    s.inputs = [Some(-2.0), Some(2.0)];
    assert_eq!(update(&mut s, Some(&mut i)), Err("graph"));
    assert_eq!(s.start_output, [true; 2]);
}
#[test]
fn unordered_threshold_resets_heading_clock_and_unordered_slide_clock_does_not_end() {
    let mut s = source();
    s.times = [f32::NAN; 2];
    s.graph_result[1] = f32::NAN;
    let mut i = SlidingInstance::after_allocation(true).unwrap();
    i.was_ground = true;
    i.heading_time = 7.0;
    update(&mut s, Some(&mut i)).unwrap();
    assert_eq!(i.heading_time.to_bits(), 0);
    assert_eq!(s.end_output, [false; 2]);
}
#[test]
fn nonground_keeps_history_except_was_ground_and_skips_all_tuning() {
    let mut s = source();
    s.ground = false;
    let mut i = SlidingInstance::after_allocation(true).unwrap();
    i.was_ground = true;
    i.heading_time = 3.0;
    i.previous_right = 2.0;
    update(&mut s, Some(&mut i)).unwrap();
    assert!(!i.was_ground);
    assert_eq!(i.heading_time, 3.0);
    assert_eq!(i.previous_right, 2.0);
    assert_eq!(s.events, ["resolve", "right", "left", "ground"]);
    assert!(SlidingInstance::after_allocation(false).is_none());
}
