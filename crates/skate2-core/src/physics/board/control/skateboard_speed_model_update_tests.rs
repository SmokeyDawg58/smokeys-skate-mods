//! Controlled source-order/numeric discriminators; no authored-frame claim.
use super::{
    SkateboardEntryState, SpeedCvar, SpeedGlobal, SpeedGraph, SpeedModelBeforeJumpContribution,
    SpeedModelSource, SpeedScalar, SpeedVector,
};
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
struct Source {
    events: Vec<String>,
    manual: bool,
    balance: f32,
    idle: bool,
    friction: f32,
    cheat: bool,
    fail: Option<SpeedCvar>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            manual: false,
            balance: 0.0,
            idle: false,
            friction: 0.0,
            cheat: false,
            fail: None,
        }
    }
}
impl SpeedModelSource for Source {
    type Error = &'static str;
    type Pointer = (SpeedGlobal, u64);
    type Surface = ();
    type Graph = SpeedGraph;
    type Cvar = SpeedCvar;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.events.push("flags8c8".into());
        Ok(0)
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.events.push("flags8cc".into());
        Ok(0x0100_0000)
    }
    fn attribute(&mut self, g: SpeedGlobal, k: u64) -> Result<Self::Pointer, Self::Error> {
        self.events.push(format!("attribute {g:?} {k:x}"));
        Ok((g, k))
    }
    fn value(&mut self, p: &Self::Pointer) -> Result<f32, Self::Error> {
        self.events.push(format!("value {:x}", p.1));
        Ok(match p.1 {
            0xf4eb_6567_aba4_d0f9 => 10.0,
            0xf728_404c_f0e1_c5f1 | 0x5c50_5baa_91bb_f3c6 => 1.0,
            _ => 0.0,
        })
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.events.push("effective".into());
        Ok(SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]))
    }
    fn vector(&mut self, v: SpeedVector) -> Result<SkeletonRootVector, Self::Error> {
        self.events.push(format!("vector {v:?}"));
        Ok(SkeletonRootVector::known(match v {
            SpeedVector::GroundNormal => [0.0, 1.0, 0.0, 0.0],
            SpeedVector::SurfaceVelocity => [0.0, 0.0, 5.0, 0.0],
            _ => [0.0, 0.0, 1.0, 0.0],
        }))
    }
    fn scalar(&mut self, s: SpeedScalar) -> Result<f32, Self::Error> {
        self.events.push(format!("scalar {s:?}"));
        Ok(match s {
            SpeedScalar::Balance => self.balance,
            SpeedScalar::TimeSinceInput => f32::from(u8::from(self.idle)),
            _ => 0.0,
        })
    }
    fn frames_spinning(&mut self) -> Result<i32, Self::Error> {
        self.events.push("frames".into());
        Ok(i32::from(!self.manual))
    }
    fn capture_cvar(&mut self, c: SpeedCvar) -> Result<Self::Cvar, Self::Error> {
        self.events.push(format!("capture {c:?}"));
        if self.fail == Some(c) {
            Err("cvar")
        } else {
            Ok(c)
        }
    }
    fn cvar_int(&mut self, c: &Self::Cvar) -> Result<i32, Self::Error> {
        self.events.push(format!("int {c:?}"));
        Ok(1)
    }
    fn cvar_bool(&mut self, c: &Self::Cvar) -> Result<bool, Self::Error> {
        self.events.push(format!("bool {c:?}"));
        Ok(*c == SpeedCvar::CheatEnabled && self.cheat)
    }
    fn cvar_float(&mut self, c: &Self::Cvar) -> Result<f32, Self::Error> {
        self.events.push(format!("float {c:?}"));
        Ok(36.0)
    }
    fn capture_surface(&mut self) -> Result<(), Self::Error> {
        self.events.push("surface".into());
        Ok(())
    }
    fn surface_graph(&mut self, (): &(), _: f32) -> Result<f32, Self::Error> {
        self.events.push("surface graph".into());
        Ok(0.0)
    }
    fn capture_graph(&mut self, g: SpeedGraph) -> Result<Self::Graph, Self::Error> {
        self.events.push(format!("graph {g:?}"));
        Ok(g)
    }
    fn evaluate_graph(&mut self, _: &SpeedGraph, _: f32) -> Result<f32, Self::Error> {
        self.events.push("evaluate".into());
        Ok(self.friction)
    }
    fn body_mass(&mut self) -> Result<f32, Self::Error> {
        self.events.push("mass".into());
        Ok(10.0)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.push("overlay".into());
        Ok(())
    }
}
fn prefix() -> SpeedModelBeforeJumpContribution {
    SpeedModelBeforeJumpContribution {
        should_apply_force: true,
        absolute_forward_speed: 5.0,
        time_delta: 0.25,
        maximum_difference: 2.0,
        starting_target: 6.0,
    }
}
fn output() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: SkeletonRootVector::known([99.0; 4]),
        local_space_position: SkeletonRootVector::known([88.0; 4]),
    }
}
#[test]
fn ordinary_force_updates_same_target_and_preserves_late_source_order() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source::default();
    let mut out = output();
    state
        .try_speed_model_update(prefix(), &mut s, &mut HostReckoningNumeric, &mut out)
        .unwrap();
    assert_eq!(state.target_speed.to_bits(), 6.0_f32.to_bits());
    assert!((out.world_space_force.xyz[2] - 40.0).abs() < 0.000_1);
    let c = s
        .events
        .iter()
        .position(|v| v == "capture ManualFrames")
        .unwrap();
    assert_eq!(
        &s.events[c..c + 3],
        ["capture ManualFrames", "frames", "int ManualFrames"]
    );
    assert_eq!(s.events.last().unwrap(), "bool Debug");
}
#[test]
fn false_permission_still_runs_model_and_actual_mass_without_coefficient_lookup() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source::default();
    let mut out = output();
    let mut p = prefix();
    p.should_apply_force = false;
    state
        .try_speed_model_update(p, &mut s, &mut HostReckoningNumeric, &mut out)
        .unwrap();
    assert_eq!(state.target_speed.to_bits(), 6.0_f32.to_bits());
    assert!(s.events.iter().any(|e| e == "mass"));
    assert!(!s.events.iter().any(|e| e.contains("f728404cf0e1c5f1")));
}
#[test]
fn source_target_and_force_partial_writes_survive_distinct_late_errors() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source {
        fail: Some(SpeedCvar::CheatEnabled),
        ..Source::default()
    };
    let mut out = output();
    assert!(
        state
            .try_speed_model_update(prefix(), &mut s, &mut HostReckoningNumeric, &mut out)
            .is_err()
    );
    assert_eq!(state.target_speed.to_bits(), 6.0_f32.to_bits());
    assert_eq!(out.world_space_force.xyz[2].to_bits(), 99.0_f32.to_bits());
    s.fail = Some(SpeedCvar::Debug);
    assert!(
        state
            .try_speed_model_update(prefix(), &mut s, &mut HostReckoningNumeric, &mut out)
            .is_err()
    );
    assert!((out.world_space_force.xyz[2] - 40.0).abs() < 0.000_1);
}
#[test]
fn zero_manual_torque_does_not_skip_real_manual_reads() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source {
        manual: true,
        ..Source::default()
    };
    let mut out = output();
    state
        .try_speed_model_update(prefix(), &mut s, &mut HostReckoningNumeric, &mut out)
        .unwrap();
    assert!(s.events.iter().any(|e| e == "scalar Spin"));
    assert!(s.events.iter().any(|e| e.contains("b52d8ff06890b800")));
}
#[test]
fn fused_negative_subtraction_preserves_source_negative_zero_target() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source {
        idle: true,
        friction: 1.0,
        ..Source::default()
    };
    let mut out = output();
    let mut p = prefix();
    p.time_delta = 1.0;
    p.starting_target = 1.0;
    p.absolute_forward_speed = 0.0;
    state
        .try_speed_model_update(p, &mut s, &mut HostReckoningNumeric, &mut out)
        .unwrap();
    assert_eq!(state.target_speed.to_bits(), (-0.0_f32).to_bits());
}
#[test]
fn live_cheat_changes_force_delta_without_rewriting_target() {
    let mut state = SkateboardEntryState::new();
    let mut s = Source {
        cheat: true,
        ..Source::default()
    };
    let mut out = output();
    state
        .try_speed_model_update(prefix(), &mut s, &mut HostReckoningNumeric, &mut out)
        .unwrap();
    assert_eq!(state.target_speed.to_bits(), 6.0_f32.to_bits());
    assert!((out.world_space_force.xyz[2] - 16.0).abs() < 0.000_1);
    assert!(s.events.iter().any(|e| e == "float CheatSpeed"));
}
