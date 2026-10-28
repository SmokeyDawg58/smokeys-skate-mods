use super::*;

struct Inputs {
    reads: Vec<&'static str>,
    correction: SkeletonRootVector,
    hook: SkeletonRootVector,
    trajectory: SkeletonRootVector,
    debug: bool,
}
impl BadGroundInputs for Inputs {
    type Error = ();
    fn correction(&mut self) -> Result<SkeletonRootVector, ()> {
        self.reads.push("correction");
        Ok(self.correction)
    }
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, ()> {
        self.reads.push("hook");
        Ok(self.hook)
    }
    fn trajectory_translation(&mut self) -> Result<SkeletonRootVector, ()> {
        self.reads.push("trajectory");
        Ok(self.trajectory)
    }
    fn min_traj_w(&mut self) -> Result<f32, ()> {
        self.reads.push("min trajectory");
        Ok(0.005)
    }
    fn max_skeleton_disp(&mut self) -> Result<f32, ()> {
        self.reads.push("max displacement");
        Ok(0.01)
    }
    fn debug_coll_response_a(&mut self) -> Result<bool, ()> {
        self.reads.push("debug");
        Ok(self.debug)
    }
}

#[test]
fn distant_trajectory_clears_prior_response_without_reading_debug() {
    let mut direction = Some(SkeletonRootVector::known([1.0, 0.0, 0.0, 1.0]));
    let mut valid = Some(true);
    let correction = SkeletonRootVector {
        xyz: [-2.0, 3.0, 4.0],
        w: None,
    };
    let mut source = Inputs {
        reads: Vec::new(),
        correction,
        hook: SkeletonRootVector::known([0.0; 4]),
        trajectory: SkeletonRootVector::known([0.1, 0.0, 0.0, 0.0]),
        debug: true,
    };
    let result = calculate_bad_ground_correction(&mut direction, &mut valid, &mut source).unwrap();
    assert_eq!(result, correction);
    assert_eq!(valid, Some(false));
    assert_eq!(
        source.reads,
        [
            "correction",
            "hook",
            "trajectory",
            "min trajectory",
            "max displacement"
        ]
    );
}

#[test]
fn opposing_response_uses_late_debug_switch_and_preserves_unknown_w() {
    let mut direction = Some(SkeletonRootVector::known([0.0; 4]));
    let mut valid = Some(false);
    let mut source = Inputs {
        reads: Vec::new(),
        correction: SkeletonRootVector {
            xyz: [-2.0, 3.0, 4.0],
            w: None,
        },
        hook: SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        trajectory: SkeletonRootVector::known([0.0; 4]),
        debug: true,
    };
    let result = calculate_bad_ground_correction(&mut direction, &mut valid, &mut source).unwrap();
    assert_eq!(result.xyz, [0.0, 3.0, 4.0]);
    assert_eq!(result.w, None);
    assert_eq!(valid, Some(true));
    assert_eq!(
        source.reads,
        [
            "correction",
            "hook",
            "trajectory",
            "min trajectory",
            "max displacement",
            "debug"
        ]
    );
}
