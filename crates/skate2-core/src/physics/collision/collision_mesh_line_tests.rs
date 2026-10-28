#![expect(
    clippy::float_cmp,
    reason = "finite binary-exact fixtures check source equality and tie behavior"
)]

use super::*;

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Debug)]
enum Call {
    GpDot([f32; 3], [f32; 3]),
    GpRoot(f32),
    GpReciprocal(f32),
    TriangleDot([f32; 4], [f32; 4]),
    TriangleRoot(f32),
}

impl Call {
    fn kind(&self) -> char {
        match self {
            Self::GpDot(..) => 'd',
            Self::GpRoot(..) => 's',
            Self::GpReciprocal(..) => 'r',
            Self::TriangleDot(..) => 'D',
            Self::TriangleRoot(..) => 'S',
        }
    }
}

#[derive(Default)]
struct Numeric {
    calls: Vec<Call>,
    triangle_call: usize,
    fail_triangle: Option<usize>,
    triangle_overrides: Vec<(usize, f32)>,
    zero_first_gp_normal_seed: bool,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}

impl AssemblyGroundNumeric for Numeric {
    fn sum_three_products(&mut self, a: [f32; 3], b: [f32; 3]) -> f32 {
        self.calls.push(Call::GpDot(a, b));
        dot(a, b)
    }

    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
        self.calls.push(Call::GpRoot(value));
        if self.zero_first_gp_normal_seed && self.calls.len() == 2 {
            0.0
        } else {
            1.0 / value.sqrt()
        }
    }

    fn reciprocal_seed(&mut self, value: f32) -> f32 {
        self.calls.push(Call::GpReciprocal(value));
        1.0 / value
    }
}

impl Numeric {
    fn triangle_value(&mut self, value: f32) -> Result<f32, usize> {
        let index = self.triangle_call;
        self.triangle_call += 1;
        if self.fail_triangle == Some(index) {
            return Err(index);
        }
        Ok(self
            .triangle_overrides
            .iter()
            .find_map(|&(at, replacement)| (at == index).then_some(replacement))
            .unwrap_or(value))
    }
}

impl ReckoningNumeric for Numeric {
    type Error = usize;

    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push(Call::TriangleDot(a, b));
        self.triangle_value(dot([a[0], a[1], a[2]], [b[0], b[1], b[2]]))
    }

    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(Call::TriangleRoot(value));
        self.triangle_value(1.0 / value.sqrt())
    }

    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("triangle source uses scalar division")
    }

    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("mesh triangle path has no integer conversion")
    }
}

struct Fixture {
    mesh: ClusteredMesh,
    bytes: Vec<u8>,
    forward: AnimationMatrix,
    inverse: AnimationMatrix,
}

impl Fixture {
    fn new(vertices: &[[f32; 3]], units: &[Vec<u8>]) -> Self {
        const TREE: usize = 0x60;
        const TABLE: usize = 0x90;
        const CLUSTER: usize = 0xa0;
        let stream = units.concat();
        let cluster_size = 16 + vertices.len() * 16 + stream.len();
        let mut bytes = vec![0_u8; CLUSTER + cluster_size];
        write_u32(&mut bytes, 48, u32::try_from(TREE).unwrap());
        write_u32(&mut bytes, 52, u32::try_from(TABLE).unwrap());
        write_u32(&mut bytes, 56, 1.0_f32.to_bits());
        bytes[62] = 1;
        bytes[63] = 2;
        write_u32(&mut bytes, 64, 1);
        write_u32(&mut bytes, 68, 1);
        write_u32(&mut bytes, 72, u32::try_from(units.len()).unwrap());
        write_u32(&mut bytes, 76, u32::try_from(units.len()).unwrap());
        let allocation_size = u32::try_from(bytes.len()).unwrap();
        write_u32(&mut bytes, 80, allocation_size);
        write_u32(&mut bytes, TREE + 8, u32::try_from(units.len()).unwrap());
        for lane in 0..3 {
            write_u32(&mut bytes, TREE + 16 + lane * 4, (-20.0_f32).to_bits());
            write_u32(&mut bytes, TREE + 32 + lane * 4, 20.0_f32.to_bits());
        }
        write_u32(&mut bytes, TABLE, u32::try_from(CLUSTER).unwrap());
        write_u16(&mut bytes, CLUSTER, u16::try_from(units.len()).unwrap());
        write_u16(
            &mut bytes,
            CLUSTER + 2,
            u16::try_from(stream.len()).unwrap(),
        );
        write_u16(
            &mut bytes,
            CLUSTER + 4,
            u16::try_from(vertices.len()).unwrap(),
        );
        write_u16(
            &mut bytes,
            CLUSTER + 8,
            u16::try_from(cluster_size).unwrap(),
        );
        bytes[CLUSTER + 10] = u8::try_from(vertices.len()).unwrap();
        for (index, point) in vertices.iter().enumerate() {
            for (lane, value) in point.iter().enumerate() {
                write_u32(
                    &mut bytes,
                    CLUSTER + 16 + index * 16 + lane * 4,
                    value.to_bits(),
                );
            }
            // This decoded vertex W is deliberately noncanonical. Point
            // transform consumes only XYZ, reconstructing W from placement.
            write_u32(
                &mut bytes,
                CLUSTER + 16 + index * 16 + 12,
                73.0_f32.to_bits(),
            );
        }
        bytes[CLUSTER + 16 + vertices.len() * 16..].copy_from_slice(&stream);
        Self {
            mesh: ClusteredMesh::parse(&bytes).unwrap(),
            bytes,
            forward: IDENTITY,
            inverse: IDENTITY,
        }
    }

    fn occurrence(&self, slot: usize, owner: Option<u32>) -> CollisionMeshOccurrence<'_, u32> {
        CollisionMeshOccurrence {
            mesh: &self.mesh,
            bytes: &self.bytes,
            forward: &self.forward,
            inverse: &self.inverse,
            owner,
            slot,
        }
    }
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn triangle(indices: [u8; 3], surface: u16) -> Vec<u8> {
    let [low, high] = surface.to_le_bytes();
    vec![0x81, indices[0], indices[1], indices[2], low, high]
}

fn fixture(units: &[Vec<u8>]) -> Fixture {
    Fixture::new(&[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0, 0.0]], units)
}

pub(crate) fn simple_triangle_mesh(surface: u16) -> (ClusteredMesh, Vec<u8>) {
    let fixture = fixture(&[triangle([0, 1, 2], surface)]);
    (fixture.mesh, fixture.bytes)
}

fn input() -> CollisionMeshLineInput {
    CollisionMeshLineInput {
        start: SkeletonRootVector::known([0.5, 0.75, 2.0, 7.0]),
        end: SkeletonRootVector::known([0.5, 0.75, -2.0, 13.0]),
        delta: SkeletonRootVector::known([0.0, 0.0, -4.0, 6.0]),
        model_box: AssemblyGroundQueryBox {
            minimum: [-10.0; 3],
            maximum: [10.0; 3],
        },
        line_fatness: 0.0,
    }
}

fn kinds(numeric: &Numeric) -> String {
    numeric.calls.iter().map(Call::kind).collect()
}

#[test]
fn actual_serialized_triangle_builds_complete_gp_before_wrapper_and_publication() {
    let fixture = fixture(&[triangle([0, 1, 2], 0xabcd)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    try_mesh_line(
        &fixture.occurrence(4, Some(17)),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(
        nearest.position,
        SkeletonRootVector::known([0.5, 0.75, 0.0, 10.0])
    );
    assert_eq!(
        nearest.normal,
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0])
    );
    assert_eq!(nearest.line_parameter, 0.5);
    assert_eq!(nearest.surface_id, 0xabcd);
    assert_eq!(nearest.owner, Some(17));
    assert_eq!(nearest.hit_count, 1);
    assert_eq!(
        nearest.provenance,
        Some(CollisionMeshLineProvenance {
            occurrence_slot: 4,
            cluster_index: 0,
            unit_byte_offset: 0,
            primitive_ordinal: 0
        })
    );
    assert_eq!(kinds(&numeric), "dsdddsssrrrDDDDDS");
    let Call::GpDot(a, b) = numeric.calls[0] else {
        panic!()
    };
    assert_eq!(a, [0.0, 0.0, 6.0]);
    assert_eq!(a, b);
    assert!(matches!(numeric.calls[1], Call::GpRoot(36.0)));
    let Call::GpReciprocal(value) = numeric.calls[8] else {
        panic!()
    };
    assert!(value > 0.0);
    assert!(matches!(numeric.calls[16], Call::TriangleRoot(36.0)));
}

#[test]
fn duplicate_units_count_all_hits_and_keep_first_surface_and_provenance() {
    let fixture = fixture(&[triangle([0, 1, 2], 0x1234), triangle([0, 1, 2], 0x5678)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    try_mesh_line(
        &fixture.occurrence(9, Some(8)),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest.hit_count, 2);
    assert_eq!(nearest.surface_id, 0x1234);
    assert_eq!(nearest.provenance.unwrap().unit_byte_offset, 0);
    assert_eq!(kinds(&numeric), "dsdddsssrrrDDDDDSdsdddsssrrrDDDDDS");
}

#[test]
fn second_closer_unit_replaces_first_in_encounter_order() {
    let fixture = Fixture::new(
        &[
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 1.0],
            [0.0, 3.0, 1.0],
        ],
        &[triangle([0, 1, 2], 10), triangle([3, 4, 5], 20)],
    );
    let mut nearest = CollisionMeshLineNearest::new();
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut Numeric::default(),
    )
    .unwrap();
    assert_eq!(nearest.hit_count, 2);
    assert_eq!(nearest.surface_id, 20);
    assert_eq!(nearest.line_parameter, 0.25);
    assert_eq!(nearest.position.xyz, [0.5, 0.75, 1.0]);
    assert_eq!(nearest.provenance.unwrap().unit_byte_offset, 6);
    assert_eq!(nearest.owner, None);
}

#[test]
fn quad_shared_edge_tests_both_primitives_after_complete_both_gp_records() {
    let fixture = Fixture::new(
        &[
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [2.0, 2.0, 0.0],
        ],
        &[vec![0x82, 0, 1, 2, 3, 0x34, 0x12]],
    );
    let mut input = input();
    input.start.xyz = [1.0, 1.0, 2.0];
    input.end.xyz = [1.0, 1.0, -2.0];
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    try_mesh_line(
        &fixture.occurrence(3, None),
        &input,
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest.hit_count, 2);
    assert_eq!(nearest.provenance.unwrap().primitive_ordinal, 0);
    assert_eq!(kinds(&numeric), "ddssdddsssrrrdddsssrrrDDDDDSDDDDDS");
}

#[test]
fn late_second_wrapper_failure_preserves_first_hit_without_counting_second() {
    let fixture = fixture(&[triangle([0, 1, 2], 10), triangle([0, 1, 2], 20)]);
    for failed in [6, 10, 11] {
        let mut numeric = Numeric {
            fail_triangle: Some(failed),
            ..Numeric::default()
        };
        let mut nearest = CollisionMeshLineNearest::new();
        let error = try_mesh_line(
            &fixture.occurrence(0, Some(4)),
            &input(),
            &mut nearest,
            &mut numeric,
        )
        .unwrap_err();
        assert!(
            matches!(error, CollisionMeshLineError::Cursor(MeshLineCursorError::Visitor(CollisionMeshLineUnitError::Triangle(CollisionTriangleLineError::Numeric(index)))) if index == failed)
        );
        assert_eq!(nearest.hit_count, 1);
        assert_eq!(nearest.surface_id, 10);
        assert_eq!(nearest.line_parameter, 0.5);
        assert_eq!(numeric.triangle_call, failed + 1);
        assert_eq!(numeric.calls.len(), 22 + failed + 1);
    }
}

#[test]
fn late_malformed_unit_does_not_erase_completed_hit() {
    let fixture = fixture(&[triangle([0, 1, 2], 0x2345), vec![0x0f]]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    let error = try_mesh_line(
        &fixture.occurrence(1, Some(2)),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CollisionMeshLineError::Cursor(MeshLineCursorError::Decode(_))
    ));
    assert_eq!(nearest.hit_count, 1);
    assert_eq!(nearest.surface_id, 0x2345);
    assert_eq!(numeric.calls.len(), 17);
}

#[test]
fn model_box_cull_precedes_all_gp_and_triangle_numeric_work() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut input = input();
    input.model_box = AssemblyGroundQueryBox {
        minimum: [5.0; 3],
        maximum: [6.0; 3],
    };
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input,
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest, CollisionMeshLineNearest::new());
    assert!(numeric.calls.is_empty());
}

#[test]
fn geometric_miss_still_builds_full_gp_but_never_publishes_or_counts() {
    let fixture = fixture(&[triangle([0, 2, 1], 7)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric::default();
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest, CollisionMeshLineNearest::new());
    assert_eq!(kinds(&numeric), "dsdddsssrrrD");
}

#[test]
fn nearest_uses_gp_normal_even_when_wrapper_normal_differs() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric {
        zero_first_gp_normal_seed: true,
        ..Numeric::default()
    };
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest.normal, SkeletonRootVector::known([0.0; 4]));
    let Call::TriangleDot(a, b) = numeric.calls[15] else {
        panic!()
    };
    assert_eq!(a[..3], [0.0, 0.0, 6.0]);
    assert_eq!(a, b);
    assert!(matches!(numeric.calls[16], Call::TriangleRoot(36.0)));
}

#[test]
fn tolerated_endpoint_hit_keeps_unclamped_position_and_clamps_only_parameter() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let numerator = 24.0 - 24.0 * f32::from_bits(0xb727_c5ac);
    let mut numeric = Numeric {
        triangle_overrides: vec![(3, numerator)],
        ..Numeric::default()
    };
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    let raw_parameter = numerator * (1.0_f32 / 24.0);
    assert!(raw_parameter > 1.0);
    assert_eq!(nearest.line_parameter, 1.0);
    assert_eq!(
        nearest.position.xyz[2].to_bits(),
        (-4.0_f32).mul_add(raw_parameter, 2.0).to_bits()
    );
    assert!(nearest.position.xyz[2] < -2.0);
    assert_eq!(
        nearest.position.w.unwrap().to_bits(),
        6.0_f32.mul_add(raw_parameter, 7.0).to_bits()
    );
}

#[test]
fn nan_raw_parameter_becomes_one_while_nan_position_is_retained() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric {
        triangle_overrides: vec![(3, f32::NAN)],
        ..Numeric::default()
    };
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut numeric,
    )
    .unwrap();
    assert_eq!(nearest.line_parameter, 1.0);
    assert!(nearest.position.xyz.into_iter().all(f32::is_nan));
    assert_eq!(nearest.hit_count, 1);
    assert_eq!(nearest.normal.xyz, [0.0, 0.0, 1.0]);
}

#[test]
fn count_wraps_and_nan_existing_nearest_never_replaces() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut nearest = CollisionMeshLineNearest::new();
    nearest.line_parameter = f32::from_bits(0x7fc0_1234);
    nearest.hit_count = u32::MAX;
    nearest.owner = Some(89);
    try_mesh_line(
        &fixture.occurrence(0, Some(1)),
        &input(),
        &mut nearest,
        &mut Numeric::default(),
    )
    .unwrap();
    assert_eq!(nearest.hit_count, 0);
    assert_eq!(nearest.line_parameter.to_bits(), 0x7fc0_1234);
    assert_eq!(nearest.owner, Some(89));
    assert_eq!(nearest.provenance, None);
    assert_eq!(nearest.position, SkeletonRootVector::known([0.0; 4]));
}

#[test]
fn actual_forward_inverse_and_original_delta_have_distinct_roles() {
    let mut fixture = fixture(&[triangle([0, 1, 2], 7)]);
    fixture.forward[12] = 100.0;
    fixture.inverse[12] = -100.0;
    let mut input = input();
    input.start.xyz[0] += 100.0;
    input.end.xyz[0] += 100.0;
    // KD uses this endpoint; thin must retain the separately supplied delta.
    input.end.xyz[2] = -8.0;
    input.start.w = None;
    let mut nearest = CollisionMeshLineNearest::new();
    try_mesh_line(
        &fixture.occurrence(2, Some(3)),
        &input,
        &mut nearest,
        &mut Numeric::default(),
    )
    .unwrap();
    assert_eq!(nearest.line_parameter, 0.5);
    assert_eq!(nearest.position.xyz, [100.5, 0.75, 0.0]);
    assert_eq!(nearest.position.w, None);
}

#[test]
fn placed_gp_normal_retains_known_noncanonical_cross_w() {
    let mut fixture = fixture(&[triangle([0, 1, 2], 7)]);
    fixture.forward[3] = 2048.5;
    fixture.forward[7] = f32::from_bits(0x44aa_b555); // 4097/3 rounded
    let mut nearest = CollisionMeshLineNearest::new();
    try_mesh_line(
        &fixture.occurrence(0, None),
        &input(),
        &mut nearest,
        &mut Numeric::default(),
    )
    .unwrap();
    // Actual placement constructs W=[1,4098,4098]. The rounded-product/FMA
    // residue is -1; the same GP normalization multiplier is then applied.
    assert_eq!(nearest.normal.w.unwrap().to_bits(), 0xbe2a_aaab);
    assert_eq!(nearest.normal.xyz, [0.0, 0.0, 1.0]);
}

#[test]
fn fat_wrapper_numeric_failure_is_late_after_gp_construction() {
    let fixture = fixture(&[triangle([0, 1, 2], 7)]);
    let mut input = input();
    input.line_fatness = 0.1;
    let mut nearest = CollisionMeshLineNearest::new();
    let mut numeric = Numeric {
        fail_triangle: Some(0),
        ..Numeric::default()
    };
    let error = try_mesh_line(
        &fixture.occurrence(0, None),
        &input,
        &mut nearest,
        &mut numeric,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CollisionMeshLineError::Cursor(MeshLineCursorError::Visitor(
            CollisionMeshLineUnitError::Triangle(CollisionTriangleLineError::Numeric(0))
        ))
    ));
    assert_eq!(nearest, CollisionMeshLineNearest::new());
    assert_eq!(kinds(&numeric), "dsdddsssrrrD");
}
