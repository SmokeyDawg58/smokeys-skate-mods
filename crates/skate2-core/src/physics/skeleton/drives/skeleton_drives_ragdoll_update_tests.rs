#![expect(
    clippy::float_cmp,
    reason = "exact native publications and preserved drive payloads"
)]
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    skateboard_wheels::{FRESH_VOLUME_TRANSFORM, HostSkateboardWheelMassNumeric},
    skeleton_drives::{HostSkeletonDriveNumeric, SkeletonDriveAttributeRead, SkeletonDriveCvars},
};
use std::sync::Arc;

#[derive(Default)]
struct Services {
    events: Vec<String>,
    follow: bool,
    rigid: bool,
    damping_reads: u8,
    fail_second: bool,
    fail_key: Option<u64>,
}
impl SkeletonDrivesUpdateAttributeRead for Services {
    type Error = &'static str;
    type Pointer = u64;
    type Layout = ();
    fn lookup_global(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.events.push(format!("animation:{key:x}"));
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.events.push(format!("float:{key:x}"));
        if self.fail_key == Some(*key) {
            Err("attribute")
        } else {
            Ok(1.0)
        }
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("ragdoll does not read normal follow attribute")
    }
    fn capture_animation_layout(&mut self, _: SkeletonDriveType) -> Result<(), Self::Error> {
        self.events.push("layout".into());
        Ok(())
    }
    fn animation_float(&mut self, (): &(), offset: usize) -> Result<f32, Self::Error> {
        self.events.push(format!("layout:{offset}"));
        Ok([2.0, 3.0, 5.0, 7.0][offset / 4])
    }
}
impl RagdollDriveAttributes for Services {
    fn lookup_skeleton_drive(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.events.push(format!("skeleton:{key:x}"));
        Ok(key)
    }
}
impl SkeletonDrivesUpdateCvars for Services {
    type Error = &'static str;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        panic!("no offboard scalar")
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        self.damping_reads += 1;
        self.events.push("damping".into());
        if self.fail_second && self.damping_reads == 2 {
            Err("second damping")
        } else {
            Ok(f32::from(self.damping_reads))
        }
    }
}
impl RagdollDriveCvars for Services {
    fn follow_animation(&mut self) -> Result<bool, Self::Error> {
        self.events.push("follow".into());
        Ok(self.follow)
    }
    fn rigid_ragdoll(&mut self) -> Result<bool, Self::Error> {
        self.events.push("rigid".into());
        Ok(self.rigid)
    }
}
impl SkeletonDriveAttributeRead for Services {
    type Pointer = u64;
    type Error = &'static str;
    fn find_and_change_animation_drive(&mut self, _: SkeletonDriveType) -> Result<(), Self::Error> {
        Ok(())
    }
    fn lookup_global(&mut self, key: u64) -> Result<u64, Self::Error> {
        Ok(key)
    }
    fn float(&mut self, _: &u64) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
    fn animation_float(&mut self, _: SkeletonDriveType, _: usize) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
}
impl SkeletonDriveCvars for Services {
    type Error = &'static str;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
}
fn bone() -> SkeletonBoneDrives {
    let mut bone = SkeletonBoneDrives::fresh();
    bone.flags = 4;
    bone.soft = [10.0, 20.0, 4.0, 8.0];
    bone.dynamics[0].write().unwrap().linear = SimulationDriveDynamicsParams {
        raw_type: 9,
        spring: 11.0,
        damping: 12.0,
        strength: 13.0,
    };
    bone
}
#[test]
fn rigid_preserves_linear_and_follow_skips_rigid_and_damping() {
    let bone = bone();
    let mut attrs = Services::default();
    let mut cvars = Services {
        rigid: true,
        ..Services::default()
    };
    ragdoll_dynamics::<_, _, ()>(
        1,
        SkeletonDriveType::Local,
        &bone,
        0.25,
        &mut attrs,
        &mut cvars,
    )
    .unwrap();
    let d = *bone.dynamics[0].read().unwrap();
    assert_eq!(d.linear.raw_type, 9);
    assert_eq!(d.linear.spring, 11.0);
    assert_eq!(d.angular.spring, 7.0 * DISPLACEMENT_SCALE);
    assert_eq!(d.angular.strength, 5.0 * STRENGTH_SCALE);
    assert_eq!(attrs.events, ["layout", "layout:8", "layout:12"]);
    assert_eq!(cvars.events, ["follow", "rigid"]);
    let mut cvars = Services {
        follow: true,
        ..Services::default()
    };
    attrs.events.clear();
    ragdoll_dynamics::<_, _, ()>(
        1,
        SkeletonDriveType::Local,
        &bone,
        0.25,
        &mut attrs,
        &mut cvars,
    )
    .unwrap();
    assert_eq!(cvars.events, ["follow"]);
    assert_eq!(
        attrs.events,
        ["layout", "layout:8", "layout:12", "layout:4", "layout:0"]
    );
}
#[test]
fn source_gt_branch_and_second_damping_failure_preserve_angular_prefix() {
    for value in [0.5, -1.0, f32::NAN] {
        let bone = bone();
        let mut c = Services::default();
        ragdoll_dynamics::<_, _, ()>(
            1,
            SkeletonDriveType::Local,
            &bone,
            value,
            &mut Services::default(),
            &mut c,
        )
        .unwrap();
        let d = *bone.dynamics[0].read().unwrap();
        assert_eq!(c.damping_reads, 1);
        assert_eq!(d.linear.raw_type, 1);
        assert_eq!(d.linear.strength, 0.0);
    }
    let bone = bone();
    let mut c = Services {
        fail_second: true,
        ..Services::default()
    };
    assert!(matches!(
        ragdoll_dynamics::<_, _, ()>(
            1,
            SkeletonDriveType::Local,
            &bone,
            0.75,
            &mut Services::default(),
            &mut c
        ),
        Err(Error::Cvar("second damping"))
    ));
    let d = *bone.dynamics[0].read().unwrap();
    assert_eq!(d.angular.spring, 3.0);
    assert_eq!(d.angular.damping, 1.0);
    assert_eq!(d.linear.raw_type, 9);
    let mut c = Services::default();
    ragdoll_dynamics::<_, _, ()>(
        1,
        SkeletonDriveType::Local,
        &bone,
        0.75,
        &mut Services::default(),
        &mut c,
    )
    .unwrap();
    let d = *bone.dynamics[0].read().unwrap();
    assert_eq!(d.linear.spring, 1.5);
    assert_eq!(d.linear.damping, 2.0);
    assert_eq!(d.linear.strength, 6.0 * STRENGTH_SCALE);
}
#[test]
fn inverse_handles_non_rigid_scale_and_duplicate_y_w() {
    let mut m = FRESH_VOLUME_TRANSFORM;
    m[0] = 2.0;
    m[5] = 4.0;
    m[10] = 8.0;
    m[12] = 4.0;
    m[13] = 8.0;
    m[14] = 16.0;
    let inv = inverse(&m, &mut HostReckoningNumeric).unwrap();
    assert_eq!([inv[0], inv[5], inv[10]], [0.5, 0.25, 0.125]);
    assert_eq!(
        [inv[12], inv[13], inv[14], inv[15]],
        [-2.0, -2.0, -2.0, -2.0]
    );
    assert_eq!([inv[3], inv[7], inv[11]], [0.0, 0.25, 0.0]);
}
fn owner() -> SkeletonDrives {
    let (h, p, pose) = crate::skeleton_data_tests::fixture();
    let data = Arc::new(crate::skeleton_data_tests::build(h, p, &pose).unwrap());
    SkeletonDrives::try_create(
        data,
        &mut Services::default(),
        &mut HostSkeletonDriveNumeric,
        &mut HostSkateboardWheelMassNumeric,
        &mut Services::default(),
    )
    .unwrap()
}
#[test]
fn live_attribute_failure_retains_residual_and_first_flag_without_later_bones() {
    let mut owner = owner();
    let before = Arc::clone(owner.bones[1].dynamics(SkeletonDriveType::Local));
    let mut a = Services {
        fail_key: Some(ROOT_WEIGHT_KEYS[0]),
        ..Services::default()
    };
    let result = owner.try_update_ragdoll(
        &[FRESH_VOLUME_TRANSFORM; 24],
        &[SkeletonDriveAttributes::constructor_value(); 24],
        RagdollDriveWeights {
            start: 0.25,
            end: 0.125,
            controlled: 0.125,
        },
        &mut a,
        &mut Services::default(),
        &mut HostSkeletonDriveNumeric,
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(RagdollDriveUpdateError::Drive(Error::Attribute(
            "attribute"
        )))
    ));
    assert_eq!(owner.ragdoll_strength, 0.5);
    assert_eq!(owner.bones[1].flags, 4);
    assert_eq!(owner.bones[2].flags, 0);
    assert!(Arc::ptr_eq(
        &before,
        owner.bones[1].dynamics(SkeletonDriveType::Local)
    ));
}

#[test]
fn ragdoll_updates_existing_bone_payloads_but_not_hook_or_row23() {
    let mut owner = owner();
    let before = Arc::clone(owner.bones[1].dynamics(SkeletonDriveType::Local));
    let hook_before = *owner.hook_dynamics[0].read().unwrap();
    let mut coefficient = SkeletonDriveAttributes::constructor_value();
    coefficient.ragdoll_strength_start = 2.0;
    coefficient.ragdoll_strength = 4.0;
    coefficient.ragdoll_strength_controlled = 6.0;
    coefficient.ragdoll_strength_end = 8.0;
    let mut attributes = Services::default();
    owner
        .try_update_ragdoll(
            &[FRESH_VOLUME_TRANSFORM; 24],
            &[coefficient; 24],
            RagdollDriveWeights {
                start: 0.25,
                end: 0.125,
                controlled: 0.125,
            },
            &mut attributes,
            &mut Services::default(),
            &mut HostSkeletonDriveNumeric,
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(owner.bones[1].strengths, [4.25, 4.25]);
    assert_eq!(owner.bones[22].flags, 4);
    assert_eq!(owner.bones[23].flags, 0);
    assert!(Arc::ptr_eq(
        &before,
        owner.bones[1].dynamics(SkeletonDriveType::Local)
    ));
    let hook_after = *owner.hook_dynamics[0].read().unwrap();
    assert_eq!(
        hook_before.linear.spring.to_bits(),
        hook_after.linear.spring.to_bits()
    );
    assert_eq!(hook_before.linear.raw_type, hook_after.linear.raw_type);
    assert_eq!(
        attributes
            .events
            .iter()
            .filter(|e| e.starts_with("skeleton:"))
            .count(),
        22 * 4
    );
}
