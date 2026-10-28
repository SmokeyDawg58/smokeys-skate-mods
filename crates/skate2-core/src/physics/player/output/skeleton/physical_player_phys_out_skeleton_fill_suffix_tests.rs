use super::*;
use crate::{
    physical_player_phys_out::construct::PhysOutAllocation, reckoning_numeric::HostReckoningNumeric,
};

#[derive(Default)]
struct Source {
    calls: Vec<String>,
    effective: usize,
    fail_audio: bool,
    fail_ik: bool,
    want: bool,
    offsets: [f32; 2],
    threshold: f32,
}
fn vector(value: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [value, 0.0, 0.0],
        w: None,
    }
}
fn matrix() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
        vector(0.0),
    ])
}
impl<N: ReckoningNumeric> SkeletonFillSuffixSource<N> for Source {
    type Error = &'static str;
    fn angular_velocity(&mut self, p: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push(format!("omega{p}"));
        Ok(vector(2.0))
    }
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("hook".into());
        Ok(vector(3.0))
    }
    fn proxy_in_collision(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn physics_velocity(&mut self, p: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push(format!("velocity{p}"));
        Ok(vector(4.0))
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.effective += 1;
        self.calls.push("effective".into());
        let mut m = matrix();
        m.set_column(2, vector(if self.effective == 1 { 11.0 } else { 22.0 }));
        Ok(m)
    }
    fn part_transform(&mut self, p: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push(format!("part{p}"));
        Ok(matrix())
    }
    fn fill_ik(&mut self, _: &mut PhysicalPlayerPhysOut, _: &mut N) -> Result<(), Self::Error> {
        self.calls.push("IK".into());
        if self.fail_ik {
            Err("IK pending")
        } else {
            Ok(())
        }
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("up".into());
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn processed_body_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("bodyZ".into());
        Ok(vector(1.0))
    }
    fn compliant_just_collided(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("compliant".into());
        Ok(true)
    }
    fn fill_audio_collision(
        &mut self,
        _: &mut PhysicalPlayerPhysOut,
        _: &mut N,
    ) -> Result<(), Self::Error> {
        self.calls.push("audio".into());
        if self.fail_audio {
            Err("audio pending")
        } else {
            Ok(())
        }
    }
    fn ik_contact_offset(&mut self, i: usize) -> Result<f32, Self::Error> {
        self.calls.push(format!("offset{i}"));
        Ok(self.offsets[i])
    }
    fn feet_compression_threshold(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("threshold".into());
        Ok(self.threshold)
    }
    fn feet_compression_curve(&mut self) -> Result<PointGraph<4>, Self::Error> {
        self.calls.push("curve".into());
        Ok(PointGraph {
            x: [-2.0, -1.0, 0.0, 1.0],
            y: [0.5; 4],
        })
    }
    fn want_respawn(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("want".into());
        Ok(self.want)
    }
    fn clear_want_respawn(&mut self) -> Result<(), Self::Error> {
        self.calls.push("clear".into());
        self.want = false;
        Ok(())
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut o = PhysicalPlayerPhysOut::new();
    o.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    o
}

#[test]
fn exact_parts_fresh_effective_and_same_captured_com_feed_publication() {
    let mut o = output();
    let mut s = Source {
        want: true,
        offsets: [-1.0, 0.0],
        threshold: 0.0,
        ..Source::default()
    };
    o.try_fill_skeleton_suffix(&mut s, vector(1.0), &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        &s.calls[..11],
        &[
            "omega23",
            "omega20",
            "omega16",
            "hook",
            "velocity8",
            "velocity4",
            "velocity17",
            "velocity21",
            "effective",
            "effective",
            "part23"
        ]
    );
    let p = o.try_skeleton_output().unwrap();
    assert_eq!(p.average_left_leg_speed_local.to_bits(), 3.0f32.to_bits());
    assert_eq!(p.effective_anim_z.xyz[0].to_bits(), 11.0f32.to_bits());
    assert_eq!(
        p.effective_skeleton_root_matrix.columns()[2].xyz[0].to_bits(),
        22.0f32.to_bits()
    );
    assert!(p.skeleton_torso_angular_vel.w.is_none());
    assert_eq!(p.feet_ik_compression.to_bits(), 0.5f32.to_bits());
    assert!(s.calls.windows(2).any(|v| v == ["part6", "part10"]));
    assert!(
        o.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    assert!(!s.want);
}
#[test]
fn mandatory_audio_failure_keeps_prior_publication_and_never_reads_compression() {
    let mut o = output();
    let mut s = Source {
        fail_audio: true,
        ..Source::default()
    };
    assert!(matches!(
        o.try_fill_skeleton_suffix(&mut s, vector(0.0), &mut HostReckoningNumeric),
        Err(SkeletonFillSuffixError::Source("audio pending"))
    ));
    assert_eq!(s.calls.last().unwrap(), "audio");
    assert!(o.concrete.collision.as_ref().unwrap().skeleton_in_contact);
    assert!(!s.calls.iter().any(|v| v == "offset0"));
}
#[test]
fn published_nan_assertion_is_late_and_prevents_ik() {
    let mut o = output();
    o.concrete.skeleton.as_mut().unwrap().skeleton_hips_pos.xyz[1] = f32::NAN;
    let mut s = Source::default();
    assert!(matches!(
        o.try_fill_skeleton_suffix(&mut s, vector(0.0), &mut HostReckoningNumeric),
        Err(SkeletonFillSuffixError::InvalidPublishedLane {
            offset: 0x40,
            lane: 1
        })
    ));
    assert_eq!(s.effective, 2);
    assert!(!s.calls.iter().any(|v| v == "IK"));
}
#[test]
fn zero_length_select_runs_estimate_but_restores_positive_zero() {
    assert_eq!(length(0.0, f32::INFINITY).to_bits(), 0);
    assert_eq!(length(-0.0, f32::NEG_INFINITY).to_bits(), 0);
}
#[test]
fn compression_nan_preserves_fsel_and_ordered_gate() {
    let mut o = output();
    let mut s = Source {
        offsets: [f32::NAN, -1.0],
        threshold: 0.0,
        ..Source::default()
    };
    o.try_fill_skeleton_suffix(&mut s, vector(0.0), &mut HostReckoningNumeric)
        .unwrap();
    assert!(!s.calls.iter().any(|v| v == "curve"));
    assert_eq!(
        o.try_skeleton_output()
            .unwrap()
            .feet_ik_compression
            .to_bits(),
        0
    );
}

#[test]
fn contact_guard_is_sampled_before_byte_but_checked_after_byte() {
    let mut missing = PhysicalPlayerPhysOut::new();
    let mut source = Source::default();
    assert!(matches!(
        missing.try_publish_skeleton_contact::<_, HostReckoningNumeric>(&mut source),
        Err(SkeletonFillSuffixError::Access(
            PhysOutAccessError::NotConstructed
        ))
    ));
    assert!(source.calls.is_empty());

    let mut locked = output();
    locked.lock_after_update1().unwrap();
    assert!(matches!(
        locked.try_publish_skeleton_contact::<_, HostReckoningNumeric>(&mut source),
        Err(SkeletonFillSuffixError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["compliant"]);
    assert!(
        !locked
            .concrete
            .collision
            .as_ref()
            .unwrap()
            .skeleton_in_contact
    );
}

#[test]
fn shoulder_vnmsub_negates_exact_zero_residual_after_fused_rounding() {
    assert_eq!(shoulder_residual(1.0, 1.0).to_bits(), (-0.0_f32).to_bits());
    assert_eq!(shoulder_refinement(1.0, 1.0).to_bits(), 1.0_f32.to_bits());
}
