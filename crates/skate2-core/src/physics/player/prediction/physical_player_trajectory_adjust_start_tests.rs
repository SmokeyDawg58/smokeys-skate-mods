use crate::physics_state_ids::{PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::reckoning_numeric::{HostReckoningNumeric, host_dot3};
struct Source {
    normal: Vector,
    blend: f32,
    ground_limit: f32,
    velocity_limit: f32,
    transition: f32,
    state: i32,
    overlay_fails: bool,
    fail_key: Option<u64>,
    calls: Vec<&'static str>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            normal: Vector::known([1.0, 0.0, 0.0, 0.0]),
            blend: 1.0,
            ground_limit: 0.5,
            velocity_limit: 0.5,
            transition: 0.25,
            state: PHYSICS_STATE_PHYSICS_AIR,
            overlay_fails: false,
            fail_key: None,
            calls: Vec::new(),
        }
    }
}
impl AdjustSource for Source {
    type Error = &'static str;
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.calls.push("normal");
        Ok(self.normal)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let (name, value) = match key {
            BLEND => ("blend", self.blend),
            GROUND_Y_LIMIT => ("ground_limit", self.ground_limit),
            VELOCITY_Y_LIMIT => ("velocity_limit", self.velocity_limit),
            TRANSITION_SCALE => ("scale", 2.0),
            _ => panic!("wrong key"),
        };
        self.calls.push(name);
        if self.fail_key == Some(key) {
            Err("attribute")
        } else {
            Ok(value)
        }
    }
    fn transition(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("transition");
        Ok(self.transition)
    }
    fn physics_state(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("state");
        Ok(self.state)
    }
    fn trajectory_scalar(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("trajectory");
        Ok(0.5)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.calls.push("overlay");
        if self.overlay_fails {
            Err("overlay")
        } else {
            Ok(())
        }
    }
}
#[test]
fn flat_normal_still_reads_all_four_attributes_and_keeps_velocity_bits() {
    let mut source = Source {
        normal: Vector::known([0.0, 1.0, 0.0, 0.0]),
        ..Source::default()
    };
    let mut velocity = Vector {
        xyz: [-0.0, 3.0, 4.0],
        w: None,
    };
    let before = velocity;
    let mut on_vert = true;
    adjust(
        &mut on_vert,
        &mut velocity,
        false,
        &mut source,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert!(!on_vert);
    assert_eq!(velocity.xyz.map(f32::to_bits), before.xyz.map(f32::to_bits));
    assert_eq!(velocity.w, None);
    assert_eq!(
        source.calls,
        [
            "normal",
            "blend",
            "ground_limit",
            "velocity_limit",
            "normal",
            "transition",
            "scale",
            "normal"
        ]
    );
}
#[test]
fn full_vert_blend_preserves_speed_and_unknown_lane() {
    let mut source = Source {
        normal: Vector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
        ..Source::default()
    };
    let mut velocity = Vector::known([3.0, 4.0, 0.0, 0.0]);
    let mut on_vert = false;
    adjust(
        &mut on_vert,
        &mut velocity,
        false,
        &mut source,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert!(on_vert);
    assert!(velocity.xyz[0].abs() < 0.000_01);
    assert!((velocity.xyz[1] - 5.0).abs() < 0.000_01);
    assert!(velocity.xyz[2].abs() < 0.000_01);
    assert_eq!(velocity.w, None);
    assert!(!source.calls.contains(&"state"));
}

#[test]
fn partial_blend_uses_original_unit_direction_not_original_speed_vector() {
    let mut source = Source {
        blend: 0.5,
        ..Source::default()
    };
    let mut velocity = Vector::known([3.0, 4.0, 0.0, 0.0]);
    let mut on_vert = false;
    adjust(
        &mut on_vert,
        &mut velocity,
        false,
        &mut source,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert!(on_vert);
    // Source mix is [.3, 2.9, 0], not [1.5, 4.5, 0]. Final scale preserves ratio.
    assert!((velocity.xyz[0] / velocity.xyz[1] - 3.0 / 29.0).abs() < 0.000_01);
}

#[test]
fn player_jump_short_circuits_non_vert_state_and_overlay_reads() {
    let mut source = Source {
        state: PHYSICS_STATE_PHYSICS_GROUND,
        ..Source::default()
    };
    let mut velocity = Vector::known([2.0, -3.0, 4.0, 0.0]);
    let before = velocity;
    let mut on_vert = false;
    adjust(
        &mut on_vert,
        &mut velocity,
        true,
        &mut source,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(velocity, before);
    assert!(!source.calls.contains(&"state"));
    assert!(!source.calls.contains(&"overlay"));
}
#[test]
fn negative_vertical_air_skips_non_air_velocity_edit_and_overlay() {
    let mut source = Source::default();
    let mut velocity = Vector::known([2.0, -3.0, 4.0, 7.0]);
    let before = velocity;
    let mut on_vert = true;
    adjust(
        &mut on_vert,
        &mut velocity,
        false,
        &mut source,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert!(!on_vert);
    assert_eq!(velocity, before);
    assert_eq!(source.calls.last(), Some(&"state"));
}
#[test]
fn non_air_tail_store_survives_unconditional_overlay_failure() {
    let mut source = Source {
        state: PHYSICS_STATE_PHYSICS_GROUND,
        overlay_fails: true,
        ..Source::default()
    };
    let mut velocity = Vector::known([2.0, -3.0, 4.0, 0.0]);
    let mut on_vert = true;
    assert!(matches!(
        adjust(
            &mut on_vert,
            &mut velocity,
            false,
            &mut source,
            &mut HostReckoningNumeric
        ),
        Err(AdjustStartError::Source("overlay"))
    ));
    assert!(!on_vert);
    assert_eq!(
        velocity.xyz.map(f32::to_bits),
        [1.0_f32, -3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(
        &source.calls[source.calls.len() - 4..],
        ["state", "normal", "trajectory", "overlay"]
    );
}
#[test]
fn attribute_failure_keeps_initial_on_vert_clear_and_original_velocity() {
    let mut source = Source {
        fail_key: Some(GROUND_Y_LIMIT),
        ..Source::default()
    };
    let mut velocity = Vector::known([1.0, 2.0, 3.0, 4.0]);
    let before = velocity;
    let mut on_vert = true;
    assert!(matches!(
        adjust(
            &mut on_vert,
            &mut velocity,
            false,
            &mut source,
            &mut HostReckoningNumeric
        ),
        Err(AdjustStartError::Source("attribute"))
    ));
    assert!(!on_vert);
    assert_eq!(velocity, before);
    assert_eq!(source.calls, ["normal", "blend", "ground_limit"]);
}
struct FailSixthDot(usize);
impl ReckoningNumeric for FailSixthDot {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.0 += 1;
        if self.0 == 6 {
            Err("dot")
        } else {
            Ok(host_dot3(a, b))
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("unused")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("unused")
    }
}
#[test]
fn late_numeric_failure_retains_on_vert_but_does_not_publish_partial_velocity() {
    let mut source = Source::default();
    let mut velocity = Vector::known([3.0, 4.0, 0.0, 0.0]);
    let before = velocity;
    let mut on_vert = false;
    assert!(matches!(
        adjust(
            &mut on_vert,
            &mut velocity,
            false,
            &mut source,
            &mut FailSixthDot(0)
        ),
        Err(AdjustStartError::Numeric("dot"))
    ));
    assert!(on_vert);
    assert_eq!(velocity, before);
}
#[test]
fn fsel_transition_clamp_includes_unordered_and_endpoint_semantics() {
    for (input, expected) in [
        (-2.0_f32, -1.0_f32),
        (-0.75, -1.0),
        (0.25, 0.0),
        (1.25, 1.0),
        (2.0, 1.0),
        (f32::NAN, 1.0),
    ] {
        assert_eq!(transition_factor(input).to_bits(), expected.to_bits());
    }
}
