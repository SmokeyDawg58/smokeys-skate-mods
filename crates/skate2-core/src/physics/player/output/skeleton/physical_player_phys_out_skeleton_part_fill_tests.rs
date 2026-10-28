use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

#[derive(Default)]
struct Source {
    reads: Vec<String>,
    queries: u16,
    fail: Option<u16>,
}
impl SkeletonPartFillSource for Source {
    type Error = &'static str;
    fn volume_to_bone_translation(&mut self, part: usize) -> Result<[f32; 4], Self::Error> {
        self.reads.push(format!("local{part}"));
        Ok([2.0, 3.0, 4.0, f32::NAN])
    }
    fn part_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.reads.push(format!("part{part}"));
        self.queries += 1;
        if self.fail == Some(self.queries) {
            return Err("query");
        }
        let generation = f32::from(self.queries);
        Ok(SkeletonRootMatrix::from_columns([
            SkeletonRootVector::known([1.0, 0.0, generation, 0.0]),
            SkeletonRootVector::known([0.0, 1.0, generation, 0.0]),
            SkeletonRootVector::known([0.0, 0.0, generation, 0.0]),
            SkeletonRootVector {
                xyz: [generation; 3],
                w: None,
            },
        ]))
    }
    fn hips_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reads.push("velocity23".into());
        Ok(SkeletonRootVector::known([31.0, 32.0, 33.0, -0.0]))
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
fn assert_vector(actual: SkeletonRootVector, xyz: [f32; 3], w: Option<f32>) {
    assert_eq!(actual.xyz.map(f32::to_bits), xyz.map(f32::to_bits));
    assert_eq!(actual.w.map(f32::to_bits), w.map(f32::to_bits));
}

#[test]
fn queries_are_fresh_and_each_field_uses_its_own_source_result() {
    let mut output = output();
    let mut source = Source::default();
    output.try_fill_skeleton_parts(&mut source).unwrap();
    assert_eq!(
        source.reads,
        [
            "local19",
            "part19",
            "local15",
            "part15",
            "part19",
            "part15",
            "part1",
            "part1",
            "part23",
            "part23",
            "part23",
            "part23",
            "part15",
            "part19",
            "velocity23",
        ]
    );
    let s = output.try_skeleton_output().unwrap();
    assert_vector(s.skeleton_foot_right_bone_pos, [3.0, 4.0, 10.0], None);
    assert_vector(s.skeleton_foot_left_bone_pos, [4.0, 5.0, 20.0], None);
    assert_vector(s.skeleton_foot_right_look, [1.0, 0.0, 3.0], Some(0.0));
    assert_vector(s.skeleton_foot_left_look, [1.0, 0.0, 4.0], Some(0.0));
    assert_vector(s.skeleton_head_look, [0.0, 1.0, 5.0], Some(0.0));
    assert_vector(s.skeleton_head_pos, [6.0; 3], None);
    assert_vector(s.skeleton_hips_look, [1.0, 0.0, 7.0], Some(0.0));
    assert_vector(s.skeleton_hips_mat_y, [0.0, 1.0, 8.0], Some(0.0));
    assert_vector(s.skeleton_hips_mat_z, [0.0, 0.0, 9.0], Some(0.0));
    assert_vector(s.skeleton_hips_pos, [10.0; 3], None);
    assert_vector(s.skeleton_foot_left_pos, [11.0; 3], None);
    assert_vector(s.skeleton_foot_right_pos, [12.0; 3], None);
    assert_vector(s.skeleton_hips_vel, [31.0, 32.0, 33.0], Some(-0.0));
}

#[test]
fn later_query_failure_preserves_earlier_stores_and_stops_further_reads() {
    let mut output = output();
    let before = output.try_skeleton_output().unwrap().skeleton_head_pos;
    let mut source = Source {
        fail: Some(6),
        ..Source::default()
    };
    assert!(matches!(
        output.try_fill_skeleton_parts(&mut source),
        Err(SkeletonPartFillError::Source("query"))
    ));
    let s = output.try_skeleton_output().unwrap();
    assert_vector(s.skeleton_head_look, [0.0, 1.0, 5.0], Some(0.0));
    assert_vector(s.skeleton_head_pos, before.xyz, before.w);
    assert_eq!(source.reads.last().unwrap(), "part1");
    assert_eq!(source.queries, 6);
}

#[test]
fn locked_output_still_reads_local_and_queries_first_part_before_rejecting_store() {
    let mut output = output();
    let before = output
        .try_skeleton_output()
        .unwrap()
        .skeleton_foot_right_bone_pos;
    output.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        output.try_fill_skeleton_parts(&mut source),
        Err(SkeletonPartFillError::Access(_))
    ));
    assert_eq!(source.reads, ["local19", "part19"]);
    // Guarded output reads are intentionally not used after the latch is locked.
    let actual = output
        .concrete
        .skeleton
        .as_ref()
        .unwrap()
        .skeleton_foot_right_bone_pos;
    assert_vector(actual, before.xyz, before.w);
}

#[test]
fn bone_point_preserves_known_w_and_ignores_local_w() {
    let matrix = SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([1.0, 0.0, 0.0, 2.0]),
        SkeletonRootVector::known([0.0, 1.0, 0.0, 3.0]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, 4.0]),
        SkeletonRootVector::known([5.0, 6.0, 7.0, 8.0]),
    ]);
    assert_vector(
        bone_position([2.0, 3.0, 4.0, f32::NAN], &matrix),
        [7.0, 9.0, 11.0],
        Some(37.0),
    );
}
