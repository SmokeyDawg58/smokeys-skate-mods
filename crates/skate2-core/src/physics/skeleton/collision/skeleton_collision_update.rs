//! Normal and ragdoll `SkeletonCollision::Update` (0x82b4b8f0..0x82b4c6fc).
//! Mutates the actual retained collision state. Contacts must come from the
//! world's sealed Island list; history/live bodies remain on the same owner.
//! Debug region reload remains an explicit unsupported boundary. Debug drawing
//! itself is outside this simulation projection.
use super::{SkeletonCollisionData, SkeletonCollisionNormalInfo, SkeletonCollisionVector};
use crate::{
    ragdoll::SkeletonPartIndex,
    simulation_assembly_backend::constraint_session::physical_contact_list::PhysicalContactInfo,
    simulation_assembly_backend::constraint_session::physical_island::SimulationIslandBodyHandle,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    skeleton_collision_numeric::{
        self as math, SkeletonCollisionClampError, SkeletonCollisionNumeric,
    },
    skeleton_collision_update_attributes::{
        SkeletonCollisionUpdateAttribute as Attribute, SkeletonCollisionUpdateAttributeRead,
    },
    skeleton_physics_history::SkeletonPhysicsHistory,
};
use std::fmt;

#[cfg(test)]
#[path = "skeleton_collision_update_tests.rs"]
mod tests;

/// Current `SkeletonCollisionUpdateInfo` projection. The actual Skeleton owner
/// supplies ragdoll status separately; these inputs are not a live Actor claim.
pub(crate) struct SkeletonCollisionUpdateInput {
    pub(crate) ignore_rigid_body: Option<SimulationRigidBodyObservation>,
    pub(crate) ground_position: [f32; 3],
    pub(crate) ground_normal: [f32; 3],
    pub(crate) system_velocity: [f32; 3],
    pub(crate) centre_of_mass_velocity: [f32; 3],
    pub(crate) timestep: f32,
    pub(crate) danger_zone: u8,
    pub(crate) ai_skater: u8,
    pub(crate) offboard: u8,
    pub(crate) offboard_air: u8,
    pub(crate) ground_trick: u8,
    pub(crate) request_partial_ragdoll: u8,
}

/// Native fence helper captures both targets before either value read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FenceThreshold {
    Distance,
    OpposingDot,
}

pub(crate) trait SkeletonCollisionCvars {
    type Error;
    type FenceTarget;
    fn capture_fence_threshold(
        &mut self,
        field: FenceThreshold,
    ) -> Result<Self::FenceTarget, Self::Error>;
    fn read_fence_threshold(&mut self, target: &Self::FenceTarget) -> Result<f32, Self::Error>;
    fn draw_body_part_collision(&mut self) -> Result<u8, Self::Error>;
    fn draw_collision_update(&mut self) -> Result<u8, Self::Error>;
    fn show_ground_plane(&mut self) -> Result<u8, Self::Error>;
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn allow_any_contact(&mut self) -> Result<u8, Self::Error>;
    fn maximum_ground_distance_offboard(&mut self) -> Result<f32, Self::Error>;
    fn small_object_force_scalar(&mut self) -> Result<f32, Self::Error>;
    fn debug_always_collide(&mut self) -> Result<u8, Self::Error>;
}

/// Borrowed actual pool/session reads. No alternative body or history owner.
pub(crate) trait SkeletonCollisionLiveRead {
    type Error;
    fn other_body_group(&mut self, body: &SimulationIslandBodyHandle) -> Result<u32, Self::Error>;
    fn raw_state(&mut self, body: SimulationRigidBodyObservation) -> Result<u32, Self::Error>;
    fn cached_inverse_mass(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<f32, Self::Error>;
    fn velocity(&mut self, body: SimulationRigidBodyObservation) -> Result<[f32; 3], Self::Error>;
    fn part_transform(&mut self, part: usize) -> Result<[f32; 16], Self::Error>;
}

#[derive(Debug)]
pub(crate) enum SkeletonCollisionUpdateError<A, C, N, L> {
    DebugRegionReloadUnsupported,
    InvalidPart(u32),
    Attribute(A),
    Cvar(C),
    Numeric(N),
    Clamp(SkeletonCollisionClampError<N>),
    Live(L),
}
impl<A: fmt::Debug, C: fmt::Debug, N: fmt::Debug, L: fmt::Debug> fmt::Display
    for SkeletonCollisionUpdateError<A, C, N, L>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DebugRegionReloadUnsupported => {
                f.write_str("debug-triggered collision region reload is not implemented")
            }
            Self::InvalidPart(part) => {
                write!(f, "collision Part {part} is outside Skeleton's 24 Parts")
            }
            Self::Attribute(error) => write!(f, "collision attribute: {error:?}"),
            Self::Cvar(error) => write!(f, "collision current cvar/overlay: {error:?}"),
            Self::Numeric(error) => write!(f, "collision numeric service: {error:?}"),
            Self::Clamp(error) => write!(f, "collision clamp: {error:?}"),
            Self::Live(error) => write!(f, "collision live input: {error:?}"),
        }
    }
}
impl<A: fmt::Debug, C: fmt::Debug, N: fmt::Debug, L: fmt::Debug> std::error::Error
    for SkeletonCollisionUpdateError<A, C, N, L>
{
}

type CollisionResult<T, A, C, N, L> = Result<T, SkeletonCollisionUpdateError<A, C, N, L>>;

const COLLISION_GROUP: [usize; 24] = [
    usize::MAX,
    0,
    1,
    2,
    2,
    2,
    1,
    3,
    3,
    3,
    1,
    1,
    1,
    1,
    1,
    6,
    6,
    4,
    4,
    7,
    7,
    5,
    5,
    1,
];

fn abs_bits(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & 0x7fff_ffff)
}
fn fsel(condition: f32, nonnegative: f32, negative: f32) -> f32 {
    if condition >= 0.0 {
        nonnegative
    } else {
        negative
    }
}

impl SkeletonCollisionData {
    /// Runtime prefix, NOT constructor Reset: priorities, `NormalInfo` and
    /// grouped audio surfaces retain their original selective persistence.
    fn clear_update_fields(&mut self, timestep: f32) {
        self.unrideable_contact = 0;
        self.opposing_contacts = 0;
        self.opposing_contacts_on_plane = 0;
        self.is_proxy_in_collision = 0;
        self.collision_priority = 0.0;
        self.skater_contact = 0.0;
        self.vehicle_contact = 0.0;
        self.is_hook_to_hips_valid = 0;
        self.pedestrian_contact = 0.0;
        self.foot_hit_skateboard = 0;
        self.most_vertical_toe_contact_normal = SkeletonCollisionVector::ZERO;
        for part in 0..24 {
            self.time_since_collision[part] -= timestep;
            self.just_collided[part] = 0;
            self.bone_data[part].reset();
        }
        for group in 0..8 {
            self.contact_force[group] = 0.0;
            self.contact_friction_force[group] = 0.0;
            self.slide_speed[group] = 0.0;
            self.contact_normal[group] = SkeletonCollisionVector::ZERO;
            self.contact_bone[group] = None;
        }
        self.most_vertical_contact_normal =
            SkeletonCollisionVector::known([-0.0, -1.0, -0.0, -0.0]);
        if self.is_wiping_out == 0 {
            self.time_in_wipeout = 0.0;
            self.contact_force_total_during_wipeout = 0.0;
            self.slide_force_total_during_wipeout = 0.0;
        }
    }

    fn update_specific_regions<C: SkeletonCollisionCvars>(
        &mut self,
        history: &SkeletonPhysicsHistory,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        for region in 0..2 {
            // 82b3ef04..28: oldRecent && wipingOut. Constructor-unknown
            // bytes remain unknown in ragdoll until an actual contact publishes.
            self.specific_parts.recent_contact[region] = if self.is_wiping_out == 0 {
                Some(0)
            } else {
                self.specific_parts.recent_contact[region].map(|value| u8::from(value != 0))
            };
            self.specific_parts.contact[region] = Some(0);
            let offset = self.specific_parts.offsets[region];
            let matrix = history
                .record()
                .transform_typed(self.specific_parts.parts[region].index())
                .expect("native specific part range");
            let columns = matrix.columns();
            let x = columns[0].zip(columns[3], |axis, position| {
                axis.mul_add(offset[0], position)
            });
            let y = columns[1].zip(x, |axis, position| axis.mul_add(offset[1], position));
            let position = columns[2].zip(y, |axis, position| axis.mul_add(offset[2], position));
            self.specific_parts.world_positions[region] = [
                position.xyz[0],
                position.xyz[1],
                position.xyz[2],
                position.w.unwrap_or(0.0),
            ];
            self.specific_parts.world_position_w_known[region] = position.w.is_some();
            if cvars.draw_body_part_collision()? != 0 {
                cvars.require_debug_overlay()?;
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "source-ordered actual collision update with distinct lazy services"
    )]
    #[expect(
        clippy::float_cmp,
        clippy::assign_op_pattern,
        reason = "source uses exact priority equality and scalar-left multiplication order"
    )]
    pub(crate) fn try_update_normal<
        A: SkeletonCollisionUpdateAttributeRead,
        C: SkeletonCollisionCvars,
        N: SkeletonCollisionNumeric,
        L: SkeletonCollisionLiveRead,
    >(
        &mut self,
        is_ragdoll: bool,
        info: &SkeletonCollisionUpdateInput,
        contacts: &[PhysicalContactInfo],
        history: &SkeletonPhysicsHistory,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
        live: &mut L,
    ) -> CollisionResult<(), A::Error, C::Error, N::Error, L::Error> {
        use SkeletonCollisionUpdateError as Error;
        let direction = math::try_normalize_safe_xyz(info.system_velocity, [0.0; 3], numeric)
            .map_err(Error::Numeric)?;
        let speed_squared = numeric
            .dot3(info.system_velocity, info.system_velocity)
            .map_err(Error::Numeric)?;
        self.collision_timer -= info.timestep;
        self.compliant_just_collided = 0;
        self.non_compliant_just_collided = 0;
        self.num_normals = 0;
        self.hit_vehicle = 0;
        self.hit_world = 0;
        let wipeout_time = self.time_in_wipeout + info.timestep;
        let speed_seed = numeric
            .reciprocal_sqrt_estimate(speed_squared)
            .map_err(Error::Numeric)?;
        self.time_in_wipeout = wipeout_time;
        self.is_wiping_out = u8::from(is_ragdoll);
        let speed = math::length_from_squared_and_estimate(speed_squared, speed_seed);
        let duration_pointer = attributes
            .lookup(Attribute::TimerDuration)
            .map_err(Error::Attribute)?;
        let duration = attributes
            .float(&duration_pointer)
            .map_err(Error::Attribute)?;
        self.clear_update_fields(info.timestep);
        if cvars.draw_body_part_collision().map_err(Error::Cvar)? != 0 {
            return Err(Error::DebugRegionReloadUnsupported);
        }
        self.update_specific_regions(history, cvars)
            .map_err(Error::Cvar)?;
        if std::env::var_os("SKATE2_TRACE_TOE_TARGETS").is_some() && !is_ragdoll {
            let positions = history.record().positions();
            eprintln!(
                "toe bodies: ground={:?} danger_zone={} part15={:?} part16={:?} part19={:?} part20={:?}",
                info.ground_position,
                info.danger_zone,
                positions[15],
                positions[16],
                positions[19],
                positions[20]
            );
        }
        if info.request_partial_ragdoll != 0 {
            self.collision_timer = duration;
        }
        let mut priority = 0.0;
        for contact in contacts {
            let part = SkeletonPartIndex::from_index(contact.part() as usize)
                .ok_or(Error::InvalidPart(contact.part()))?;
            let group = match contact.other_body() {
                Some(body) => live.other_body_group(body).map_err(Error::Live)?,
                None => 0,
            };
            if group == 4 && matches!(contact.part(), 15 | 16 | 19 | 20) {
                self.foot_hit_skateboard = 1;
            }
            if contact.self_colliding() {
                continue;
            }
            if contact.surface() & 0xf80 == 0x300 {
                self.unrideable_contact = 1;
            }
            let spy = contact.spy();
            let opposite = if contact.is_body_a() {
                spy.body_b()
            } else {
                spy.body_a()
            };
            if info.ignore_rigid_body == Some(opposite) {
                continue;
            }
            if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some()
                && !is_ragdoll
                && group != 4
                && matches!(contact.part(), 15 | 16 | 19 | 20)
            {
                let delta: [f32; 3] = std::array::from_fn(|axis| {
                    contact.position()[axis] - info.ground_position[axis]
                });
                let normal = contact.normal();
                let height = delta[0] * info.ground_normal[0]
                    + delta[1] * info.ground_normal[1]
                    + delta[2] * info.ground_normal[2];
                let align = (normal[0] * info.ground_normal[0]
                    + normal[1] * info.ground_normal[1]
                    + normal[2] * info.ground_normal[2])
                    .abs();
                let body = history.record().positions()[part.index()];
                let body_velocity = history.record().velocities()[part.index()];
                eprintln!(
                    "known air: foot contact part={} group={group} danger_zone={} ground_trick={} height={height} align={align} body={body:?} body_velocity={body_velocity:?} position={:?}",
                    contact.part(),
                    info.danger_zone,
                    info.ground_trick,
                    contact.position()
                );
            }
            if !is_ragdoll
                && (info.danger_zone == 0 || info.ground_trick != 0)
                && is_ground_plane(contact, info, attributes, cvars, numeric)?
            {
                if cvars.draw_collision_update().map_err(Error::Cvar)? != 0 {
                    cvars.require_debug_overlay().map_err(Error::Cvar)?;
                }
                continue;
            }
            let normal = *contact.normal();
            if !is_ragdoll && normal[1] > self.most_vertical_contact_normal.xyz()[1] {
                self.most_vertical_contact_normal = SkeletonCollisionVector::xyz_only(normal);
            }
            if !is_ragdoll
                && matches!(contact.part(), 15 | 19)
                && normal[1] > self.most_vertical_toe_contact_normal.xyz()[1]
            {
                self.most_vertical_toe_contact_normal = SkeletonCollisionVector::xyz_only(normal);
            }
            let mut specific = false;
            for region in 0..2 {
                if cvars.allow_any_contact().map_err(Error::Cvar)? != 0
                    || self.specific_parts.parts[region] == part
                {
                    let delta = std::array::from_fn(|axis| {
                        contact.position()[axis]
                            - self.specific_parts.world_position_typed(region).xyz[axis]
                    });
                    if numeric.dot3(delta, delta).map_err(Error::Numeric)?
                        < self.specific_parts.radius_squared[region]
                    {
                        specific = true;
                        self.specific_parts.contact[region] = Some(1);
                        self.specific_parts.recent_contact[region] = Some(1);
                        if cvars.draw_body_part_collision().map_err(Error::Cvar)? != 0 {
                            cvars.require_debug_overlay().map_err(Error::Cvar)?;
                        }
                    }
                }
            }
            if self.num_normals >= 20 {
                continue;
            }
            let index = part.index();
            self.time_since_collision[index] = 0.0;
            self.just_collided[index] = 1;
            if self.priority[index] > priority {
                priority = self.priority[index];
                self.num_normals = 0;
            }
            if self.priority[index] == priority {
                let normal_index = usize::try_from(self.num_normals)
                    .expect("update initializes a nonnegative normal count");
                self.normal_info[normal_index] = SkeletonCollisionNormalInfo {
                    normal: SkeletonCollisionVector::xyz_only(normal),
                    part,
                };
                self.num_normals += 1;
            }
            let part_mass = history.part_masses()[index];
            let inverse_a = if live.raw_state(spy.body_a()).map_err(Error::Live)? == 1 {
                f32::from_bits(0x3a83_126f)
            } else {
                live.cached_inverse_mass(spy.body_a())
                    .map_err(Error::Live)?
            };
            let inverse_b = if live.raw_state(spy.body_b()).map_err(Error::Live)? == 1 {
                f32::from_bits(0x3a83_126f)
            } else {
                live.cached_inverse_mass(spy.body_b())
                    .map_err(Error::Live)?
            };
            let mass_a = 1.0 / inverse_a;
            let mass_b = 1.0 / inverse_b;
            let other_mass = if contact.is_body_a() { mass_b } else { mass_a };
            let small_pointer = attributes
                .lookup(Attribute::SmallObjectMassThreshold)
                .map_err(Error::Attribute)?;
            let small = other_mass > f32::from_bits(0x38d1_b717)
                && other_mass < attributes.float(&small_pointer).map_err(Error::Attribute)?;
            let friction_scale =
                ((mass_b + mass_a) / (mass_b * mass_a)) * f32::from_bits(0x3991_a2b5);
            let friction = spy.friction_force().map(|value| value * friction_scale);
            let friction_speed = math::try_length_xyz(friction, numeric).map_err(Error::Numeric)?;
            let modeled = if info.offboard != 0 || info.offboard_air != 0 {
                [
                    info.centre_of_mass_velocity[0],
                    0.0,
                    info.centre_of_mass_velocity[2],
                ]
            } else {
                let velocity = history.record().velocities()[index];
                let along = numeric
                    .dot3([velocity[0], velocity[1], velocity[2]], direction)
                    .map_err(Error::Numeric)?;
                math::try_clamp_xyz(direction.map(|value| value * along), speed, numeric)
                    .map_err(Error::Clamp)?
            };
            let other_velocity = live.velocity(opposite).map_err(Error::Live)?;
            let relative = std::array::from_fn(|axis| other_velocity[axis] - modeled[axis]);
            let mut force = abs_bits(numeric.dot3(relative, normal).map_err(Error::Numeric)?);
            if small {
                force = cvars.small_object_force_scalar().map_err(Error::Cvar)? * force;
            }
            match group {
                6 => {
                    let route = if info.ai_skater != 0 {
                        Attribute::AiGroupSixForceMultiplier
                    } else {
                        Attribute::GroupSixForceMultiplier
                    };
                    let pointer = attributes.lookup(route).map_err(Error::Attribute)?;
                    force = attributes.float(&pointer).map_err(Error::Attribute)? * force;
                    self.skater_contact =
                        fsel(self.skater_contact - force, self.skater_contact, force);
                }
                10 => {
                    self.hit_vehicle = 1;
                    self.vehicle_contact =
                        fsel(self.vehicle_contact - force, self.vehicle_contact, force);
                }
                13 => {
                    self.pedestrian_contact = fsel(
                        self.pedestrian_contact - force,
                        self.pedestrian_contact,
                        force,
                    );
                }
                _ => {}
            }
            if group != 4 {
                self.hit_world = 1;
            }
            if index == 0 {
                self.is_proxy_in_collision = 1;
            } else {
                let bucket = COLLISION_GROUP[index];
                let bone = &mut self.bone_data[index];
                let previous = if specific {
                    bone.sub_part_contact_force
                } else {
                    bone.contact_force
                };
                let bone_wins = force > previous;
                let group_wins = force > self.contact_force[bucket];
                let slide = if bone_wins || group_wins {
                    math::cross_xyz(relative, normal)
                } else {
                    [0.0; 3]
                };
                if bone_wins {
                    if specific {
                        bone.sub_part_contact_force = force;
                        bone.sub_part_contact_normal = SkeletonCollisionVector::xyz_only(normal);
                        bone.sub_part_contact_surface_id = contact.surface();
                        bone.sub_part_slide_vector = SkeletonCollisionVector::xyz_only(slide);
                    } else {
                        bone.contact_force = force;
                        bone.contact_normal = SkeletonCollisionVector::xyz_only(normal);
                        bone.contact_position =
                            SkeletonCollisionVector::xyz_only(*contact.position());
                        bone.contact_surface_id = contact.surface();
                        bone.slide_vector = SkeletonCollisionVector::xyz_only(slide);
                    }
                    bone.hit_vehicle |= u8::from(group == 10);
                    bone.hit_pedestrian |= u8::from(group == 13);
                    bone.hit_skater |= u8::from(group == 6);
                    bone.hit_world |= u8::from(!matches!(group, 6 | 10 | 13));
                }
                if group_wins {
                    let squared = numeric.dot3(slide, slide).map_err(Error::Numeric)?;
                    let friction_force = friction_speed * part_mass;
                    self.contact_force[bucket] = force;
                    let slide_estimate = numeric
                        .reciprocal_sqrt_estimate(squared)
                        .map_err(Error::Numeric)?;
                    self.slide_speed[bucket] =
                        math::length_from_squared_and_estimate(squared, slide_estimate);
                    self.contact_friction_force[bucket] = friction_force;
                    self.contact_normal[bucket] = SkeletonCollisionVector::xyz_only(normal);
                    self.contact_bone[bucket] = Some(part);
                    self.audio_surface_type[bucket] = contact.surface() & 0x7f;
                }
                self.collision_timer = duration;
            }
            if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                let delta: [f32; 3] = std::array::from_fn(|axis| {
                    contact.position()[axis] - info.ground_position[axis]
                });
                let height = delta[0] * info.ground_normal[0]
                    + delta[1] * info.ground_normal[1]
                    + delta[2] * info.ground_normal[2];
                eprintln!(
                    "known air: skeleton contact part={index} group={group} compliant={} small={small} danger_zone={} ground_trick={} offboard={} height={height} normal={:?} position={:?} ground={:?} ground_normal={:?}",
                    self.is_compliant[index],
                    info.danger_zone,
                    info.ground_trick,
                    info.offboard,
                    contact.normal(),
                    contact.position(),
                    info.ground_position,
                    info.ground_normal
                );
            }
            if self.is_compliant[index] != 0 && !small {
                self.compliant_just_collided = 1;
            } else {
                self.non_compliant_just_collided = 1;
            }
            self.is_hook_to_hips_valid =
                u8::from(self.compliant_just_collided != 0 || info.offboard != 0);
        }
        if is_ragdoll {
            self.opposing_contacts_on_plane =
                u8::from(self.check_impaled_on_fence(cvars, numeric)?);
        } else {
            self.opposing_contacts = u8::from(self.check_conflicting_contacts(numeric, live)?);
        }
        self.collision_priority = priority;
        self.drive_strength = if duration == 0.0 {
            1.0
        } else {
            let ratio = self.collision_timer / duration;
            let lower = fsel(-ratio, 0.0, ratio);
            let upper = fsel(1.0 - lower, lower, 1.0);
            1.0 - upper
        };
        if cvars.debug_always_collide().map_err(Error::Cvar)? != 0 {
            self.drive_strength = 0.0;
            self.compliant_just_collided = 1;
        }
        self.compliant_active = u8::from(self.drive_strength < 1.0);
        Ok(())
    }

    /// `CheckForBeingImpaledOnAFence82b3ab40`. Reads actual per-bone response
    /// records, not contact groups or live body transforms.
    fn check_impaled_on_fence<A, C: SkeletonCollisionCvars, N: SkeletonCollisionNumeric, L>(
        &self,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<bool, SkeletonCollisionUpdateError<A, C::Error, N::Error, L>> {
        use SkeletonCollisionUpdateError as Error;
        let distance_target = cvars
            .capture_fence_threshold(FenceThreshold::Distance)
            .map_err(Error::Cvar)?;
        let dot_target = cvars
            .capture_fence_threshold(FenceThreshold::OpposingDot)
            .map_err(Error::Cvar)?;
        let distance = cvars
            .read_fence_threshold(&distance_target)
            .map_err(Error::Cvar)?;
        let dot_threshold = cvars
            .read_fence_threshold(&dot_target)
            .map_err(Error::Cvar)?;
        for i in 1..23 {
            let first = &self.bone_data[i];
            // Raw BO4/BI25 skips GT-clear, including unordered NaN.
            if first.contact_force <= 0.0 || first.contact_force.is_nan() {
                continue;
            }
            for second in &self.bone_data[i + 1..24] {
                if second.contact_force <= 0.0 || second.contact_force.is_nan() {
                    continue;
                }
                let normal = *first.contact_normal.xyz();
                if numeric
                    .dot3(normal, *second.contact_normal.xyz())
                    .map_err(Error::Numeric)?
                    < dot_threshold
                {
                    let delta = std::array::from_fn(|axis| {
                        first.contact_position.xyz()[axis] - second.contact_position.xyz()[axis]
                    });
                    if abs_bits(numeric.dot3(normal, delta).map_err(Error::Numeric)?) < distance {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn check_conflicting_contacts<
        A,
        C,
        N: SkeletonCollisionNumeric,
        L: SkeletonCollisionLiveRead,
    >(
        &self,
        numeric: &mut N,
        live: &mut L,
    ) -> Result<bool, SkeletonCollisionUpdateError<A, C, N::Error, L::Error>> {
        use SkeletonCollisionUpdateError as Error;
        for i in 0..8 {
            if self.contact_force[i] <= 0.0 || self.contact_force[i].is_nan() {
                continue;
            }
            let Some(part_i) = self.contact_bone[i] else {
                continue;
            };
            if self.is_compliant[part_i.index()] == 0 {
                continue;
            }
            for j in 0..8 {
                if i == j || self.contact_force[j] <= 0.0 || self.contact_force[j].is_nan() {
                    continue;
                }
                let Some(part_j) = self.contact_bone[j] else {
                    continue;
                };
                if self.is_compliant[part_j.index()] == 0 {
                    continue;
                }
                let normal = *self.contact_normal[i].xyz();
                if numeric
                    .dot3(normal, *self.contact_normal[j].xyz())
                    .map_err(Error::Numeric)?
                    < f32::from_bits(0xbf33_3333)
                {
                    let transform_i = live.part_transform(part_i.index()).map_err(Error::Live)?;
                    let transform_j = live.part_transform(part_j.index()).map_err(Error::Live)?;
                    let delta =
                        std::array::from_fn(|axis| transform_j[12 + axis] - transform_i[12 + axis]);
                    if numeric.dot3(delta, normal).map_err(Error::Numeric)? < 0.0 {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

fn is_ground_plane<
    A: SkeletonCollisionUpdateAttributeRead,
    C: SkeletonCollisionCvars,
    N: SkeletonCollisionNumeric,
    L,
>(
    contact: &PhysicalContactInfo,
    info: &SkeletonCollisionUpdateInput,
    attributes: &mut A,
    cvars: &mut C,
    numeric: &mut N,
) -> CollisionResult<bool, A::Error, C::Error, N::Error, L> {
    use SkeletonCollisionUpdateError as Error;
    let maximum = if info.offboard != 0 {
        cvars
            .maximum_ground_distance_offboard()
            .map_err(Error::Cvar)?
    } else {
        let pointer = attributes
            .lookup(Attribute::GroundMaximumDistance)
            .map_err(Error::Attribute)?;
        attributes.float(&pointer).map_err(Error::Attribute)?
    };
    let alignment_pointer = attributes
        .lookup(Attribute::GroundMinimumAlignment)
        .map_err(Error::Attribute)?;
    let delta = std::array::from_fn(|axis| contact.position()[axis] - info.ground_position[axis]);
    let alignment = abs_bits(
        numeric
            .dot3(*contact.normal(), info.ground_normal)
            .map_err(Error::Numeric)?,
    );
    let distance = numeric
        .dot3(info.ground_normal, delta)
        .map_err(Error::Numeric)?;
    let ground = distance < maximum
        && (info.offboard != 0
            || alignment
                > attributes
                    .float(&alignment_pointer)
                    .map_err(Error::Attribute)?);
    if cvars.show_ground_plane().map_err(Error::Cvar)? != 0 {
        cvars.require_debug_overlay().map_err(Error::Cvar)?;
    }
    Ok(ground)
}
