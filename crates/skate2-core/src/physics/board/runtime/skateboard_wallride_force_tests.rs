//! Controlled boundary tests; no claim that these inputs came from a game frame.
use super::{
    SkateboardFootForce, SkeletonRootVector, WallRideHit, WallRideScalar, WallRideSource,
    WallRideVector, ZERO, try_calculate,
};
use crate::reckoning_numeric::HostReckoningNumeric;
struct Source {
    normal: SkeletonRootVector,
    jump: bool,
    height: f32,
    velocity: SkeletonRootVector,
    queries: usize,
    fail_jump: bool,
    captures: Vec<u64>,
}
impl Source {
    fn active() -> Self {
        Self {
            normal: SkeletonRootVector {
                xyz: [1.0, 0.0, 0.0],
                w: None,
            },
            jump: true,
            height: 9.0,
            velocity: SkeletonRootVector::known([8.0; 4]),
            queries: 0,
            fail_jump: false,
            captures: vec![],
        }
    }
}
impl WallRideSource for Source {
    type Error = &'static str;
    type Scalar = u64;
    type TrajectoryLayout = ();
    type DebugTarget = ();
    fn capture_feet(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.captures.push(key);
        if self.fail_jump && key == 0xfd00_fa38_6f0a_c466 {
            return Err("jump coefficient");
        }
        Ok(key)
    }
    fn load_feet(&mut self, key: &u64) -> Result<f32, Self::Error> {
        Ok(match key {
            0xbaba_9160_6102_af9a | 0x90c4_e79c_bfba_ed5e | 0xc2bc_e9a7_d58b_e7bb => 0.5,
            0x19c8_04c3_e3e2_b566 => 0.0,
            0xd91a_09c7_e923_7430 => 4.0,
            0x843a_276a_c05b_4a8b => 2.0,
            0x569d_f113_0ffd_dbb6 => 8.0,
            0x7291_4fa6_24d2_947b => 0.25,
            0xfd00_fa38_6f0a_c466 => 5.0,
            _ => panic!("unexpected key"),
        })
    }
    fn feet_graph(&mut self, _: f32) -> Result<f32, Self::Error> {
        Ok(0.5)
    }
    fn capture_trajectory_layout(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn trajectory_height(&mut self, (): &()) -> Result<f32, Self::Error> {
        Ok(0.5)
    }
    fn vector(&mut self, v: WallRideVector) -> Result<SkeletonRootVector, Self::Error> {
        Ok(match v {
            WallRideVector::GroundNormal | WallRideVector::Up => self.normal,
            WallRideVector::Velocity => SkeletonRootVector {
                xyz: [2.0, -1.0, 3.0],
                w: None,
            },
            WallRideVector::DeckPosition => SkeletonRootVector {
                xyz: [0.0, 2.0, 0.0],
                w: None,
            },
        })
    }
    fn scalar(&mut self, v: WallRideScalar) -> Result<f32, Self::Error> {
        Ok(match v {
            WallRideScalar::Mass => 2.0,
            WallRideScalar::Gravity => 10.0,
        })
    }
    fn wheels(&mut self) -> Result<i32, Self::Error> {
        Ok(1)
    }
    fn capture_debug(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn debug_value(&mut self, (): &()) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("disabled")
    }
    fn line(
        &mut self,
        start: SkeletonRootVector,
        end: SkeletonRootVector,
    ) -> Result<Option<WallRideHit>, Self::Error> {
        self.queries += 1;
        assert_eq!(
            start.xyz.map(f32::to_bits),
            [0.5f32, 2.0, 0.0].map(f32::to_bits)
        );
        assert!((end.xyz[1] + 2.0).abs() < 0.000_01);
        assert!(start.w.is_none() && end.w.is_none());
        Ok(Some(WallRideHit {
            position: SkeletonRootVector {
                xyz: [0.5, 1.0, 0.0],
                w: None,
            },
            normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        }))
    }
    fn clear_jump_flag(&mut self) -> Result<(), Self::Error> {
        self.jump = false;
        Ok(())
    }
    fn clear_jump_height(&mut self) -> Result<(), Self::Error> {
        self.height = 0.0;
        Ok(())
    }
    fn publish_jump_flag(&mut self) -> Result<(), Self::Error> {
        self.jump = true;
        Ok(())
    }
    fn publish_jump_velocity(&mut self, v: SkeletonRootVector) -> Result<(), Self::Error> {
        self.velocity = v;
        Ok(())
    }
}
#[test]
fn measured_flat_gate_reads_third_key_clears_only_actual_jump_flag_height() {
    let mut source = Source::active();
    source.normal = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let old = source.velocity;
    let mut force = SkateboardFootForce {
        world_space_force: old,
        local_space_position: old,
    };
    assert!(!try_calculate(&mut source, &mut HostReckoningNumeric, 1.0, 0.0, &mut force).unwrap());
    assert!(!source.jump);
    assert_eq!(source.height.to_bits(), 0);
    assert_eq!(source.velocity, old);
    assert_eq!(source.captures.len(), 3);
    assert_eq!(source.queries, 0);
    assert_eq!(force.world_space_force, ZERO);
    assert_eq!(force.local_space_position, ZERO);
}
#[test]
fn active_hit_publishes_jump_and_force_without_inventing_w() {
    let mut source = Source::active();
    let mut force = SkateboardFootForce {
        world_space_force: ZERO,
        local_space_position: ZERO,
    };
    assert!(try_calculate(&mut source, &mut HostReckoningNumeric, 1.0, 0.0, &mut force).unwrap());
    assert_eq!(source.queries, 1);
    assert!(source.jump);
    assert_eq!(
        source.velocity.xyz.map(f32::to_bits),
        [5.0f32, -0.25, 3.0].map(f32::to_bits)
    );
    assert!(source.velocity.w.is_none());
    assert_eq!(
        force.world_space_force,
        SkeletonRootVector::known([0.0, -10.0, 0.0, 0.0])
    );
}
#[test]
fn late_jump_attribute_error_preserves_flag_but_not_yet_velocity_or_force_store() {
    let mut source = Source::active();
    source.fail_jump = true;
    let old = source.velocity;
    let mut force = SkateboardFootForce {
        world_space_force: ZERO,
        local_space_position: ZERO,
    };
    assert!(try_calculate(&mut source, &mut HostReckoningNumeric, 1.0, 0.0, &mut force).is_err());
    assert!(source.jump);
    assert_eq!(source.height.to_bits(), 0);
    assert_eq!(source.velocity, old);
    assert_eq!(force.world_space_force, ZERO);
}
