//! `AIPath::Node`, `AIPath::InterpolatedNode` and `AIPath::Path` queries.
//!
//! Vector temporaries follow the native graphs: the quaternions decode as
//! `byte * (1 / 127.5) - 1` per lane and normalize with the refined reciprocal square root,
//! node interpolation runs `Math::Hermite::Evaluate` over `GetCurveMatrix` and blends the
//! orientations like `XMQuaternionSlerp` with a 5 degree linear threshold.
use super::vmx::{self, V4};
use skate2_formats::ai_path::{AiPath, AiPathBranchGroup, AiPathExtendedNodeData, AiPathNode};

/// `kfUpdateHz`.
pub(crate) const UPDATE_HZ: f32 = 60.0;
/// `METRES_PER_NODE_WIDTH_UNIT`, `1.0f / NODE_WIDTH_UNITS_PER_METER` (50).
pub(crate) const METRES_PER_NODE_WIDTH_UNIT: f32 = 1.0 / 50.0;
/// `VECTOR_MIN_SLERP_ANGLE`.
const MIN_SLERP_ANGLE: f32 = f32::from_bits(0x3db2_b8c3);

/// `ENodeEventType`.
pub(crate) const EVENT_START_TRICK: u8 = 1;
pub(crate) const EVENT_END_TRICK: u8 = 2;
pub(crate) const EVENT_INCIDENTAL_AIR: u8 = 4;
/// `ENodeBitFlags` masks.
pub(crate) const NODE_BOARD_FLIPPED: u8 = 1;
pub(crate) const NODE_CROUCHED: u8 = 2;
pub(crate) const NODE_AIRBORNE: u8 = 4;
pub(crate) const NODE_OFF_BOARD: u8 = 8;
/// `eExtendedNodeBitFlag_HasValidTrajectory`.
pub(crate) const EXTENDED_VALID_TRAJECTORY: u8 = 1;

/// `Score::ETrickType`.
pub(crate) mod trick_type {
    pub(crate) const NONE: i32 = 0;
    pub(crate) const AIR: i32 = 1;
    pub(crate) const FLIP: i32 = 2;
    pub(crate) const FINGER_FLIP: i32 = 3;
    pub(crate) const GRAB: i32 = 4;
    pub(crate) const GRIND: i32 = 5;
    pub(crate) const HANDPLANT: i32 = 6;
    pub(crate) const NO_COMPLY: i32 = 7;
    pub(crate) const MANUAL: i32 = 8;
    pub(crate) const SLIDE: i32 = 9;
    pub(crate) const HIPPY_JUMP: i32 = 10;
    pub(crate) const REVERT: i32 = 11;
    pub(crate) const BONELESS: i32 = 12;
    pub(crate) const FOOTPLANT: i32 = 13;
}

/// `Score::EScorableID` values the AI compares against.
pub(crate) mod scorable {
    pub(crate) const INVALID: i32 = -1;
    pub(crate) const COFFIN: i32 = 60;
    pub(crate) const GND_BSGRAB: i32 = 67;
    pub(crate) const GND_DBLGRAB: i32 = 68;
    pub(crate) const GND_FSGRAB: i32 = 69;
    pub(crate) const OLLIE: i32 = 128;
}

/// `AIPath::NodeDescription`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NodeDescription {
    pub(crate) index: i32,
    pub(crate) ratio: f32,
}

/// `AIPath::InterpolatedNode`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct InterpolatedNode {
    pub(crate) node: AiPathNode,
    pub(crate) description: NodeDescription,
}

/// `Physics::tTrajectory`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Trajectory {
    pub(crate) p0: V4,
    pub(crate) v0: V4,
    pub(crate) g: V4,
    pub(crate) t: V4,
}

/// The node the constructors build: zero fields, orientation W bytes 0xff.
pub(crate) fn constructed_node() -> AiPathNode {
    AiPathNode { board_orientation: [0, 0, 0, 0xff], skater_orientation: [0, 0, 0, 0xff], ..Default::default() }
}

/// `CompressedQuaternion::operator Quaternion` 0x824b2628.
pub(crate) fn decompress(q: [u8; 4]) -> V4 {
    let lanes = q.map(|byte| f32::from(byte).mul_add(f32::from_bits(0x3c00_8081), -1.0));
    let inverse = vmx::rsqrt(vmx::dot4(lanes, lanes));
    lanes.map(|lane| lane * inverse)
}

/// `PackComponent` 0x824b2588: the low byte of `fctidz((f + 1) * 127.5)`.
pub(crate) fn pack_component(f: f32) -> u8 {
    let scaled = (f + 1.0) * 127.5;
    (scaled as i64) as u8
}

/// `Node::GetBoardOrientationAsMatrix` 0x824b61c0: rows X, Y, Z.
pub(crate) fn board_orientation_matrix(node: &AiPathNode) -> [V4; 3] {
    vmx::basis_from_quaternion(decompress(node.board_orientation))
}

/// `Node::GetSkaterOrientationAsMatrix` 0x824b62a8.
pub(crate) fn skater_orientation_matrix(node: &AiPathNode) -> [V4; 3] {
    vmx::basis_from_quaternion(decompress(node.skater_orientation))
}

/// `Node::GetEffectiveBoardOrientationAsMatrix` 0x824c7b80: a flipped board negates the
/// whole X and Z rows.
pub(crate) fn effective_board_orientation_matrix(node: &AiPathNode) -> [V4; 3] {
    let [x, y, z] = board_orientation_matrix(node);
    if node.bit_flags & NODE_BOARD_FLIPPED != 0 {
        [x.map(|v| -v), y, z.map(|v| -v)]
    } else {
        [x, y, z]
    }
}

/// `Node::GetMatrix` 0x824c7bf8: the effective board orientation at the node position. The
/// native translation W lane is stale stack data; the affine convention's 1 stands in.
pub(crate) fn node_matrix(node: &AiPathNode) -> [V4; 4] {
    let [x, y, z] = effective_board_orientation_matrix(node);
    [x, y, z, [node.position[0], node.position[1], node.position[2], 1.0]]
}

/// `Node::GetVelocity` 0x824b6138: the per-frame direction times `kfUpdateHz`.
pub(crate) fn velocity(node: &AiPathNode) -> V4 {
    vmx::scale(vmx::v3(node.direction), UPDATE_HZ)
}

/// `Node::Reset` 0x824c7d20: zero position and direction, orientations packed from zero,
/// no extended data, zero frame count, widths, event and flags; the extra bytes stay.
pub(crate) fn reset(node: &mut AiPathNode) {
    node.position = [0.0; 3];
    node.direction = [0.0; 3];
    node.board_orientation = [pack_component(0.0); 4];
    node.skater_orientation = [pack_component(0.0); 4];
    node.extended = None;
    node.frame_count_since_last_node = 0;
    node.width_left = 0;
    node.width_right = 0;
    node.event_type = 0;
    node.bit_flags = 0;
}

/// `Node::SetGeneric` 0x824c0800: everything but the frame count and the extra bytes.
pub(crate) fn set_generic(dst: &mut AiPathNode, src: &AiPathNode) {
    dst.position = src.position;
    dst.direction = src.direction;
    dst.board_orientation = src.board_orientation;
    dst.skater_orientation = src.skater_orientation;
    dst.extended = src.extended;
    dst.width_left = src.width_left;
    dst.width_right = src.width_right;
    dst.event_type = src.event_type;
    dst.bit_flags = src.bit_flags;
}

/// `Node::GetCurveMatrix` 0x824b65a8: the two positions and the two directions scaled by
/// the second node's frame count, W lanes zero.
fn curve_matrix(n1: &AiPathNode, n2: &AiPathNode, frames: f32) -> [V4; 4] {
    let d1 = n1.direction;
    let d2 = n2.direction;
    [
        vmx::v3(n1.position),
        vmx::v3(n2.position),
        [d1[0] * frames, d1[1] * frames, d1[2] * frames, 0.0 * frames],
        [d2[0] * frames, d2[1] * frames, d2[2] * frames, 0.0 * frames],
    ]
}

/// `Math::Hermite::Evaluate` 0x829b0f70 against `hermiteBasisMat` rows (2,-2,1,1),
/// (-3,3,-2,-1), (0,0,1,0), (1,0,0,0); the W lane repeats X.
pub(crate) fn hermite_evaluate(m: &[V4; 4], t: f32) -> V4 {
    let t2 = t * t;
    let t3 = (t * t) * t;
    let lane = |i: usize| {
        let c3 = m[3][i].mul_add(1.0, m[2][i].mul_add(1.0, m[1][i].mul_add(-2.0, m[0][i] * 2.0)));
        let c2 = m[3][i].mul_add(-1.0, m[2][i].mul_add(-2.0, m[1][i].mul_add(3.0, m[0][i] * -3.0)));
        let c1 = m[3][i].mul_add(0.0, m[2][i].mul_add(1.0, m[1][i].mul_add(0.0, m[0][i] * 0.0)));
        let c0 = m[3][i].mul_add(0.0, m[2][i].mul_add(0.0, m[1][i].mul_add(0.0, m[0][i])));
        c0.mul_add(1.0, c1.mul_add(t, c2.mul_add(t2, c3 * t3)))
    };
    let x = lane(0);
    [x, lane(1), lane(2), x]
}

/// `Node::GetInterpolatedDirection` 0x824b67b8: near either end, or with under 0.1 frames,
/// the nearer node's direction; otherwise the curve's central difference over +-0.01 scaled
/// by 50 / frames.
fn interpolated_direction(n1: &AiPathNode, n2: &AiPathNode, ratio: f32, curve: &[V4; 4], frames: f32) -> V4 {
    if ratio <= 0.01 || ratio >= f32::from_bits(0x3f7d_70a4) || frames <= 0.1 {
        return vmx::v3(if ratio <= 0.5 { n1.direction } else { n2.direction });
    }
    let before = hermite_evaluate(curve, ratio - 0.01);
    let after = hermite_evaluate(curve, ratio + 0.01);
    vmx::scale(vmx::sub(after, before), 50.0 / frames)
}

/// `Node::CalculateBoardOrientationAsQuaternion` 0x824c0aa8 (and the skater twin
/// 0x824c0ca8): shortest arc, `normalize(lerp)` within the 5 degree threshold, else slerp
/// whose `1 / sin(theta)` is `vrefp` with two fused Newton steps.
fn interpolate_orientation(from: [u8; 4], to: [u8; 4], t: f32) -> V4 {
    let q2 = decompress(to);
    let mut q1 = decompress(from);
    let mut cosine = vmx::dot4(q1, q2);
    if 0.0 > cosine {
        q1 = q1.map(|v| -v);
        cosine = -cosine;
    }
    if cosine > vmx::cos(MIN_SLERP_ANGLE) {
        let dot = vmx::dot4(q1, q2);
        let forward = vmx::madd(vmx::sub(q2, q1), [t; 4], q1);
        let backward = vmx::nmsub(vmx::add(q2, q1), [t; 4], q1);
        let chosen = if dot > 0.0 { forward } else { backward };
        let inverse = vmx::rsqrt(vmx::dot4(chosen, chosen));
        return chosen.map(|v| v * inverse);
    }
    let theta = vmx::acos(cosine);
    let sine = vmx::sin(theta);
    let estimate = 1.0 / sine;
    let first = estimate.mul_add((-estimate).mul_add(sine, 1.0), estimate);
    let reciprocal = first.mul_add((-first).mul_add(sine, 1.0), first);
    let w2 = reciprocal * vmx::sin(t * theta);
    let to_part = q2.map(|v| v * w2);
    let w1 = reciprocal * vmx::sin((1.0 - t) * theta);
    std::array::from_fn(|i| q1[i].mul_add(w1, to_part[i]))
}

/// `Node::GetInterpolatedPosAndDirection` 0x824b6ad0: the curve between the nodes over the
/// second's frame count at `ratio`, and its direction there.
pub(crate) fn interpolated_pos_and_direction(n1: &AiPathNode, n2: &AiPathNode, ratio: f32) -> (V4, V4) {
    let frames = f32::from(n2.frame_count_since_last_node);
    let curve = curve_matrix(n1, n2, frames);
    (hermite_evaluate(&curve, ratio), interpolated_direction(n1, n2, ratio, &curve, frames))
}

/// `Node::SetInterpolated` 0x824c0ea8 through `SetInterpolatedPosAndDirection` 0x824c09b0.
pub(crate) fn set_interpolated(dst: &mut AiPathNode, n1: &AiPathNode, n2: &AiPathNode, ratio: f32) {
    let (position, direction) = interpolated_pos_and_direction(n1, n2, ratio);
    dst.position = vmx::xyz(position);
    dst.direction = vmx::xyz(direction);
    dst.board_orientation = interpolate_orientation(n1.board_orientation, n2.board_orientation, ratio).map(pack_component);
    dst.skater_orientation = interpolate_orientation(n1.skater_orientation, n2.skater_orientation, ratio).map(pack_component);
    let lerp = |a: u8, b: u8| {
        let step = (f32::from(i16::from(b) - i16::from(a)) * ratio) as i64;
        a.wrapping_add(step as u8)
    };
    dst.width_left = lerp(n1.width_left, n2.width_left);
    dst.width_right = lerp(n1.width_right, n2.width_right);
    let source = if ratio >= 0.5 { n2 } else { n1 };
    dst.event_type = source.event_type;
    dst.extended = source.extended;
    dst.bit_flags = source.bit_flags;
    dst.frame_count_since_last_node = 0;
}

/// `Node::GetWidthVector` 0x824b6b88: `normalize(dir)` crossed with the skater orientation's
/// Y row.
pub(crate) fn width_vector(node: &AiPathNode, dir: V4) -> V4 {
    let inverse = vmx::rsqrt(vmx::dot3(dir, dir));
    let unit = dir.map(|v| v * inverse);
    vmx::cross(unit, skater_orientation_matrix(node)[1])
}

/// `Node::GetWidePos` 0x824b6cd8: the position moved along the width vector by the right
/// (ratio >= 0) or left width in metres times the ratio. A zero width leaves the width vector
/// unscaled; a degenerate direction leaves the position.
pub(crate) fn wide_pos(node: &AiPathNode, ratio: f32) -> V4 {
    let dir = vmx::v3(node.direction);
    let position = vmx::v3(node.position);
    if !(vmx::dot3(dir, dir) > 1.0e-5) {
        return position;
    }
    let width = width_vector(node, dir);
    let units = if ratio >= 0.0 { node.width_right } else { node.width_left };
    let metres = f32::from(units) * METRES_PER_NODE_WIDTH_UNIT;
    let offset = if metres > 0.0 { width.map(|v| v * metres) } else { width };
    vmx::madd(offset, [ratio; 4], position)
}

/// `Node::GetTrajectoryOffset` 0x824b6390.
pub(crate) fn trajectory_offset(node: &AiPathNode) -> Option<V4> {
    node.extended.map(|ext| vmx::v3(ext.trajectory_offset))
}

/// `ExtendedNodeData::GetTrajectory` 0x824c0320: start position and velocity, gravity -9.8,
/// time -1.
pub(crate) fn trajectory(ext: &AiPathExtendedNodeData) -> Trajectory {
    Trajectory {
        p0: vmx::v3(ext.trajectory_start_position),
        v0: vmx::v3(ext.trajectory_start_velocity),
        g: [0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0],
        t: [-1.0; 4],
    }
}

/// `InterpolatedNode::SetWide` 0x824c2d40.
pub(crate) fn set_wide(dst: &mut InterpolatedNode, src: &InterpolatedNode, ratio: f32) {
    set_generic(&mut dst.node, &src.node);
    dst.node.position = vmx::xyz(wide_pos(&src.node, ratio));
    dst.node.frame_count_since_last_node = src.node.frame_count_since_last_node;
    dst.description = src.description;
}

/// `Node::GetTrickScorable` 0x824b0628.
pub(crate) fn trick_scorable(node: &AiPathNode) -> i32 {
    node.extended.map_or(scorable::INVALID, |ext| i32::from(ext.trick_scorable))
}

/// `Node::GetTrickType` 0x824b6990.
pub(crate) fn node_trick_type(node: &AiPathNode) -> i32 {
    crate::physical_player_phys_out::conditioners::scorable_id_trick_type(trick_scorable(node))
        .unwrap_or(trick_type::NONE)
}

/// `Node::GetAirSpin180Count` 0x824b06b0.
pub(crate) fn air_spin_180_count(node: &AiPathNode) -> i32 {
    node.extended.map_or(0, |ext| i32::from(ext.air_spin_180_count))
}

/// `IsTrickNodeDefinitelyOnGround` 0x824b7258.
fn is_trick_node_definitely_on_ground(node: &AiPathNode) -> bool {
    match node_trick_type(node) {
        trick_type::GRAB => matches!(
            trick_scorable(node),
            scorable::GND_BSGRAB | scorable::GND_DBLGRAB | scorable::GND_FSGRAB | scorable::COFFIN
        ),
        trick_type::GRIND | trick_type::MANUAL | trick_type::SLIDE => true,
        _ => false,
    }
}

/// The path's node count as the native `int`.
pub(crate) fn node_count(path: &AiPath) -> i32 {
    path.nodes.len() as i32
}

pub(crate) fn node(path: &AiPath, index: i32) -> &AiPathNode {
    &path.nodes[index as usize]
}

/// `Path::SetInterpolatedNode` 0x824c1c80: between `index` and the next node, or the last
/// node with its frame count.
pub(crate) fn set_interpolated_node(path: &AiPath, dst: &mut AiPathNode, index: i32, ratio: f32) {
    if index + 1 < node_count(path) {
        set_interpolated(dst, node(path, index), node(path, index + 1), ratio);
    } else {
        let last = node(path, node_count(path) - 1);
        set_generic(dst, last);
        dst.frame_count_since_last_node = last.frame_count_since_last_node;
    }
}

/// The frames from `ratio` along to the next node, `(int)(frames * (1 - ratio))`.
fn frames_to_next(path: &AiPath, index: i32, ratio: f32) -> i32 {
    (f32::from(node(path, index).frame_count_since_last_node) * (1.0 - ratio)) as i32
}

/// `Path::GetNextEventNodeIndex` 0x824b0b20: the first node after `index` with one of the
/// event types within `max_frames`; `frames` counts up to it.
pub(crate) fn next_event_node_index(
    path: &AiPath,
    index: i32,
    ratio: f32,
    max_frames: i32,
    frames: &mut i32,
    events: &[u8],
) -> i32 {
    next_matching_node_index(path, index, ratio, max_frames, frames, |n| events.contains(&n.event_type))
}

/// `Path::GetNextFlaggedNodeIndex` 0x824c1968 for flag bit `bit`.
pub(crate) fn next_flagged_node_index(
    path: &AiPath,
    index: i32,
    ratio: f32,
    max_frames: i32,
    frames: &mut i32,
    bit: u32,
) -> i32 {
    next_matching_node_index(path, index, ratio, max_frames, frames, |n| n.bit_flags & (1 << bit) != 0)
}

fn next_matching_node_index(
    path: &AiPath,
    index: i32,
    ratio: f32,
    max_frames: i32,
    frames: &mut i32,
    matches: impl Fn(&AiPathNode) -> bool,
) -> i32 {
    let count = node_count(path);
    let mut next = index + 1;
    if next >= count {
        return -1;
    }
    *frames = frames_to_next(path, next, ratio);
    while *frames < max_frames {
        if matches(node(path, next)) {
            return next;
        }
        next += 1;
        if next < count {
            *frames += i32::from(node(path, next).frame_count_since_last_node);
        }
        if next >= count {
            return -1;
        }
    }
    -1
}

/// `Path::GetFutureNodeIndex` 0x824b0d10: the node `frames` ahead, `ratio` along it.
pub(crate) fn future_node_index(path: &AiPath, index: i32, ratio: &mut f32, mut frames: f32) -> i32 {
    let count = node_count(path);
    if frames <= 0.0 {
        return index;
    }
    let mut current = index;
    for next in index + 1..count {
        let start = *ratio;
        let span = f32::from(node(path, next).frame_count_since_last_node);
        let step = frames / span;
        if start + step <= 1.0 {
            *ratio = start + step;
            return current;
        }
        *ratio = 0.0;
        current += 1;
        frames = (-(1.0 - start)).mul_add(span, frames);
        if frames <= 0.0 {
            return current;
        }
    }
    *ratio = 0.0;
    count - 1
}

/// `Path::GetNodeFrameDifference` 0x824b0e78: frames from the first description forward to
/// the second.
pub(crate) fn node_frame_difference(path: &AiPath, i1: i32, r1: f32, i2: i32, r2: f32) -> f32 {
    let count = node_count(path);
    let mut sum = 0i32;
    for index in i1 + 1..=i2 {
        sum += i32::from(node(path, index).frame_count_since_last_node);
    }
    let mut frames = sum as f32;
    if i1 + 1 < count {
        frames = (-f32::from(node(path, i1 + 1).frame_count_since_last_node)).mul_add(r1, frames);
    }
    if i2 + 1 < count {
        frames = f32::from(node(path, i2 + 1).frame_count_since_last_node).mul_add(r2, frames);
    }
    frames
}

/// `Path::GetNodeFrameDifferenceUnordered` 0x824b1078.
pub(crate) fn node_frame_difference_unordered(path: &AiPath, i1: i32, r1: f32, i2: i32, r2: f32) -> f32 {
    if i1 < i2 || (i1 == i2 && r1 < r2) {
        node_frame_difference(path, i1, r1, i2, r2)
    } else if i1 == i2 && r1 == r2 {
        0.0
    } else {
        -node_frame_difference(path, i2, r2, i1, r1)
    }
}

/// `Path::GetNearestNodeIndex(pos, start, count)` 0x824b7650: within `start..=start+count`.
pub(crate) fn nearest_node_index_in(path: &AiPath, pos: [f32; 3], start: i32, count: i32) -> i32 {
    let last = (start + count).min(node_count(path) - 1);
    let mut best = -1;
    let mut best_sqr = 0.0;
    for index in start..=last {
        let dist_sqr = super::path_manager::distance_squared(node(path, index).position, pos);
        if best < 0 || dist_sqr < best_sqr {
            best = index;
            best_sqr = dist_sqr;
        }
    }
    best
}

/// `Path::GetNearestNodeIndexWithinNumFrames` 0x824b77d8.
pub(crate) fn nearest_node_index_within_frames(path: &AiPath, pos: [f32; 3], start: i32, max_frames: u32) -> i32 {
    let mut best = start;
    let mut best_sqr = super::path_manager::distance_squared(node(path, start).position, pos);
    let mut frames = 0u32;
    for index in start + 1..node_count(path) {
        let n = node(path, index);
        frames += u32::from(n.frame_count_since_last_node);
        if frames > max_frames {
            break;
        }
        let dist_sqr = super::path_manager::distance_squared(n.position, pos);
        if dist_sqr < best_sqr {
            best = index;
            best_sqr = dist_sqr;
        }
    }
    best
}

/// `Path::GetNearestNeighbourIndex` 0x824b79f0: whichever neighbour of `index` is nearer,
/// the previous on ties; the only neighbour at either end.
fn nearest_neighbour_index(path: &AiPath, pos: [f32; 3], index: i32) -> i32 {
    let previous = index - 1;
    let next = index + 1;
    if previous < 0 {
        return next;
    }
    if next >= node_count(path) {
        return previous;
    }
    let before = super::path_manager::distance_squared(node(path, previous).position, pos);
    let after = super::path_manager::distance_squared(node(path, next).position, pos);
    if before < after { previous } else { next }
}

/// `Path::RatioForClosestPointToLine` 0x824b7b78: the projection onto `index -> index+1`,
/// 1 for a segment under 1e-6 squared.
fn ratio_for_closest_point_to_line(path: &AiPath, point: [f32; 3], index: i32) -> f32 {
    let a = vmx::v3(node(path, index).position);
    let b = vmx::v3(node(path, index + 1).position);
    let segment = vmx::sub(b, a);
    let length_sqr = vmx::dot3(segment, segment);
    let along = vmx::dot3(vmx::sub(vmx::v3(point), a), segment);
    if length_sqr <= 1.0e-6 { 1.0 } else { along / length_sqr }
}

/// `fsel` clamp to [0, 1]: NaN selects 1.
fn clamp_unit(ratio: f32) -> f32 {
    let low = if -ratio >= 0.0 { 0.0 } else { ratio };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

/// `Path::GetApproximatedNodeIndex` 0x824b7cf8: the segment around `index` that `pos`
/// projects onto, and how far along.
pub(crate) fn approximated_node_index(path: &AiPath, pos: [f32; 3], index: i32, ratio: &mut f32) -> i32 {
    let previous = index - 1;
    let next = index + 1;
    if previous < 0 {
        *ratio = clamp_unit(ratio_for_closest_point_to_line(path, pos, index));
        return index;
    }
    if next >= node_count(path) {
        *ratio = clamp_unit(ratio_for_closest_point_to_line(path, pos, previous));
        return previous;
    }
    let before = ratio_for_closest_point_to_line(path, pos, previous);
    let current = ratio_for_closest_point_to_line(path, pos, index);
    let inside = |r: f32| !(r < 0.0 || r > 1.0);
    if inside(before) {
        if !inside(current) || nearest_neighbour_index(path, pos, index) == previous {
            *ratio = before;
            return previous;
        }
        *ratio = current;
        return index;
    }
    if inside(current) {
        *ratio = current;
        return index;
    }
    if before >= 0.0 {
        *ratio = if current <= 1.0 { 0.0 } else { 1.0 };
        index
    } else {
        *ratio = 0.0;
        previous
    }
}

/// `Path::GetBranchGroup` 0x824b7f40.
pub(crate) fn branch_group(path: &AiPath, index: i32) -> Option<&AiPathBranchGroup> {
    path.branch_groups.iter().find(|group| group.source_node_index == index)
}

/// `Path::GetNextNodeOnGroundIndex` 0x824c1b68: the first grounded node after an airborne
/// stretch, unless the airborne trick is a footplant.
pub(crate) fn next_node_on_ground_index(path: &AiPath, index: i32) -> i32 {
    let mut airborne = false;
    let mut current = trick_type::NONE;
    for at in index..node_count(path) {
        let n = node(path, at);
        if n.event_type == EVENT_START_TRICK {
            current = node_trick_type(n);
        } else if n.event_type == EVENT_END_TRICK {
            current = trick_type::NONE;
        }
        if n.bit_flags & NODE_AIRBORNE != 0 {
            airborne = true;
        } else if airborne {
            if current != trick_type::FOOTPLANT {
                return at;
            }
            airborne = false;
        }
    }
    -1
}

/// `Path::GetCurrentTrickTypeAtNode` 0x824b7308: the trick running before `index`.
pub(crate) fn current_trick_type_at_node(path: &AiPath, index: i32) -> i32 {
    let mut current = trick_type::NONE;
    for at in 0..index {
        let n = node(path, at);
        if n.event_type == EVENT_START_TRICK {
            current = node_trick_type(n);
        } else if n.event_type == EVENT_END_TRICK {
            current = trick_type::NONE;
        }
    }
    current
}

/// The slope test of the safe-node searches (0x824c1874..1894): `y^2 / fma(z, z, x^2)`, zero
/// at or below 0.001 horizontal.
fn slope_sqr(dir: [f32; 3]) -> f32 {
    let horizontal = dir[2].mul_add(dir[2], dir[0] * dir[0]);
    if horizontal <= 0.001 { 0.0 } else { dir[1] * dir[1] / horizontal }
}

fn is_ground_trick(kind: i32) -> bool {
    matches!(kind, trick_type::NONE | trick_type::GRAB | trick_type::MANUAL | trick_type::SLIDE)
}

/// `Path::FindSafeNodeIndexBehind` 0x824c1270: scanning back from `index`, a grounded,
/// gentle node a few frames clear of airborne nodes and tricks.
pub(crate) fn find_safe_node_index_behind(path: &AiPath, index: i32) -> i32 {
    let mut since_trick = 0;
    let mut since_air = 0;
    let mut current = trick_type::NONE;
    let tally = |n: &AiPathNode, since_air: &mut i32, since_trick: &mut i32, current: &mut i32| {
        if n.bit_flags & NODE_AIRBORNE != 0 {
            *since_air = 0;
        } else {
            *since_air += 1;
        }
        if n.event_type == EVENT_START_TRICK {
            *current = node_trick_type(n);
            *since_trick = 0;
        } else {
            *since_trick += 1;
        }
    };
    let mut at = (index + 2).min(node_count(path) - 1);
    while at > index {
        tally(node(path, at), &mut since_air, &mut since_trick, &mut current);
        at -= 1;
    }
    let mut at = index;
    while at >= 0 {
        let n = node(path, at);
        tally(n, &mut since_air, &mut since_trick, &mut current);
        if since_air > 2
            && (is_ground_trick(current) || since_trick > 3)
            && slope_sqr(n.direction) < 1.0
            && is_ground_trick(current_trick_type_at_node(path, at))
        {
            return at;
        }
        at -= 1;
    }
    0
}

/// `Path::FindSafeNodeIndex` 0x824c1570: forward from `index`, the first of three
/// consecutive gentle ground nodes well clear of trajectories and air.
pub(crate) fn find_safe_node_index(path: &AiPath, index: i32) -> i32 {
    let count = node_count(path);
    let mut since_trajectory = 7200;
    let mut since_air = 7200;
    let mut current = trick_type::NONE;
    let step = |n: &AiPathNode, since_trajectory: &mut i32, since_air: &mut i32, current: &mut i32| {
        let frames = i32::from(n.frame_count_since_last_node);
        if n.extended.is_some_and(|ext| ext.bit_flags & EXTENDED_VALID_TRAJECTORY != 0) {
            *since_trajectory = 0;
        } else if n.event_type == EVENT_START_TRICK && is_trick_node_definitely_on_ground(n) {
            *since_trajectory = 7200;
        } else {
            *since_trajectory += frames;
        }
        if n.bit_flags & NODE_AIRBORNE != 0 {
            *since_air = 0;
        } else {
            *since_air += frames;
        }
        if n.event_type == EVENT_START_TRICK {
            *current = node_trick_type(n);
        } else if n.event_type == EVENT_END_TRICK {
            *current = trick_type::NONE;
        }
    };
    for at in 0..index {
        step(node(path, at), &mut since_trajectory, &mut since_air, &mut current);
    }
    let mut at = index;
    let mut run = 0;
    let mut first = -1;
    loop {
        if at >= count {
            return count - 1;
        }
        let n = node(path, at);
        step(n, &mut since_trajectory, &mut since_air, &mut current);
        let clear = since_trajectory > 45 && since_air > 15;
        if clear && slope_sqr(n.direction) < 1.0 && is_ground_trick(current) {
            if run == 0 {
                first = at;
            }
            run += 1;
            if run > 2 {
                return first;
            }
        } else {
            run = 0;
        }
        at += 1;
    }
}

/// `Path::DoesNextTrajectoryLeadToGrind` 0x824c1090.
pub(crate) fn does_next_trajectory_lead_to_grind(path: &AiPath, index: i32) -> bool {
    let count = node_count(path);
    let mut found = false;
    let mut frames = 0;
    let mut ended = false;
    for at in index..count {
        let n = node(path, at);
        let valid = n.extended.is_some_and(|ext| ext.bit_flags & EXTENDED_VALID_TRAJECTORY != 0);
        if !found {
            found = valid;
            continue;
        }
        frames += i32::from(n.frame_count_since_last_node);
        if valid {
            if frames > 60 {
                return false;
            }
            frames = 0;
        }
        match n.event_type {
            EVENT_START_TRICK => {
                let kind = node_trick_type(n);
                if kind == trick_type::GRIND {
                    return true;
                }
                if kind > trick_type::NO_COMPLY && kind <= trick_type::SLIDE {
                    return false;
                }
            }
            EVENT_END_TRICK => {
                if n.bit_flags & NODE_AIRBORNE == 0 {
                    return false;
                }
                ended = true;
            }
            EVENT_INCIDENTAL_AIR => return false,
            _ => {}
        }
        if !ended || n.bit_flags & NODE_AIRBORNE != 0 {
            if frames > 600 {
                return false;
            }
            continue;
        }
        if at + 1 < count {
            let next = node(path, at + 1);
            return next.event_type == EVENT_START_TRICK && node_trick_type(next) == trick_type::GRIND;
        }
        return false;
    }
    false
}

/// `Path::IsEntirelyOutOfCameraView` 0x824b73a8: no second node lies more than 0.001 in
/// front of the camera along its Z row.
pub(crate) fn is_entirely_out_of_camera_view(path: &AiPath, camera: &[V4; 4]) -> bool {
    !path.nodes.iter().step_by(2).any(|n| vmx::dot3(vmx::sub(vmx::v3(n.position), camera[3]), camera[2]) > 0.001)
}

/// `Path::CalculateMinDistSquaredApprox` 0x824b7510: over every second node from `start`.
pub(crate) fn min_dist_squared_approx(path: &AiPath, pos: [f32; 3], start: i32) -> f32 {
    path.nodes
        .iter()
        .skip(start as usize)
        .step_by(2)
        .map(|n| super::path_manager::distance_squared(n.position, pos))
        .fold(f32::MAX, |best, d| if d < best { d } else { best })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(count: usize, frames: u8) -> AiPath {
        AiPath {
            bbox_min: [0.0; 4],
            bbox_max: [0.0; 4],
            id: Default::default(),
            nodes: (0..count)
                .map(|i| AiPathNode {
                    position: [i as f32, 0.0, 0.0],
                    direction: [1.0 / f32::from(frames), 0.0, 0.0],
                    board_orientation: [127, 127, 127, 255],
                    skater_orientation: [127, 127, 127, 255],
                    frame_count_since_last_node: frames,
                    ..Default::default()
                })
                .collect(),
            has_extra_node_data: false,
            branch_groups: Vec::new(),
            bit_flags: 0,
            allowed_skaters: u64::MAX,
            skill_level: 0,
            extra_data: [0; 3],
        }
    }

    #[test]
    fn pack_and_decompress_round_the_identity() {
        assert_eq!(pack_component(0.0), 127);
        assert_eq!(pack_component(1.0), 255);
        let q = decompress([128, 128, 128, 255]);
        assert!((q[3] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn future_nodes_advance_by_frames() {
        let path = straight(5, 10);
        let mut ratio = 0.5;
        assert_eq!(future_node_index(&path, 1, &mut ratio, 3.0), 1);
        assert!((ratio - 0.8).abs() < 1e-6);
        let mut ratio = 0.5;
        assert_eq!(future_node_index(&path, 1, &mut ratio, 8.0), 2);
        assert!((ratio - 0.3).abs() < 1e-6);
        let mut ratio = 0.0;
        assert_eq!(future_node_index(&path, 3, &mut ratio, 50.0), 4);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn frame_differences_measure_between_descriptions() {
        let path = straight(5, 10);
        assert_eq!(node_frame_difference(&path, 1, 0.5, 3, 0.25), 17.5);
        assert_eq!(node_frame_difference_unordered(&path, 3, 0.25, 1, 0.5), -17.5);
    }

    #[test]
    fn interpolation_follows_a_straight_line() {
        let path = straight(3, 10);
        let mut node = constructed_node();
        set_interpolated_node(&path, &mut node, 0, 0.5);
        assert!((node.position[0] - 0.5).abs() < 1e-5);
        assert!((node.direction[0] - 0.1).abs() < 1e-5);
    }

    #[test]
    fn approximation_projects_onto_the_nearer_segment() {
        let path = straight(4, 10);
        let mut ratio = 0.0;
        assert_eq!(approximated_node_index(&path, [1.25, 0.3, 0.0], 1, &mut ratio), 1);
        assert!((ratio - 0.25).abs() < 1e-6);
    }
}
