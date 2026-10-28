use super::*;
use crate::{
    physical_player_phys_out::{PhysOutSystemVector, construct::PhysOutAllocation},
    reckoning_numeric::HostReckoningNumeric,
};
struct Source {
    reads: Vec<String>,
    flips: usize,
    root_reads: usize,
    invalid: Option<usize>,
    fail_copy: Option<usize>,
    fail_com: bool,
}
impl Source {
    fn new() -> Self {
        Self {
            reads: Vec::new(),
            flips: 0,
            root_reads: 0,
            invalid: None,
            fail_copy: None,
            fail_com: false,
        }
    }
}
impl PhysOutComSource for Source {
    type Error = &'static str;
    fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reads.push("velocity".into());
        Ok(SkeletonRootVector::known([7.0, 8.0, 9.0, -0.0]))
    }
    fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reads.push("com".into());
        if self.fail_com {
            Err("com")
        } else {
            Ok(SkeletonRootVector::known([3.0; 4]))
        }
    }
}
impl SkeletonFillPrefixSource for Source {
    type Input = usize;
    fn capture_input(&mut self) -> Result<usize, Self::Error> {
        self.reads.push("input".into());
        Ok(self.flips)
    }
    fn animation_deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reads.push("pose".into());
        Ok(SkeletonRootVector {
            xyz: [0.0, 0.5, 1.0],
            w: None,
        })
    }
    fn board_flipped(&mut self, input: &usize) -> Result<bool, Self::Error> {
        self.reads.push(format!("flip{input}"));
        self.flips += 1;
        Ok(*input == 1)
    }
    fn root_column(&mut self, column: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.reads.push(format!("root{column}"));
        self.root_reads += 1;
        if self.root_reads > 4 && self.fail_copy == Some(column) {
            return Err("copy");
        }
        Ok(SkeletonRootVector {
            xyz: [if self.invalid == Some(column) {
                f32::NAN
            } else {
                2.0
            }; 3],
            w: None,
        })
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn fresh_flip_and_outer_com_preserve_unknown_root_w() {
    let mut out = output();
    let mut source = Source::new();
    out.try_fill_skeleton_prefix(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        source.reads,
        [
            "input", "pose", "flip0", "input", "flip1", "root0", "root1", "root2", "root3",
            "root0", "root1", "root2", "root3", "velocity", "com"
        ]
    );
    let skeleton = out.try_skeleton_output().unwrap();
    assert_eq!(skeleton.skeleton_deck_pitch.to_bits(), (-0.5_f32).to_bits());
    assert!(
        skeleton
            .skeleton_root_matrix
            .columns()
            .iter()
            .all(|v| v.w.is_none())
    );
    assert_eq!(
        out.system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .unwrap()
            .value
            .w
            .unwrap()
            .to_bits(),
        (-0.0_f32).to_bits()
    );
}
#[test]
fn locked_latch_retains_native_early_pose_and_first_flip_reads() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::new();
    assert!(matches!(
        out.try_fill_skeleton_prefix(&mut source, &mut HostReckoningNumeric),
        Err(SkeletonFillPrefixOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.reads, ["input", "pose", "flip0"]);
}
#[test]
fn late_root_load_failure_preserves_preceding_column_and_angles() {
    let mut out = output();
    let mut source = Source::new();
    source.fail_copy = Some(1);
    assert!(matches!(
        out.try_fill_skeleton_prefix(&mut source, &mut HostReckoningNumeric),
        Err(SkeletonFillPrefixOutputError::Source("copy"))
    ));
    let skeleton = out.try_skeleton_output().unwrap();
    assert!(skeleton.skeleton_root_matrix.columns()[0].w.is_none());
    assert!(skeleton.skeleton_root_matrix.columns()[1].w.is_some());
    assert_eq!(skeleton.skeleton_deck_pitch.to_bits(), (-0.5_f32).to_bits());
    assert!(!source.reads.iter().any(|s| s == "velocity"));
}
#[test]
fn invalid_xyz_stops_before_matrix_copy_and_com() {
    let mut out = output();
    let mut source = Source::new();
    source.invalid = Some(2);
    assert!(matches!(
        out.try_fill_skeleton_prefix(&mut source, &mut HostReckoningNumeric),
        Err(SkeletonFillPrefixOutputError::InvalidRootColumn(2))
    ));
    assert_eq!(source.root_reads, 3);
    assert!(
        out.try_skeleton_output()
            .unwrap()
            .skeleton_root_matrix
            .columns()[0]
            .w
            .is_some()
    );
}
#[test]
fn com_failure_occurs_after_matrix_and_early_velocity_read() {
    let mut out = output();
    let mut source = Source::new();
    source.fail_com = true;
    assert!(matches!(
        out.try_fill_skeleton_prefix(&mut source, &mut HostReckoningNumeric),
        Err(SkeletonFillPrefixOutputError::Com(
            PhysOutPublicationError::Source("com")
        ))
    ));
    assert_eq!(&source.reads[source.reads.len() - 2..], ["velocity", "com"]);
    assert!(
        out.try_skeleton_output()
            .unwrap()
            .skeleton_root_matrix
            .columns()[3]
            .w
            .is_none()
    );
}
#[test]
fn exact_negative_fused_residual_retains_negative_zero() {
    assert_eq!(
        (-1.0_f32.mul_add(1.0, -1.0)).to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(inverse(1.0, 1.0).to_bits(), 1.0_f32.to_bits());
}
