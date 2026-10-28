//! `AI::ObstacleAvoider` (0x1180 bytes): the obstacles near the path ahead and the avoidance
//! decision the path controller reads. `Update` 0x824c6010 gathers the other skaters, then the
//! pedestrians, vehicles and dynamic objects near the local skater (`living_world_obstacles`),
//! measures each against its path, then folds them into a pass-ahead/pass-behind speed window, a
//! skitch target, the free spans of the path width and one decision.
//!
//! `Obstacle::Set` also records whether both actors share a path and whether each has joined
//! one (`mBothActorsOnSamePath`, `mLocalActorJoinedToPath`, `mObstacleActorJoinedToPath`); nothing
//! reads them, so they are not kept.
use super::{
    attributes::AiSkaterAttributes,
    path::{self, InterpolatedNode, NodeDescription, UPDATE_HZ},
    vmx::{self, V4},
};
use skate2_formats::ai_path::AiPath;

/// `AI` cvars at their defaults (`startup.ini` sets none).
mod cvars {
    /// `kSkaterBodyRadius`.
    pub(super) const SKATER_BODY_RADIUS: f32 = 0.75;
    /// `kNumberOfFramesToPreventReSkitching`.
    pub(super) const NUMBER_OF_FRAMES_TO_PREVENT_RE_SKITCHING: u32 = 60;
    /// `kCollisionTimeToStartAvoidingDMO`.
    pub(super) const COLLISION_TIME_TO_START_AVOIDING_DMO: f32 = 1.0;
    /// `kbUseFuturePathVelocityToDecideSkitching` (`AI_Skater_Skitching`).
    pub(super) const USE_FUTURE_PATH_VELOCITY_TO_DECIDE_SKITCHING: bool = true;
}

/// `desiredVelFrameInc`: how far ahead `TestIfSkitchable` reads the path's velocity.
const DESIRED_VEL_FRAME_INC: i32 = 3;
/// `CalculatePathObstructionBox`'s starting width-ratio bounds.
const BOX_RATIO_START: f32 = 10000.0;

const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
/// `Obstacle::Set`'s head-on limit, a fifth of π.
const HEAD_ON_ANGLE: f32 = f32::from_bits(0x3f20_d97c);
const SMALL: f32 = f32::from_bits(0x38d1_b717);
const MINIMUM_SPEED: f32 = f32::from_bits(0x3a83_126f);
const FRAME_TIME: f32 = f32::from_bits(0x3c88_8889);
const J: V4 = [0.0, 1.0, 0.0, 0.0];

/// `EAvoidDecision`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AvoidDecision {
    #[default]
    NoAvoid,
    GoAhead,
    GoBehind,
    GoAround,
    Skitch,
    Blocked,
    RunOver,
    JumpOver,
    GiveUpAndBail,
}

/// `eObstacleType` (`OBSTACLE_TEST`, 1, belongs to the unported test skaters).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ObstacleType {
    #[default]
    Unknown,
    Pedestrian,
    Skater,
    Vehicle,
    Dmo,
}

/// `AIOut_Audio` (40 bytes, `AIOut::AIOut` 0x824bfe50): the avoidance calls the controller's
/// skater would shout. Word 0's bits 22..31 start set; word 9's top seven bits are the flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AiOutAudio {
    pub(crate) words: [u32; 10],
}

impl Default for AiOutAudio {
    fn default() -> Self {
        let mut words = [0u32; 10];
        words[0] = 0xfffe_0000;
        words[1..6].fill(u32::MAX);
        Self { words }
    }
}

impl AiOutAudio {
    /// `AISkaterManager::WipeoutObservedEvent` 0x824c35f0.
    pub(crate) fn wipeout_seen(&mut self, observed: u32, character: i32) {
        self.words[9] |= 0x1000_0000;
        self.words[0] |= 0x0010_0000;
        self.words[1] = observed;
        self.words[0] |= 0x8000_0000;
        self.words[2] = character.cast_unsigned();
        self.words[0] |= 0x4000_0000;
    }

    /// `AISkaterManager::RespawnObservedEvent` 0x824ba248.
    pub(crate) fn respawn_seen(&mut self, observed: u32, character: i32) {
        self.words[9] |= 0x0800_0000;
        self.words[0] |= 0x0008_0000;
        self.words[3] = observed;
        self.words[0] |= 0x2000_0000;
        self.words[4] = character.cast_unsigned();
        self.words[0] |= 0x1000_0000;
    }

    /// `AISkaterManager::TrickObservedEvent` 0x824ba2c0.
    pub(crate) fn big_trick_seen(&mut self, observed: u32, score: i32, character: i32) {
        self.words[9] |= 0x0400_0000;
        self.words[0] |= 0x0004_0000;
        self.words[5] = observed;
        self.words[0] |= 0x0800_0000;
        self.words[6] = score.cast_unsigned();
        self.words[0] |= 0x0400_0000;
        self.words[8] = character.cast_unsigned();
        self.words[0] |= 0x0100_0000;
    }

    /// `UpdateAudioEvents` 0x824c3668 on the trickster's own output.
    pub(crate) fn big_trick_completed(&mut self, score: i32) {
        self.words[9] |= 0x0200_0000;
        self.words[0] |= 0x0002_0000;
        self.words[7] = score.cast_unsigned();
        self.words[0] |= 0x0200_0000;
    }
}

/// The local skater as `Update` reads it: its Actor and its path controller.
pub(crate) struct AvoiderLocal<'a> {
    pub(crate) player_id: i32,
    /// `Actor::GetPosition`, and the Y axis of `Actor::GetTransform`.
    pub(crate) position: V4,
    pub(crate) up: V4,
    pub(crate) path: &'a AiPath,
    /// `PathController::mVelocity`, `mEffectiveActorDirection`, `mCurrentNodeIndex`,
    /// `mPathWidthRatio` and `mIsOffBoard`.
    pub(crate) velocity: V4,
    pub(crate) effective_actor_direction: V4,
    pub(crate) current_node_index: i32,
    pub(crate) path_width_ratio: f32,
    pub(crate) is_off_board: bool,
    /// `PathController::mHasJoinedPath`.
    pub(crate) has_joined_path: bool,
    /// `PhysOut_State::mCollideWithWorldOnly`.
    pub(crate) collide_with_world_only: bool,
}

/// Another skater of `SkaterManager` as `AddSkaterObstacles` reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ObstacleSkater {
    pub(crate) player_id: i32,
    /// `Actor::GetPosition`.
    pub(crate) position: V4,
    /// `AIController::GetActorVelocity` 0x824b3c88 (the active path controller's velocity) for
    /// an AI skater, else `Actor::GetVelocity`.
    pub(crate) velocity: V4,
    /// `Actor::GetEffectiveDeckTransform`.
    pub(crate) effective_deck_transform: [V4; 4],
    /// `PhysOut_State::mCollideWithWorldOnly` and `mIsWipingOut`.
    pub(crate) collide_with_world_only: bool,
    pub(crate) is_wiping_out: bool,
    /// `PhysOut_Skeleton::mSkeletonHipsPos`, `mSkeletonHipsVel` and `mSkeletonHipsLook`.
    pub(crate) hips_position: V4,
    pub(crate) hips_velocity: V4,
    pub(crate) hips_look: V4,
}

const IDENTITY: [V4; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]];

/// `AI::ProxyObstacle` (176 bytes): what `Obstacle::Set` takes of an obstacle. Its
/// `mAIController` feeds only the unkept path-sharing fields.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProxyObstacle {
    pub(crate) position: V4,
    pub(crate) velocity: V4,
    pub(crate) heading: V4,
    pub(crate) aabb_min: V4,
    pub(crate) aabb_max: V4,
    pub(crate) transform: [V4; 4],
    pub(crate) width: f32,
    pub(crate) length: f32,
    pub(crate) radius: f32,
    pub(crate) obstacle_type: ObstacleType,
    pub(crate) can_run_over: bool,
    pub(crate) can_jump_over: bool,
}

impl Default for ProxyObstacle {
    /// `ProxyObstacle::ProxyObstacle` 0x824b3780: zero vectors, identity axes at the origin.
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            velocity: [0.0; 4],
            heading: [0.0; 4],
            aabb_min: [0.0; 4],
            aabb_max: [0.0; 4],
            transform: IDENTITY,
            width: 0.0,
            length: 0.0,
            radius: 0.0,
            obstacle_type: ObstacleType::Unknown,
            can_run_over: false,
            can_jump_over: false,
        }
    }
}

impl ProxyObstacle {
    /// `ProxyObstacle(const Actor*)` 0x824bba30: a skater heading along its effective deck,
    /// 0.75 wide, 1 long and 0.7 round, its box the radius around it.
    fn skater(skater: &ObstacleSkater) -> Self {
        let radius = f32::from_bits(0x3f33_3333);
        Self {
            position: skater.position,
            velocity: skater.velocity,
            heading: skater.effective_deck_transform[2],
            aabb_min: skater.position.map(|lane| lane - radius),
            aabb_max: skater.position.map(|lane| lane + radius),
            transform: skater.effective_deck_transform,
            width: 0.75,
            length: 1.0,
            radius,
            obstacle_type: ObstacleType::Skater,
            can_run_over: false,
            can_jump_over: false,
        }
    }

    /// The fallen body `AddSkaterObstacles` builds for a wiping-out skater: its hips,
    /// `kSkaterBodyRadius` round.
    fn body(skater: &ObstacleSkater) -> Self {
        let radius = cvars::SKATER_BODY_RADIUS;
        Self {
            position: skater.hips_position,
            velocity: skater.hips_velocity,
            heading: skater.hips_look,
            width: radius * 2.0,
            length: radius * 2.0,
            radius,
            obstacle_type: ObstacleType::Skater,
            ..Self::default()
        }
    }
}

/// A Living World obstacle and the distance `TestObstacle` takes with it, in the order the
/// native adders test them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EntityObstacle {
    pub(crate) proxy: ProxyObstacle,
    pub(crate) distance: f32,
}

/// One `AI::Obstacle` slot (272 bytes).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Obstacle {
    obstacle_type: ObstacleType,
    /// `mNearestPathNode`.
    pub(crate) nearest_path_node: InterpolatedNode,
    position: V4,
    velocity: V4,
    heading: V4,
    aabb_min: V4,
    aabb_max: V4,
    transform: [V4; 4],
    width: f32,
    length: f32,
    distance: f32,
    angle_from_forward: f32,
    min_pass_ahead_speed: f32,
    max_pass_behind_speed: f32,
    min_passable_width_ratio: f32,
    max_passable_width_ratio: f32,
    approach_speed: f32,
    time_to_collision: f32,
    is_approaching: bool,
    must_be_avoided_now: bool,
    min_pass_ahead_speed_set: bool,
    max_pass_behind_speed_set: bool,
    overlaps_path: bool,
    is_approaching_head_on: bool,
    is_skitchable: bool,
    can_run_over: bool,
    can_jump_over: bool,
}

impl Default for Obstacle {
    /// `Obstacle::Obstacle` 0x824c89c8.
    fn default() -> Self {
        Self {
            obstacle_type: ObstacleType::Unknown,
            nearest_path_node: InterpolatedNode { node: path::constructed_node(), description: NodeDescription::default() },
            position: [0.0; 4],
            velocity: [0.0; 4],
            heading: [0.0; 4],
            aabb_min: [0.0; 4],
            aabb_max: [0.0; 4],
            transform: IDENTITY,
            width: 0.0,
            length: 0.0,
            distance: 0.0,
            angle_from_forward: 0.0,
            min_pass_ahead_speed: 0.0,
            max_pass_behind_speed: 0.0,
            min_passable_width_ratio: 0.0,
            max_passable_width_ratio: 0.0,
            approach_speed: 0.0,
            time_to_collision: 0.0,
            is_approaching: false,
            must_be_avoided_now: false,
            min_pass_ahead_speed_set: false,
            max_pass_behind_speed_set: false,
            overlaps_path: false,
            is_approaching_head_on: false,
            is_skitchable: false,
            can_run_over: false,
            can_jump_over: false,
        }
    }
}

fn unsigned_angle(a: V4, b: V4) -> f32 {
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    crate::reckoning_numeric::try_unsigned_angle(a, b, &mut numeric).unwrap_or(0.0)
}

fn signed_angle(a: V4, b: V4, axis: V4) -> f32 {
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    crate::reckoning_numeric::try_signed_angle(a, b, axis, &mut numeric).unwrap_or(0.0)
}

/// The near point of a lane-split comparison: `a >= b ? x : y` as `fsel` on `a - b`.
fn select(difference: f32, ge: f32, lt: f32) -> f32 {
    if difference >= 0.0 { ge } else { lt }
}

impl Obstacle {
    /// `Set` 0x824c42e0: the proxy's fields, then its angle to the local path velocity, its
    /// place on the local path and the approach, skitch and avoidance-speed tests.
    fn set(
        &mut self,
        local: &AvoiderLocal<'_>,
        attributes: &AiSkaterAttributes,
        obstacle: &ProxyObstacle,
        distance: f32,
        angle_from_forward: f32,
    ) {
        self.obstacle_type = obstacle.obstacle_type;
        self.position = obstacle.position;
        self.velocity = obstacle.velocity;
        self.heading = obstacle.heading;
        self.width = obstacle.width;
        self.length = obstacle.length;
        self.aabb_min = obstacle.aabb_min;
        self.aabb_max = obstacle.aabb_max;
        self.transform = obstacle.transform;
        self.can_run_over = obstacle.can_run_over;
        self.can_jump_over = obstacle.can_jump_over;
        let mut angle = unsigned_angle(self.heading, local.velocity);
        if !(angle < HALF_PI) {
            angle = PI - angle;
        }
        self.distance = distance;
        self.is_approaching_head_on = angle.abs() < HEAD_ON_ANGLE;
        self.angle_from_forward = angle_from_forward;
        self.find_on_path(local, attributes);
        self.test_if_approaching(local, attributes);
        self.test_if_skitchable(local);
        self.min_pass_ahead_speed_set = false;
        self.max_pass_behind_speed_set = false;
        self.must_be_avoided_now = false;
        if self.is_approaching && self.overlaps_path {
            self.calculate_min_max_avoidance_speeds(local, attributes);
            let size = if self.is_approaching_head_on { self.length } else { self.width };
            let gap = self.distance - size * 0.5;
            let near_angle = attributes.must_avoid_near_angle_degrees * DEGREES_TO_RADIANS;
            if gap < attributes.must_avoid_distance
                || (!(gap >= attributes.must_avoid_near_distance) && !(self.angle_from_forward.abs() >= near_angle))
            {
                self.must_be_avoided_now = true;
            }
        }
    }

    /// `FindOnPath` 0x824c4238: the nearest place on the local path within 180 frames of its
    /// current node, then `CalculatePathObstruction`, or for a vehicle or dynamic object
    /// `CalculatePathObstructionBox`.
    fn find_on_path(&mut self, local: &AvoiderLocal<'_>, attributes: &AiSkaterAttributes) {
        let position = vmx::xyz(self.position);
        let nearest = path::nearest_node_index_within_frames(local.path, position, local.current_node_index, 180);
        let mut ratio = 0.0;
        let index = path::approximated_node_index(local.path, position, nearest, &mut ratio);
        self.nearest_path_node.description = NodeDescription { index, ratio };
        path::set_interpolated_node(local.path, &mut self.nearest_path_node.node, index, ratio);
        if matches!(self.obstacle_type, ObstacleType::Vehicle | ObstacleType::Dmo) {
            self.calculate_path_obstruction_box(local, attributes);
        } else {
            self.calculate_path_obstruction(attributes);
        }
    }

    /// `CalculatePathObstructionBox` 0x824c3a70: out of reach of the path's width at its node
    /// (grown by half the longer side and the local skater's radius), no overlap. Otherwise each
    /// corner of the transformed box, measured across the path at its own nearest place, widens
    /// the span of width ratios blocked; a corner inside the skater's radius counts on both
    /// sides. The span overlaps the path when it meets [-1, 1].
    fn calculate_path_obstruction_box(&mut self, local: &AvoiderLocal<'_>, attributes: &AiSkaterAttributes) {
        let radius = attributes.local_skater_radius;
        let node = &self.nearest_path_node.node;
        let offset = vmx::sub(self.position, vmx::v3(node.position));
        let squared = vmx::dot3(offset, offset);
        let width = if vmx::dot3(offset, vmx::cross(vmx::v3(node.direction), J)) > 0.0 {
            node.width_right
        } else {
            node.width_left
        };
        let size = select(self.width - self.length, self.width, self.length);
        let reach = f32::from(width) * path::METRES_PER_NODE_WIDTH_UNIT + size.mul_add(0.5, radius);
        if squared >= reach * reach {
            self.overlaps_path = false;
            return;
        }
        let (min, max) = (self.aabb_min, self.aabb_max);
        let corners = [
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ];
        let unit = path::METRES_PER_NODE_WIDTH_UNIT;
        let count = path::node_count(local.path);
        let mut lowest = BOX_RATIO_START;
        let mut highest = -BOX_RATIO_START;
        for [x, y, z] in corners {
            let t = &self.transform;
            let corner: V4 = std::array::from_fn(|lane| {
                t[2][lane].mul_add(z, t[1][lane].mul_add(y, t[0][lane].mul_add(x, t[3][lane])))
            });
            let position = vmx::xyz(corner);
            let nearest = path::nearest_node_index_within_frames(local.path, position, local.current_node_index, 180);
            let mut ratio = 0.0;
            let index = path::approximated_node_index(local.path, position, nearest, &mut ratio);
            let (at, direction, right, left) = if index + 1 < count {
                let first = path::node(local.path, index);
                let second = path::node(local.path, index + 1);
                let (at, direction) = path::interpolated_pos_and_direction(first, second, ratio);
                let right = f32::from(first.width_right) * unit;
                let left = f32::from(first.width_left) * unit;
                (
                    at,
                    direction,
                    f32::from(second.width_right).mul_add(unit, -right).mul_add(ratio, right),
                    f32::from(second.width_left).mul_add(unit, -left).mul_add(ratio, left),
                )
            } else {
                let last = path::node(local.path, count - 1);
                (
                    vmx::v3(last.position),
                    vmx::v3(last.direction),
                    f32::from(last.width_right) * unit,
                    f32::from(last.width_left) * unit,
                )
            };
            let offset = vmx::sub(corner, at);
            let across = [offset[0], 0.0, offset[2], 0.0];
            let distance = vmx::sqrt_guarded(vmx::dot3(across, across));
            let right_side = vmx::dot3(across, vmx::cross(direction, J)) > 0.0;
            let within = distance < radius;
            let side = if right_side { distance } else { -distance };
            let (low_width, high_width) = match (right_side, within) {
                (true, false) => (right, right),
                (false, false) => (left, left),
                _ => (left, right),
            };
            let low = if low_width <= SMALL { -(radius - side) } else { -((radius - side) / low_width) };
            if !(low >= lowest) {
                lowest = low;
            }
            let high = if high_width <= SMALL { side + radius } else { (side + radius) / high_width };
            if !(high <= highest) {
                highest = high;
            }
        }
        self.min_passable_width_ratio = lowest;
        self.max_passable_width_ratio = highest;
        self.overlaps_path = !(lowest > 1.0) && highest >= -1.0;
    }

    /// `CalculatePathObstruction` 0x824bbb98: whether the obstacle, grown by the local skater's
    /// radius, reaches into the path's width at its node, and the span of width ratios it
    /// blocks, signed by its side of the path direction.
    fn calculate_path_obstruction(&mut self, attributes: &AiSkaterAttributes) {
        let size = if self.is_approaching_head_on { self.width } else { self.length };
        let half = size * 0.5 + attributes.local_skater_radius;
        let node = &self.nearest_path_node.node;
        let left = f32::from(node.width_left) * path::METRES_PER_NODE_WIDTH_UNIT;
        let right = f32::from(node.width_right) * path::METRES_PER_NODE_WIDTH_UNIT;
        let width = select(left - right, left, right);
        let offset = vmx::sub(self.position, vmx::v3(node.position));
        let squared = vmx::dot3(offset, offset);
        let reach = width + half;
        if squared >= reach * reach {
            self.overlaps_path = false;
            return;
        }
        self.overlaps_path = true;
        let direction = vmx::v3(node.direction);
        let unit = vmx::scale(direction, vmx::rsqrt(vmx::dot3(direction, direction)));
        let distance = vmx::sqrt_guarded(squared);
        let side = if 0.0 > vmx::dot3(offset, vmx::cross(unit, J)) { -distance } else { distance };
        let low = half - side;
        let high = side + half;
        if width > SMALL {
            let inverse = 1.0 / width;
            self.max_passable_width_ratio = inverse * high;
            self.min_passable_width_ratio = -(inverse * low);
        } else {
            self.min_passable_width_ratio = -low;
            self.max_passable_width_ratio = high;
        }
    }

    /// `TestIfApproaching` 0x824bbee8: whether the gap closes over the next frame, and the time
    /// until the local skater's radius meets the obstacle's near side.
    fn test_if_approaching(&mut self, local: &AvoiderLocal<'_>, attributes: &AiSkaterAttributes) {
        let local_step = vmx::scale(local.velocity, FRAME_TIME);
        let next = vmx::madd(self.velocity, [FRAME_TIME; 4], self.position);
        let local_next = vmx::add(local.position, local_step);
        let next_distance = vmx::length3(vmx::sub(next, local_next));
        self.approach_speed = (self.distance - next_distance) * UPDATE_HZ;
        if self.approach_speed <= 0.0 {
            self.is_approaching = false;
            self.time_to_collision = f32::MAX;
            return;
        }
        self.is_approaching = true;
        let size = if self.is_approaching_head_on { self.length } else { self.width };
        let gap = self.distance - (size * 0.5 + attributes.local_skater_radius);
        self.time_to_collision = if gap <= 0.0 { 0.0 } else { gap / self.approach_speed };
    }

    /// `TestIfSkitchable` 0x824bc100: a moving vehicle within about 37 degrees of the local
    /// skater's course, itself heading that way. The course is the path's velocity a few frames
    /// ahead once the skater has joined it.
    fn test_if_skitchable(&mut self, local: &AvoiderLocal<'_>) {
        self.is_skitchable = false;
        if self.obstacle_type != ObstacleType::Vehicle {
            return;
        }
        let mut course = local.velocity;
        if cvars::USE_FUTURE_PATH_VELOCITY_TO_DECIDE_SKITCHING && local.has_joined_path {
            let last = path::node_count(local.path) - 1;
            let ahead = local.current_node_index + DESIRED_VEL_FRAME_INC;
            course = path::velocity(path::node(local.path, if ahead < last { ahead } else { last }));
        }
        let obstacle_speed = vmx::length3(self.velocity);
        if obstacle_speed < SMALL {
            return;
        }
        let course_speed = vmx::length3(course);
        if course_speed < SMALL {
            return;
        }
        let toward = vmx::sub(self.position, local.position);
        let toward_inverse = 1.0 / (vmx::length3(toward) * course_speed);
        if (toward_inverse * vmx::dot3(course, toward)).clamp(-1.0, 1.0) <= 0.8 {
            return;
        }
        let heading_inverse = 1.0 / (course_speed * obstacle_speed);
        if !((heading_inverse * vmx::dot3(self.velocity, course)).clamp(-1.0, 1.0) <= 0.8) {
            self.is_skitchable = true;
        }
    }

    /// `CalculateMinMaxAvoidanceSpeeds` 0x824bc418: where the obstacle's course crosses the
    /// local skater's line, the speed that reaches it before the obstacle's near edge and the
    /// speed that reaches it only after its far edge.
    fn calculate_min_max_avoidance_speeds(&mut self, local: &AvoiderLocal<'_>, attributes: &AiSkaterAttributes) {
        let position = local.position;
        let obstacle_speed = vmx::length3(self.velocity);
        if obstacle_speed < MINIMUM_SPEED {
            return;
        }
        let local_velocity = local.velocity;
        let local_squared = vmx::dot3(local_velocity, local_velocity);
        let local_speed = vmx::sqrt_guarded(local_squared);
        if local_speed < MINIMUM_SPEED {
            return;
        }
        let along = vmx::reciprocal(local_squared) * vmx::dot3(vmx::sub(self.position, position), local_velocity);
        let closest = vmx::madd(local_velocity, [along; 4], position);
        let perpendicular = vmx::length3(vmx::sub(closest, self.position));
        if perpendicular < MINIMUM_SPEED {
            return;
        }
        let inverse = 1.0 / (local_speed * obstacle_speed);
        let cosine = (inverse * vmx::dot3(self.velocity, local_velocity)).clamp(-1.0, 1.0);
        let sine = vmx::sin(vmx::acos(cosine));
        if sine < SMALL {
            return;
        }
        let inverse_sine = 1.0 / sine;
        let inverse_speed = 1.0 / obstacle_speed;
        let crossing = vmx::madd(self.velocity, [perpendicular * inverse_sine * inverse_speed; 4], self.position);
        let to_crossing = vmx::sub(crossing, position);
        let crossing_distance = vmx::length3(to_crossing);
        let behind = !(crossing_distance <= MINIMUM_SPEED) && 0.0 > vmx::dot3(to_crossing, local_velocity);
        let radius = attributes.local_skater_radius;
        let size = if self.is_approaching_head_on { self.width } else { self.length };
        let half = size * 0.5 + radius;
        let adjustment = attributes.avoidance_speed_adjustment;
        if perpendicular > half + SMALL || (behind && !(perpendicular <= radius)) {
            self.min_pass_ahead_speed_set = true;
            self.min_pass_ahead_speed = if behind {
                0.0
            } else {
                crossing_distance / ((perpendicular - half) * inverse_sine * inverse_speed)
            };
            self.min_pass_ahead_speed += adjustment;
        }
        let far_time = (perpendicular + half) * inverse_sine * inverse_speed;
        self.max_pass_behind_speed_set = true;
        let extent = if self.is_approaching_head_on { self.length } else { self.width };
        let gap = crossing_distance - extent * 0.5;
        let gap = select(-gap, 0.0, gap);
        let speed = gap / far_time - adjustment;
        self.max_pass_behind_speed = select(-speed, 0.0, speed);
    }
}

/// `AI::WidthTuple`: a free span of path width ratios.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct WidthTuple {
    min: f32,
    max: f32,
}

impl WidthTuple {
    /// `ClipWithObstacle` 0x824b36c0.
    fn clip_with_obstacle(&mut self, min: f32, max: f32) {
        if min <= self.min && max >= self.min {
            self.min = max;
        }
        if min <= self.max && max >= self.max {
            self.max = min;
        }
    }
}

/// `AI::NavigableWidthRatios`: up to eight free spans across the path, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct NavigableWidthRatios {
    tuples: [WidthTuple; 8],
    count: usize,
}

impl NavigableWidthRatios {
    /// `AddObstacle` 0x824bb720: spans the obstacle covers go, the span it lies inside splits
    /// and the others clip; the split's upper half joins before the first span above it.
    fn add_obstacle(&mut self, min: f32, max: f32) {
        let mut removed = [false; 8];
        let mut split = None;
        for (index, tuple) in self.tuples[..self.count].iter_mut().enumerate() {
            if !(min > tuple.min) && max >= tuple.max {
                removed[index] = true;
            } else if !(min <= tuple.min) && max < tuple.max {
                split = Some(WidthTuple { min: max, max: tuple.max });
                tuple.max = min;
            } else {
                tuple.clip_with_obstacle(min, max);
            }
        }
        let mut rebuilt = [WidthTuple::default(); 8];
        let mut count = 0;
        for (index, tuple) in self.tuples[..self.count].iter().enumerate() {
            if removed[index] {
                continue;
            }
            if let Some(upper) = split.filter(|upper| upper.min < tuple.min) {
                if count < 8 {
                    rebuilt[count] = upper;
                    count += 1;
                }
                split = None;
            }
            if count < 8 {
                rebuilt[count] = *tuple;
                count += 1;
            }
        }
        if let Some(upper) = split
            && count < 8
        {
            rebuilt[count] = upper;
            count += 1;
        }
        self.tuples = rebuilt;
        self.count = count;
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ObstacleAvoider {
    navigable_width_ratios: NavigableWidthRatios,
    obstacles: [Obstacle; 16],
    obstacle_count: usize,
    pub(crate) first_path_width_obstacle_index: usize,
    pub(crate) first_skitch_obstacle_index: usize,
    prevent_skitching_num_frames_remaining: u32,
    pub(crate) desired_speed: f32,
    pub(crate) min_speed_to_pass_infront_of_all_obstacles: f32,
    pub(crate) max_speed_to_pass_behind_of_all_obstacles: f32,
    pub(crate) width_ratio_used: f32,
    pub(crate) avoid_decision: AvoidDecision,
    pass_ahead_possible: bool,
    pass_behind_possible: bool,
    pass_around_possible: bool,
    pub(crate) imminent_trick_launch: bool,
    should_attempt_skitch: bool,
    prevent_skitching: bool,
}

/// Whether `a`'s place on the path lies beyond `b`'s.
fn is_further_along(a: &NodeDescription, b: &NodeDescription) -> bool {
    a.index > b.index || (a.index == b.index && a.ratio > b.ratio)
}

impl ObstacleAvoider {
    /// `Update` 0x824c6010: the skaters, then the Living World obstacles in the native adders'
    /// order.
    pub(crate) fn update(
        &mut self,
        local: &AvoiderLocal<'_>,
        skaters: &[ObstacleSkater],
        entities: &[EntityObstacle],
        attributes: &AiSkaterAttributes,
        desired_speed: f32,
        imminent_trick_launch: bool,
    ) {
        self.desired_speed = desired_speed;
        self.imminent_trick_launch = imminent_trick_launch;
        self.min_speed_to_pass_infront_of_all_obstacles = 0.0;
        self.obstacle_count = 0;
        self.max_speed_to_pass_behind_of_all_obstacles = f32::MAX;
        self.first_path_width_obstacle_index = 0;
        self.first_skitch_obstacle_index = 0;
        self.avoid_decision = AvoidDecision::NoAvoid;
        self.add_skater_obstacles(local, skaters, attributes);
        for entity in entities {
            self.test_obstacle(local, attributes, &entity.proxy, entity.distance);
        }
        self.calculate_speed_adjust(attributes);
        self.calculate_skitching();
        self.calculate_safe_position();
        self.pick_avoidance_strategy(local);
    }

    /// `AddSkaterObstacles` 0x824c5928: every other skater not colliding with the world only
    /// within the gathering radius, and the fallen body of one wiping out.
    fn add_skater_obstacles(&mut self, local: &AvoiderLocal<'_>, skaters: &[ObstacleSkater], attributes: &AiSkaterAttributes) {
        let position = local.position;
        let range = attributes.obstacle_radius * attributes.obstacle_radius;
        for skater in skaters {
            if skater.player_id == local.player_id || skater.collide_with_world_only {
                continue;
            }
            let offset = vmx::sub(position, skater.position);
            let squared = vmx::dot3(offset, offset);
            if !(squared >= range) {
                self.test_obstacle(local, attributes, &ProxyObstacle::skater(skater), vmx::sqrt_guarded(squared));
            }
            if skater.is_wiping_out {
                let offset = vmx::sub(position, skater.hips_position);
                let squared = vmx::dot3(offset, offset);
                if !(squared >= range) {
                    self.test_obstacle(local, attributes, &ProxyObstacle::body(skater), vmx::sqrt_guarded(squared));
                }
            }
        }
    }

    /// `TestObstacle` 0x824c4700: an obstacle well ahead of the local skater's heading, or
    /// close and inside a wider angle, takes the next slot.
    fn test_obstacle(&mut self, local: &AvoiderLocal<'_>, attributes: &AiSkaterAttributes, obstacle: &ProxyObstacle, distance: f32) {
        let forward = if local.is_off_board {
            local.effective_actor_direction
        } else {
            let speed = vmx::length3(local.velocity);
            if SMALL > speed {
                return;
            }
            vmx::scale(local.velocity, vmx::reciprocal(speed))
        };
        let radius = obstacle.radius;
        let toward = vmx::madd(forward, [radius; 4], vmx::sub(obstacle.position, local.position));
        if SMALL > vmx::dot3(toward, toward) {
            return;
        }
        let mut angle = signed_angle(forward, toward, local.up);
        if angle > PI {
            angle -= TWO_PI;
        }
        let magnitude = angle.abs();
        let ahead = attributes.obstacle_ahead_angle_degrees * DEGREES_TO_RADIANS;
        let near = attributes.obstacle_near_angle_degrees * DEGREES_TO_RADIANS;
        if (magnitude < ahead
            || (!(magnitude >= near) && !(distance - radius >= attributes.obstacle_near_distance)))
            && self.obstacle_count < self.obstacles.len()
        {
            self.obstacles[self.obstacle_count].set(local, attributes, obstacle, distance, angle);
            self.obstacle_count += 1;
        }
    }

    /// `CalculateSpeedAdjust` 0x824bcae0: the fastest pass-ahead minimum and the slowest
    /// pass-behind maximum over the approaching obstacles; passing ahead is out when it needs
    /// more than the desired speed and the margin.
    fn calculate_speed_adjust(&mut self, attributes: &AiSkaterAttributes) {
        self.pass_ahead_possible = true;
        self.pass_behind_possible = true;
        for obstacle in &self.obstacles[..self.obstacle_count] {
            if !obstacle.is_approaching {
                continue;
            }
            if obstacle.min_pass_ahead_speed_set {
                let minimum = self.min_speed_to_pass_infront_of_all_obstacles;
                self.min_speed_to_pass_infront_of_all_obstacles =
                    select(minimum - obstacle.min_pass_ahead_speed, minimum, obstacle.min_pass_ahead_speed);
            } else {
                self.pass_ahead_possible = false;
                if obstacle.must_be_avoided_now {
                    self.min_speed_to_pass_infront_of_all_obstacles = f32::MAX;
                }
            }
            if obstacle.max_pass_behind_speed_set {
                let maximum = self.max_speed_to_pass_behind_of_all_obstacles;
                self.max_speed_to_pass_behind_of_all_obstacles =
                    select(maximum - obstacle.max_pass_behind_speed, obstacle.max_pass_behind_speed, maximum);
            } else {
                self.pass_behind_possible = false;
                if obstacle.must_be_avoided_now {
                    self.max_speed_to_pass_behind_of_all_obstacles = 0.0;
                }
            }
        }
        if !(self.desired_speed + attributes.pass_ahead_speed_margin >= self.min_speed_to_pass_infront_of_all_obstacles) {
            self.pass_ahead_possible = false;
            self.min_speed_to_pass_infront_of_all_obstacles = f32::MAX;
        }
    }

    /// `CalculateSkitching` 0x824b3a00: the skitchable obstacle nearest along the path, with
    /// `kNumberOfFramesToPreventReSkitching` frames before another once one is let go.
    fn calculate_skitching(&mut self) {
        if self.prevent_skitching_num_frames_remaining != 0 {
            self.prevent_skitching_num_frames_remaining -= 1;
        } else {
            self.prevent_skitching = false;
        }
        let was_attempting = self.should_attempt_skitch;
        self.should_attempt_skitch = false;
        self.first_skitch_obstacle_index = 0;
        if !self.prevent_skitching {
            for index in 0..self.obstacle_count {
                let obstacle = &self.obstacles[index];
                if !obstacle.is_skitchable {
                    continue;
                }
                let first = &self.obstacles[self.first_skitch_obstacle_index].nearest_path_node.description;
                if !self.should_attempt_skitch || is_further_along(first, &obstacle.nearest_path_node.description) {
                    self.first_skitch_obstacle_index = index;
                }
                self.should_attempt_skitch = true;
            }
        }
        if was_attempting && !self.should_attempt_skitch {
            self.prevent_skitching = true;
            self.prevent_skitching_num_frames_remaining = cvars::NUMBER_OF_FRAMES_TO_PREVENT_RE_SKITCHING;
        }
    }

    /// `CalculateSafePosition` 0x824bcc48: the free spans left by the obstacles overlapping the
    /// path, and the overlapping obstacle nearest along it.
    fn calculate_safe_position(&mut self) {
        self.navigable_width_ratios.count = 1;
        self.first_path_width_obstacle_index = 0;
        self.navigable_width_ratios.tuples[0] = WidthTuple { min: -1.0, max: 1.0 };
        for index in 0..self.obstacle_count {
            let obstacle = self.obstacles[index];
            if !obstacle.overlaps_path {
                continue;
            }
            self.navigable_width_ratios
                .add_obstacle(obstacle.min_passable_width_ratio, obstacle.max_passable_width_ratio);
            let first = &self.obstacles[self.first_path_width_obstacle_index].nearest_path_node.description;
            if is_further_along(first, &obstacle.nearest_path_node.description) {
                self.first_path_width_obstacle_index = index;
            }
        }
        self.pass_around_possible = self.navigable_width_ratios.count != 0;
    }

    /// `PickAvoidanceStrategy` 0x824bd330: skitch, else go around through the best free span,
    /// else go ahead, else go behind (`kbDoBlockedNavmeshTest` is off). Behind a dynamic object
    /// the skater runs or jumps over it when it allows that and is under
    /// `kCollisionTimeToStartAvoidingDMO` away (no avoidance until then); one it can do neither
    /// to that leaves no speed to pass behind, inside half a second, makes it give up and bail
    /// unless it collides with the world only.
    fn pick_avoidance_strategy(&mut self, local: &AvoiderLocal<'_>) {
        if self.obstacle_count == 0 {
            return;
        }
        if self.should_attempt_skitch {
            let obstacle = &self.obstacles[self.first_skitch_obstacle_index];
            self.avoid_decision = AvoidDecision::Skitch;
            let middle = (obstacle.max_passable_width_ratio + obstacle.min_passable_width_ratio) * 0.5;
            let floor = select(-1.0 - middle, -1.0, middle);
            self.width_ratio_used = select(1.0 - floor, floor, 1.0);
            return;
        }
        if self.pass_around_possible {
            self.width_ratio_used = self.pick_optimum_avoidance_width(local);
            self.avoid_decision = AvoidDecision::GoAround;
            return;
        }
        if self.pass_ahead_possible {
            self.avoid_decision = AvoidDecision::GoAhead;
            return;
        }
        self.avoid_decision = AvoidDecision::GoBehind;
        let obstacle = &self.obstacles[self.first_path_width_obstacle_index];
        let dmo = obstacle.obstacle_type == ObstacleType::Dmo;
        let soon = obstacle.time_to_collision < cvars::COLLISION_TIME_TO_START_AVOIDING_DMO;
        if dmo && obstacle.can_run_over {
            self.avoid_decision = if soon { AvoidDecision::RunOver } else { AvoidDecision::NoAvoid };
        } else if dmo && obstacle.can_jump_over {
            self.avoid_decision = if soon { AvoidDecision::JumpOver } else { AvoidDecision::NoAvoid };
        }
        if self.avoid_decision == AvoidDecision::GoBehind
            && self.max_speed_to_pass_behind_of_all_obstacles < 0.1
            && !self.imminent_trick_launch
            && dmo
        {
            self.avoid_decision = if local.collide_with_world_only || obstacle.time_to_collision >= 0.5 {
                AvoidDecision::NoAvoid
            } else {
                AvoidDecision::GiveUpAndBail
            };
        }
    }

    /// `PickOptimumAvoidanceWidth` 0x824bd090: the middle of the free span nearest the local
    /// skater's width ratio; off the path (two widths out) the span whose middle, at the first
    /// overlapping obstacle's node, lies nearest the skater.
    fn pick_optimum_avoidance_width(&self, local: &AvoiderLocal<'_>) -> f32 {
        let tuples = &self.navigable_width_ratios.tuples[..self.navigable_width_ratios.count];
        let ratio = local.path_width_ratio;
        let mut best = 0.0;
        if ratio.abs() >= 2.0 {
            let node = &self.obstacles[self.first_path_width_obstacle_index].nearest_path_node.node;
            let position = local.position;
            let mut nearest = f32::MAX;
            for tuple in tuples {
                let middle = (tuple.max + tuple.min) * 0.5;
                let offset = vmx::sub(position, path::wide_pos(node, middle));
                let squared = vmx::dot3(offset, offset);
                if !(squared >= nearest) {
                    best = middle;
                    nearest = squared;
                }
            }
            return best;
        }
        let mut nearest = f32::MAX;
        for tuple in tuples {
            let distance = if ratio >= tuple.min {
                if ratio <= tuple.max { 0.0 } else { ratio - tuple.max }
            } else {
                tuple.min - ratio
            };
            if !(distance >= nearest) {
                nearest = distance;
                best = (tuple.max + tuple.min) * 0.5;
            }
        }
        best
    }

    /// `AdjustDesiredSpeed` 0x824b3928: going ahead raises the speed to the minimum that
    /// passes in front; going behind lowers it to the maximum that passes behind, unless a
    /// trick launch is imminent.
    pub(crate) fn adjust_desired_speed(&self, desired: f32) -> (bool, f32) {
        match self.avoid_decision {
            AvoidDecision::GoAhead if !(desired >= self.min_speed_to_pass_infront_of_all_obstacles) => {
                (true, self.min_speed_to_pass_infront_of_all_obstacles)
            }
            AvoidDecision::GoBehind
                if !(desired <= self.max_speed_to_pass_behind_of_all_obstacles || self.imminent_trick_launch) =>
            {
                (true, self.max_speed_to_pass_behind_of_all_obstacles)
            }
            _ => (false, desired),
        }
    }

    /// `GetFirstObstacleNode` 0x824b3b00.
    pub(crate) fn first_obstacle_node(&self) -> InterpolatedNode {
        self.obstacles[self.first_path_width_obstacle_index].nearest_path_node
    }

    /// `OutputAvoidanceDecision` 0x824b3b60.
    pub(crate) fn output_avoidance_decision(&self, audio: &mut AiOutAudio) {
        match self.avoid_decision {
            AvoidDecision::GoBehind => {
                if self.max_speed_to_pass_behind_of_all_obstacles < self.desired_speed * 0.5 {
                    audio.words[9] |= 0x8000_0000;
                    audio.words[0] |= 0x0080_0000;
                }
            }
            AvoidDecision::GoAround => {
                if self.width_ratio_used.abs() > f32::from_bits(0x3e19_999a) {
                    audio.words[9] |= 0x4000_0000;
                    audio.words[0] |= 0x0040_0000;
                }
            }
            _ => {}
        }
    }

    /// `AdjustDesiredDestinationNode` 0x824c5ee0: around or blocked, toward the first path-width
    /// obstacle; skitching, toward the first skitch obstacle. A destination short of the
    /// obstacle slides sideways by the width ratio; past it, the nearer of the obstacle and the
    /// avoidance node does.
    pub(crate) fn adjust_desired_destination_node(
        &self,
        destination: &mut InterpolatedNode,
        avoidance: &InterpolatedNode,
    ) -> bool {
        let index = match self.avoid_decision {
            AvoidDecision::GoAround | AvoidDecision::Blocked => self.first_path_width_obstacle_index,
            AvoidDecision::Skitch => self.first_skitch_obstacle_index,
            _ => return false,
        };
        let nearest = self.obstacles[index].nearest_path_node;
        if is_further_along(&nearest.description, &destination.description) {
            destination.node.position = vmx::xyz(path::wide_pos(&destination.node, self.width_ratio_used));
        } else {
            let source = if is_further_along(&nearest.description, &avoidance.description) { nearest } else { *avoidance };
            let mut wide = *destination;
            path::set_wide(&mut wide, &source, self.width_ratio_used);
            *destination = wide;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratios(spans: &[(f32, f32)]) -> NavigableWidthRatios {
        let mut ratios = NavigableWidthRatios::default();
        for (index, &(min, max)) in spans.iter().enumerate() {
            ratios.tuples[index] = WidthTuple { min, max };
        }
        ratios.count = spans.len();
        ratios
    }

    fn spans(ratios: &NavigableWidthRatios) -> Vec<(f32, f32)> {
        ratios.tuples[..ratios.count].iter().map(|tuple| (tuple.min, tuple.max)).collect()
    }

    /// A path along +X, one metre per ten frames, two metres wide on each side.
    fn straight(count: usize) -> AiPath {
        AiPath {
            bbox_min: [0.0; 4],
            bbox_max: [0.0; 4],
            id: skate2_formats::ai_path::AiPathId([0; 16]),
            nodes: (0..count)
                .map(|i| skate2_formats::ai_path::AiPathNode {
                    position: [i as f32, 0.0, 0.0],
                    direction: [0.1, 0.0, 0.0],
                    board_orientation: [127, 127, 127, 255],
                    skater_orientation: [127, 127, 127, 255],
                    frame_count_since_last_node: 10,
                    width_left: 100,
                    width_right: 100,
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

    fn local(path: &AiPath) -> AvoiderLocal<'_> {
        AvoiderLocal {
            player_id: 1,
            position: [0.0, 0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0, 0.0],
            path,
            velocity: [5.0, 0.0, 0.0, 0.0],
            effective_actor_direction: [1.0, 0.0, 0.0, 0.0],
            current_node_index: 0,
            path_width_ratio: 0.0,
            is_off_board: false,
            has_joined_path: false,
            collide_with_world_only: false,
        }
    }

    #[test]
    fn a_box_beside_the_path_blocks_from_its_inside_edge_out() {
        let path = straight(12);
        let attributes = AiSkaterAttributes { local_skater_radius: 0.5, ..AiSkaterAttributes::default() };
        let mut obstacle = Obstacle {
            obstacle_type: ObstacleType::Dmo,
            position: [5.0, 0.5, 0.5, 1.0],
            aabb_min: [-0.5, 0.0, -0.5, 0.0],
            aabb_max: [0.5, 1.0, 0.5, 0.0],
            transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [5.0, 0.0, 0.5, 1.0]],
            width: 1.0,
            length: 1.0,
            ..Obstacle::default()
        };
        obstacle.find_on_path(&local(&path), &attributes);
        assert!(obstacle.overlaps_path);
        assert!((obstacle.min_passable_width_ratio + 0.25).abs() < 1e-5);
        assert!((obstacle.max_passable_width_ratio - 0.75).abs() < 1e-5);
    }

    #[test]
    fn a_dynamic_object_is_run_over_jumped_or_bailed_on() {
        let path = straight(2);
        let mut avoider = ObstacleAvoider { obstacle_count: 1, max_speed_to_pass_behind_of_all_obstacles: 0.05, ..ObstacleAvoider::default() };
        let decide = |avoider: &mut ObstacleAvoider, obstacle: Obstacle, world_only: bool| {
            avoider.obstacles[0] = obstacle;
            let mut local = local(&path);
            local.collide_with_world_only = world_only;
            avoider.pick_avoidance_strategy(&local);
            avoider.avoid_decision
        };
        let dmo = Obstacle { obstacle_type: ObstacleType::Dmo, time_to_collision: 0.3, ..Obstacle::default() };
        assert_eq!(decide(&mut avoider, Obstacle { can_run_over: true, ..dmo }, false), AvoidDecision::RunOver);
        assert_eq!(decide(&mut avoider, Obstacle { can_jump_over: true, ..dmo }, false), AvoidDecision::JumpOver);
        assert_eq!(decide(&mut avoider, Obstacle { can_run_over: true, time_to_collision: 2.0, ..dmo }, false), AvoidDecision::NoAvoid);
        assert_eq!(decide(&mut avoider, dmo, false), AvoidDecision::GiveUpAndBail);
        assert_eq!(decide(&mut avoider, dmo, true), AvoidDecision::NoAvoid);
        assert_eq!(decide(&mut avoider, Obstacle { time_to_collision: 0.6, ..dmo }, false), AvoidDecision::NoAvoid);
        let vehicle = Obstacle { obstacle_type: ObstacleType::Vehicle, ..dmo };
        assert_eq!(decide(&mut avoider, vehicle, false), AvoidDecision::GoBehind);
    }

    #[test]
    fn only_a_vehicle_heading_along_the_course_is_skitchable() {
        let path = straight(12);
        let skitch = |obstacle_type, velocity| {
            let mut obstacle = Obstacle { obstacle_type, position: [10.0, 0.0, 0.5, 1.0], velocity, ..Obstacle::default() };
            obstacle.test_if_skitchable(&local(&path));
            obstacle.is_skitchable
        };
        assert!(skitch(ObstacleType::Vehicle, [6.0, 0.0, 0.0, 0.0]));
        assert!(!skitch(ObstacleType::Vehicle, [-6.0, 0.0, 0.0, 0.0]));
        assert!(!skitch(ObstacleType::Vehicle, [0.0, 0.0, 6.0, 0.0]));
        assert!(!skitch(ObstacleType::Pedestrian, [6.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn an_obstacle_inside_a_span_splits_it_in_order() {
        let mut free = ratios(&[(-1.0, 1.0)]);
        free.add_obstacle(-0.25, 0.25);
        assert_eq!(spans(&free), [(-1.0, -0.25), (0.25, 1.0)]);
    }

    #[test]
    fn covered_spans_go_and_overlapped_ones_clip() {
        let mut free = ratios(&[(-1.0, -0.25), (0.25, 1.0)]);
        free.add_obstacle(-0.5, 0.5);
        assert_eq!(spans(&free), [(-1.0, -0.5), (0.5, 1.0)]);
        free.add_obstacle(-2.0, -0.5);
        assert_eq!(spans(&free), [(0.5, 1.0)]);
    }

    #[test]
    fn nearest_span_middle_wins_on_the_path() {
        let mut avoider = ObstacleAvoider::default();
        avoider.navigable_width_ratios = ratios(&[(-1.0, -0.5), (0.25, 1.0)]);
        avoider.obstacle_count = 1;
        let path = AiPath {
            bbox_min: [0.0; 4],
            bbox_max: [0.0; 4],
            id: skate2_formats::ai_path::AiPathId([0; 16]),
            nodes: Vec::new(),
            has_extra_node_data: false,
            branch_groups: Vec::new(),
            bit_flags: 0,
            allowed_skaters: u64::MAX,
            skill_level: 0,
            extra_data: [0; 3],
        };
        let local = AvoiderLocal {
            player_id: 1,
            position: [0.0; 4],
            up: [0.0, 1.0, 0.0, 0.0],
            path: &path,
            velocity: [0.0; 4],
            effective_actor_direction: [0.0; 4],
            current_node_index: 0,
            path_width_ratio: 0.1,
            is_off_board: false,
            has_joined_path: false,
            collide_with_world_only: false,
        };
        assert_eq!(avoider.pick_optimum_avoidance_width(&local), 0.625);
    }
}
