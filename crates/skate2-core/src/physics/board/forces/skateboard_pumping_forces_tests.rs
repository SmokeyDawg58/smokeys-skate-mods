//! Controlled source-boundary tests, not an authored skating frame.
use super::{
    AbsorptionSource, ENABLED, NO_INTENTION_SCALE, PumpForceSource, try_absorption, try_pump,
};
use crate::{
    reckoning_numeric::HostReckoningNumeric, skateboard_foot_forces::SkateboardFootForce,
    skeleton_root_transforms::SkeletonRootVector,
};
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source branch test controls"
)]
struct Source {
    crouch: f32,
    delta: f32,
    absorption: f32,
    scale: f32,
    intended: bool,
    acceleration: f32,
    mass: f32,
    dt: f32,
    enabled: bool,
    overlay: bool,
    draw: bool,
    missing_velocity: bool,
    velocity: SkeletonRootVector,
    events: Vec<&'static str>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            crouch: 0.0,
            delta: 1.0,
            absorption: 3.0,
            scale: 2.0,
            intended: true,
            acceleration: 3.0,
            mass: 2.0,
            dt: 0.5,
            enabled: true,
            overlay: true,
            draw: false,
            missing_velocity: false,
            velocity: SkeletonRootVector {
                xyz: [0.0, 0.0, 1.0],
                w: None,
            },
            events: Vec::new(),
        }
    }
}
impl AbsorptionSource for Source {
    type Error = &'static str;
    type Pointer = ();
    fn crouch(&mut self) -> Result<f32, Self::Error> {
        self.events.push("crouch");
        Ok(self.crouch)
    }
    fn crouch_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("delta");
        Ok(self.delta)
    }
    fn capture_scale(&mut self) -> Result<(), Self::Error> {
        self.events.push("capture feet");
        Ok(())
    }
    fn load_scale(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.events.push("load feet");
        Ok(self.scale)
    }
    fn absorption(&mut self) -> Result<f32, Self::Error> {
        self.events.push("absorption");
        Ok(self.absorption)
    }
    fn draw_absorption(&mut self) -> Result<bool, Self::Error> {
        self.events.push("draw absorption");
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        overlay(self)
    }
}
fn overlay(source: &mut Source) -> Result<(), &'static str> {
    source.events.push("overlay");
    if source.overlay {
        Ok(())
    } else {
        Err("overlay")
    }
}
impl PumpForceSource for Source {
    type Error = &'static str;
    type Pointer = u64;
    fn player_controlled_pump(&mut self) -> Result<bool, Self::Error> {
        self.events.push("intended");
        Ok(self.intended)
    }
    fn acceleration(&mut self) -> Result<f32, Self::Error> {
        self.events.push("acceleration");
        Ok(self.acceleration)
    }
    fn draw_intention(&mut self) -> Result<bool, Self::Error> {
        self.events.push("draw intention");
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        overlay(self)
    }
    fn capture(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.events.push(if key == ENABLED {
            "capture enabled"
        } else {
            "capture intention"
        });
        Ok(key)
    }
    fn load_scalar(&mut self, pointer: &u64) -> Result<f32, Self::Error> {
        assert_eq!(*pointer, NO_INTENTION_SCALE);
        self.events.push("load intention");
        Ok(self.scale)
    }
    fn load_enabled(&mut self, pointer: &u64) -> Result<bool, Self::Error> {
        assert_eq!(*pointer, ENABLED);
        self.events.push("load enabled");
        Ok(self.enabled)
    }
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.events.push("mass");
        Ok(self.mass)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(self.dt)
    }
    fn z_axis_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.events.push("velocity");
        if self.missing_velocity {
            Err("velocity")
        } else {
            Ok(self.velocity)
        }
    }
}
fn old_output() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: SkeletonRootVector::known([7.0; 4]),
        local_space_position: SkeletonRootVector::known([8.0; 4]),
    }
}
#[test]
fn absorption_zero_crouch_skips_delta_and_inactive_requires_overlay_without_cvar() {
    let mut source = Source::default();
    assert_eq!(
        try_absorption(&mut source).unwrap().to_bits(),
        6.0_f32.to_bits()
    );
    assert_eq!(
        source.events,
        [
            "crouch",
            "capture feet",
            "load feet",
            "absorption",
            "draw absorption"
        ]
    );
    source.crouch = 1.0;
    source.overlay = false;
    source.events.clear();
    assert_eq!(try_absorption(&mut source), Err("overlay"));
    assert_eq!(source.events, ["crouch", "delta", "overlay"]);
}
#[test]
fn absorption_negative_delta_and_nan_follow_source_selection() {
    let mut source = Source {
        crouch: f32::NAN,
        delta: -1.0,
        absorption: -3.0,
        ..Source::default()
    };
    assert_eq!(try_absorption(&mut source).unwrap().to_bits(), 0);
    source.absorption = f32::NAN;
    assert!(try_absorption(&mut source).unwrap().is_nan());
    source.delta = f32::NAN;
    source.events.clear();
    assert_eq!(try_absorption(&mut source).unwrap().to_bits(), 0);
    assert_eq!(source.events, ["crouch", "delta", "overlay"]);
}
#[test]
fn pump_reads_acceleration_before_optional_scale_and_uses_same_math() {
    let mut source = Source {
        intended: false,
        ..Source::default()
    };
    let mut output = old_output();
    try_pump(&mut source, &mut HostReckoningNumeric, &mut output).unwrap();
    assert_eq!(
        source.events,
        [
            "intended",
            "acceleration",
            "draw intention",
            "capture intention",
            "load intention",
            "mass",
            "dt",
            "capture enabled",
            "load enabled",
            "velocity"
        ]
    );
    assert!((output.world_space_force.xyz[2] - 24.0).abs() < 0.000_1);
    assert_eq!(output.world_space_force.w, None);
    assert_eq!(output.local_space_position.w.map(f32::to_bits), Some(0));
}
#[test]
fn disabled_pump_still_reads_and_normalizes_source_vector() {
    let mut source = Source {
        enabled: false,
        missing_velocity: true,
        ..Source::default()
    };
    let mut output = old_output();
    assert!(try_pump(&mut source, &mut HostReckoningNumeric, &mut output).is_err());
    assert_eq!(source.events.last(), Some(&"velocity"));
    assert_eq!(output.world_space_force.xyz.map(f32::to_bits), [0; 3]);
    source.missing_velocity = false;
    try_pump(&mut source, &mut HostReckoningNumeric, &mut output).unwrap();
    assert_eq!(output.world_space_force.w, None);
    source.velocity = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    try_pump(&mut source, &mut HostReckoningNumeric, &mut output).unwrap();
    assert_eq!(output.world_space_force.w.map(f32::to_bits), Some(0));
}
#[test]
fn missing_early_debug_preserves_old_output_and_skips_attributes() {
    let mut source = Source {
        intended: false,
        draw: true,
        overlay: false,
        ..Source::default()
    };
    let mut output = old_output();
    assert!(try_pump(&mut source, &mut HostReckoningNumeric, &mut output).is_err());
    assert_eq!(
        source.events,
        ["intended", "acceleration", "draw intention", "overlay"]
    );
    assert_eq!(
        output.world_space_force.xyz.map(f32::to_bits),
        [7.0_f32; 3].map(f32::to_bits)
    );
}
