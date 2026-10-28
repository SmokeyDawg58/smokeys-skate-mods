use super::*;

struct Source {
    reads: Vec<&'static str>,
    fail: Option<&'static str>,
    flags: u32,
}
impl Source {
    fn read(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.reads.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
fn vector() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [-0.0, 12.0, -4.0],
        w: None,
    }
}
fn matrix() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([vector(); 4])
}
impl LaunchSource for Source {
    type Error = &'static str;
    fn board(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("board")?;
        Ok(vector())
    }
    fn com(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("com")?;
        Ok(vector())
    }
    fn animated_com(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("animated_com")?;
        Ok(vector())
    }
    fn animated_board(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("animated_board")?;
        Ok(vector())
    }
    fn orientation(&mut self, inverse: bool) -> Result<SkeletonRootMatrix, Self::Error> {
        self.read(if inverse { "inverse" } else { "orientation" })?;
        Ok(matrix())
    }
    fn velocity(&mut self, before_jump: bool) -> Result<SkeletonRootVector, Self::Error> {
        self.read(if before_jump {
            "before_jump"
        } else {
            "velocity"
        })?;
        Ok(vector())
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("dt")?;
        Ok(-0.0)
    }
    fn flags(&mut self) -> Result<u32, Self::Error> {
        self.read("flags")?;
        Ok(self.flags)
    }
    fn cone_angle(&mut self, offset: usize) -> Result<f32, Self::Error> {
        match offset {
            0x248 => {
                self.read("layout_x")?;
                Ok(11.0)
            }
            0x240 => {
                self.read("layout_z")?;
                Ok(22.0)
            }
            _ => panic!("unexpected trajectory layout offset"),
        }
    }
}
fn source(fail: Option<&'static str>, flags: u32) -> Source {
    Source {
        reads: Vec::new(),
        fail,
        flags,
    }
}
#[test]
fn ordered_copies_preserve_unknown_w_signed_zero_and_unwritten_launch_controls() {
    let mut output = RetainedTrajectoryLaunchInfo::constructed();
    output.player_jumped = true;
    output.use_start_override = true;
    output.trajectory_start_override = [19.0; 4].into();
    output.board_position_override = [23.0; 4].into();
    let mut input = source(None, 0x2000);
    fill_from_source(&mut input, &mut output).unwrap();
    assert_eq!(
        input.reads,
        [
            "board",
            "com",
            "animated_com",
            "animated_board",
            "orientation",
            "inverse",
            "velocity",
            "before_jump",
            "dt",
            "flags",
            "layout_x",
            "layout_z"
        ]
    );
    for value in [
        output.board_position,
        output.com_position,
        output.animated_com_position,
        output.animated_board_position,
        output.start_velocity,
        output.velocity_before_jump,
    ] {
        assert_eq!(value.w, None);
        assert_eq!(value.xyz.map(f32::to_bits), vector().xyz.map(f32::to_bits));
    }
    for orientation in [
        output.reckoning_orientation,
        output.reckoning_orientation_inverse,
    ] {
        assert!(
            orientation
                .columns()
                .iter()
                .all(|column| column.w.is_none())
        );
    }
    assert_eq!(output.time_delta.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(output.num_trajectories, 7);
    assert_eq!(
        [output.cone_angle_x, output.cone_angle_z].map(f32::to_bits),
        [11.0_f32, 22.0].map(f32::to_bits)
    );
    assert!(output.player_jumped && output.use_start_override);
    assert_eq!(
        output
            .trajectory_start_override
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [19.0_f32.to_bits(); 4]
    );
    assert_eq!(
        output
            .board_position_override
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [23.0_f32.to_bits(); 4]
    );
}
#[test]
fn late_second_layout_failure_preserves_first_layout_and_prior_stores() {
    let mut output = RetainedTrajectoryLaunchInfo::constructed();
    output.cone_angle_z = 37.0;
    let mut input = source(Some("layout_z"), 0x1000);
    assert_eq!(fill_from_source(&mut input, &mut output), Err("layout_z"));
    assert_eq!(output.cone_angle_x.to_bits(), 11.0_f32.to_bits());
    assert_eq!(output.cone_angle_z.to_bits(), 37.0_f32.to_bits());
    assert_eq!(output.num_trajectories, 1);
    assert_eq!(output.start_velocity.w, None);
}
#[test]
fn missing_com_preserves_board_store_and_does_not_read_outer_or_reckoning() {
    let mut output = RetainedTrajectoryLaunchInfo::constructed();
    let before = output.com_position;
    let mut input = source(Some("com"), 0);
    assert_eq!(fill_from_source(&mut input, &mut output), Err("com"));
    assert_eq!(input.reads, ["board", "com"]);
    assert_eq!(output.board_position.w, None);
    assert_eq!(output.com_position, before);
}
#[test]
fn failure_between_matrices_retains_transform_without_inventing_inverse() {
    let mut output = RetainedTrajectoryLaunchInfo::constructed();
    let before = output.reckoning_orientation_inverse;
    let mut input = source(Some("inverse"), 0);
    assert_eq!(fill_from_source(&mut input, &mut output), Err("inverse"));
    assert_eq!(output.reckoning_orientation, matrix());
    assert_eq!(output.reckoning_orientation_inverse, before);
    assert!(!input.reads.contains(&"velocity"));
}
#[test]
fn generated_null_layout_retains_default_zero_policy() {
    assert_eq!(layout_word(None, 0x248).to_bits(), 0);
    assert_eq!(layout_word(None, 0x240).to_bits(), 0);
}
