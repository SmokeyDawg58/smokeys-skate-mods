//! Retained LW pedestrian post-adjust, head-target, and spatial-adjust owners.
//! These constructor states do not substitute for their later collision/pose updates.
use crate::{actor_face_playback::AnimationPoseRef, attribute_runtime::AttributeRuntimeInstance};
use skate2_formats::andale::AndaleHierarchy;
use std::sync::Arc;

#[path = "pedestrian_jump_trajectory.rs"]
pub(crate) mod jump_trajectory;
#[path = "pedestrian_head_tracking_update.rs"]
pub(crate) mod head_update;
#[path = "pedestrian_post_adjust_update.rs"]
mod post_update;
#[path = "../../animation/graphs/pedestrian_post_adjust_codec.rs"]
pub(crate) mod post_adjust_codec;

pub(crate) const POST_ADJUST_REFERENCE: u64 = 0x6395_4595_5147_d889;
pub(crate) const HEAD_TRACKING_REFERENCE: u64 = 0x1422_7727_ead7_f757;
const BONE_ADJUSTMENTS: u64 = 0x43a2_b449_3780_45fb;
const BONE_ADJUSTMENT_TYPE: u64 = 0x772b_033a_57a5_7dbd;
const BONE_NAMES: [&str; 6] = ["Head", "Neck1", "Neck", "Spine3", "Spine2", "Spine1"];

#[derive(Clone, Copy)]
pub(crate) struct BoneAdjustment {
    pub(crate) bone: u8,
    pub(crate) axis_angle: [u8; 6],
}

pub(crate) struct PostAdjust {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) bind_pose: AnimationPoseRef,
    pub(crate) hierarchy: Arc<AndaleHierarchy>,
    pub(crate) adjustments: [Option<BoneAdjustment>; 6],
    pub(crate) bone_yaws: [f32; 6],
    pub(crate) max_bone_yaws: [Option<f32>; 6],
    pub(crate) head_yaw: f32,
    pub(crate) max_head_yaw: Option<f32>,
    pub(crate) head_pitch: f32,
    pub(crate) max_head_pitch: Option<f32>,
}
impl PostAdjust {
    /// Retain allocation before fallible ctor82e8aac8 reads. Unwritten members
    /// are absent, not invented default bone0 transforms or angle constraints.
    pub(crate) fn before_constructor(
        attributes: AttributeRuntimeInstance,
        bind_pose: AnimationPoseRef,
        hierarchy: Arc<AndaleHierarchy>,
    ) -> Self {
        Self { attributes, bind_pose, hierarchy, adjustments: [None; 6],
            bone_yaws: [0.; 6], max_bone_yaws: [None; 6], head_yaw: 0.,
            max_head_yaw: None, head_pitch: 0., max_head_pitch: None }
    }

    /// cLivingWorldPostAdjust82e8aac8. The second lookup uses BoneIndex,
    /// not the loop index; preserve this distinction for reordered authored arrays.
    pub(crate) fn construct(&mut self) -> Result<(), String> {
        let data = self.attributes.try_cached_layout_field(12, 4)
            .map_err(|e| e.to_string())?.ok_or("post-adjust layout absent")?;
        let degrees = f32::from_be_bytes(data.bytes().try_into()
            .map_err(|_| "post-adjust pitch field size")?);
        self.max_head_pitch = Some(degrees * 0.017453292_f32);
        let count = self.attributes.try_length(BONE_ADJUSTMENTS).map_err(|e| e.to_string())?;
        for index in 0..count {
            let (bone, _) = self.bone_mapping(index)?;
            let name = BONE_NAMES.get(bone as usize).ok_or("post-adjust bone enum outside six slots")?;
            let packed = skate2_formats::andale::PackedFastString::<5>::encode(name)
                .map_err(|e| format!("post-adjust bone name: {e:?}"))?;
            let bone_id = match self.hierarchy.packed_bone_names.iter().position(|value| *value == packed) {
                Some(index) => u8::try_from(index).map_err(|_| "post-adjust hierarchy bone exceeds byte")?,
                None => 255, // GetBone returns -1; native narrows to the replay byte.
            };
            // QuatCompressAxisAngle82e7c7c8(identity): pitch/yaw quantize
            // floor(0.5*65535+0.5)=32768; half-angle quantizes to zero.
            self.adjustments[bone as usize] = Some(BoneAdjustment {
                bone: bone_id, axis_angle: [0x80, 0, 0x80, 0, 0, 0],
            });
            let (_, max_degrees) = self.bone_mapping(bone)?;
            self.max_bone_yaws[bone as usize] = Some(max_degrees * 0.017453292_f32);
        }
        let mut sum = 0.0;
        for yaw in self.max_bone_yaws {
            sum += yaw.ok_or("post-adjust constructor left a maximum bone yaw unwritten")?;
        }
        self.max_head_yaw = Some(sum);
        Ok(())
    }
    fn bone_mapping(&self, index: u32) -> Result<(u32, f32), String> {
        let data = self.attributes.try_plain_array_data(BONE_ADJUSTMENTS, index,
            BONE_ADJUSTMENT_TYPE, 8).map_err(|e| e.to_string())?;
        let Some(data) = data else { return Ok((0, 0.)); }; // DefaultDataArea(8)
        let bytes = data.bytes();
        Ok((u32::from_be_bytes(bytes[0..4].try_into().map_err(|_| "bone index width")?),
            f32::from_be_bytes(bytes[4..8].try_into().map_err(|_| "bone angle width")?)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum HeadTrackingMode {
    None = 0, TargetableCenterOfMass = 1, TargetableVisibilityPoint = 2,
    StraightAhead = 3, WanderDynamic = 4, BehaviourSource = 5, Flee = 6,
    SkaterCenterOfMass = 7,
}
#[derive(Clone, Copy)]
pub(crate) struct HeadTarget {
    pub(crate) mode: HeadTrackingMode,
    pub(crate) targetable: Option<(u32, u32)>,
    pub(crate) behaviour_source: Option<u32>,
    pub(crate) flee_look_back_timer: Option<f32>,
}
impl HeadTarget {
    fn straight_ahead() -> Self {
        Self { mode: HeadTrackingMode::StraightAhead, targetable: None,
            behaviour_source: None, flee_look_back_timer: None }
    }
}
pub(crate) struct HeadTracking {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) resolved_target: [f32; 4],
    pub(crate) base: HeadTarget,
    pub(crate) stack: [HeadTarget; 3],
    stack_level: i32,
    pub(crate) point_of_interest: [f32; 4],
    pub(crate) flee_look_back_time: Option<f32>,
    pub(crate) flee_looking_back: Option<bool>,
}
impl HeadTracking {
    /// cLivingWorldHeadTracking82e74250; timers have no constructor writes.
    pub(crate) fn new(attributes: AttributeRuntimeInstance) -> Self {
        let mut base = HeadTarget::straight_ahead();
        base.mode = HeadTrackingMode::WanderDynamic;
        Self { attributes, resolved_target: [0.; 4], base,
            stack: [HeadTarget::straight_ahead(); 3], stack_level: -1,
            point_of_interest: [0.; 4], flee_look_back_time: None, flee_looking_back: None }
    }
    pub(crate) fn current(&self) -> &HeadTarget {
        if self.stack_level < 0 { &self.base } else { &self.stack[self.stack_level as usize] }
    }
    /// PushHeadTrackingMode82e6b4d8 ignores pushes at capacity; it does not
    /// overwrite the slot's flee timer. Targetable modes require their own API.
    pub(crate) fn push(&mut self, mode: HeadTrackingMode) -> Result<(), String> {
        if matches!(mode, HeadTrackingMode::TargetableCenterOfMass | HeadTrackingMode::TargetableVisibilityPoint) {
            return Err("PushHeadTrackingMode native assertion: targetable mode".into());
        }
        if self.stack_level < 2 {
            self.stack_level += 1;
            let target = &mut self.stack[self.stack_level as usize];
            target.mode = mode;
            target.targetable = None;
            target.behaviour_source = None;
        }
        Ok(())
    }
    /// PopHeadTrackingMode82e6b818 preserves dormant stack contents.
    pub(crate) fn pop(&mut self) { self.stack_level = (self.stack_level - 1).max(-1); }
    /// OnRemoveBehaviourSource82e743c0 scans dormant slots too. Only a match
    /// in slot2 decrements StackLevel, independently of the current top slot.
    pub(crate) fn on_remove_behaviour_source(&mut self,source:Option<u32>){
        for (index,target) in self.stack.iter_mut().enumerate(){
            if target.behaviour_source==source{
                target.mode=HeadTrackingMode::StraightAhead;target.targetable=None;target.behaviour_source=None;
                if index==2{self.stack_level=(self.stack_level-1).max(-1);}
            }
        }
    }
    /// PushBehaviourSourceTrackingMode82e6b720 preserves the slot's timer.
    pub(crate) fn push_behaviour_source(&mut self, source:Option<u32>)->Result<(),String>{
        let Some(source)=source else{return self.push(HeadTrackingMode::StraightAhead);};
        if self.stack_level>=2{return Err("native behaviour-source head tracking stack assertion".into());}
        self.stack_level+=1;
        let target=&mut self.stack[self.stack_level as usize];
        target.mode=HeadTrackingMode::BehaviourSource;
        target.targetable=None;
        target.behaviour_source=Some(source);
        Ok(())
    }
    ///82e6b5a0 validates both targetable modes but stores CenterOfMass for
    /// either. Failed entity resolution pushes StraightAhead; flee time survives.
    pub(crate) fn push_targetable(&mut self, mode: HeadTrackingMode,
        id: crate::living_world_chase::TargetableId,
        exists: impl FnOnce() -> Result<bool, String>,
    ) -> Result<(), String> {
        if !matches!(mode, HeadTrackingMode::TargetableCenterOfMass | HeadTrackingMode::TargetableVisibilityPoint) {
            return Err("native targetable head tracking mode assertion".into());
        }
        if id.is_valid() {
            if self.stack_level >= 2 { return Err("native targetable head tracking stack capacity assertion".into()); }
            if exists()? {
                self.stack_level += 1;
                let target = &mut self.stack[self.stack_level as usize];
                target.mode = HeadTrackingMode::TargetableCenterOfMass;
                target.targetable = Some((id.kind as u32, id.id as u32));
                target.behaviour_source = None;
                return Ok(());
            }
        }
        self.push(HeadTrackingMode::StraightAhead)
    }
    /// NotifyEntityCulled82e6b840 visits only live stack entries; base and
    /// dormant slots survive. Targetable tuple stores (entity kind, ID).
    pub(crate) fn notify_entity_culled(&mut self, entity_id: u32) {
        for index in 0..=self.stack_level {
            let target = &mut self.stack[index as usize];
            if target.targetable == Some((1, entity_id)) {
                target.targetable = None;
                target.mode = HeadTrackingMode::StraightAhead;
                target.behaviour_source = None;
            }
        }
    }

    /// SetStraightAheadTarget82e744b8: up*SimLODPose head height +
    /// translation, then forward. vmaddfp's encoded A/C/B operands must not
    /// be confused with the decompiler's printed register order.
    pub(crate) fn set_straight_ahead(&mut self, orientation: [[f32; 4]; 4], head_height: f32) {
        self.resolved_target = std::array::from_fn(|i|
            orientation[1][i].mul_add(head_height, orientation[3][i]) + orientation[2][i]);
    }
}

/// Jump fields not written by ctor are absent until JumpNow82e7d5f0 produces
/// them; this avoids claiming zero trajectory state from reused MemStore bytes.
pub(crate) struct JumpInfo {
    pub(crate) trajectory: Option<JumpTrajectory>,
    pub(crate) elapsed_time: f32,
    pub(crate) up_time: f32,
    pub(crate) down_time: f32,
    pub(crate) is_jumping: bool,
    pub(crate) downward_motion: Option<bool>,
}
pub(crate) struct JumpTrajectory {
    pub(crate) initial_velocity: [f32; 4],
    pub(crate) downward_velocity: [f32; 4],
    pub(crate) landing_position: [f32; 4],
    pub(crate) ledge_position: [f32; 4],
    pub(crate) initial_height: f32,
    pub(crate) horizontal_speed: f32,
}
pub(crate) struct SpatialAdjust {
    pub(crate) jump: JumpInfo,
    pub(crate) ground_normal: [f32; 4],
    pub(crate) ground_position: [f32; 4],
    pub(crate) up_vector: [f32; 4],
    pub(crate) locked_position: [f32; 4],
    pub(crate) sliding_position: [f32; 4],
    pub(crate) last_known_good_ground_position: [f32; 4],
    pub(crate) ground_surface_id: u32,
    pub(crate) up_shift: f32,
    pub(crate) sliding_speed: f32,
    pub(crate) did_slide: bool,
    pub(crate) has_valid_ground_position: bool,
    pub(crate) pulling_back: bool,
    pub(crate) sliding_to_position: bool,
    pub(crate) position_locked: bool,
}
/// Values read from the retained runtime cvars by the spatial update caller.
pub(crate) struct SpatialAdjustTuning {
    pub(crate) pullback_speed: f32,
    pub(crate) y_shift_time: f32,
    pub(crate) sliding_clamp: f32,
    pub(crate) jump_use_landing: bool,
    pub(crate) jump_use_ledge: bool,
    pub(crate) jump_arrival_radius: f32,
}
impl SpatialAdjust {
    /// cLivingWorldSpatialAdjust::Update82e745a0. The collision owner reads
    /// actual skater part0; its original collision gate is evaluated here
    /// lazily, after lock/jump/pullback/scripted-slide.
    pub(crate) fn update<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, dt: f32, orientation: &mut [[f32; 4]; 4],
        collision: &crate::pedestrian_collision::PedestrianCollision,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
        apply_collision_ground_offset: bool, apply_collision_sliding: bool,
        rig: Option<&crate::pedestrian_navigation::rig::NavRig>,
        tuning: &SpatialAdjustTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        use crate::reckoning_numeric::{try_normalize_safe, try_normalize_unsafe};
        self.did_slide = false;
        if !self.has_valid_ground_position && !self.jump.is_jumping { return Ok(()); }
        let position = orientation[3];
        let updated = if self.position_locked {
            validate_vector(self.locked_position, "locked position")?;
            self.locked_position
        } else if self.jump.is_jumping {
            self.update_jump(dt, position, tuning, numeric)?
        } else if self.pulling_back {
            self.pullback_position(dt, position, tuning.pullback_speed, numeric)?
        } else {
            let ground_up = if apply_collision_ground_offset { self.ground_normal } else { [0., 1., 0., 0.] };
            let blended: [f32; 4] = std::array::from_fn(|i|
                (ground_up[i] - orientation[1][i]).mul_add(0.1, orientation[1][i]));
            let up = try_normalize_unsafe(blended, numeric)
                .map_err(|e| format!("pedestrian ground normal: {e:?}"))?;
            validate_vector(up, "normalized up")?;
            self.up_vector = up;
            // Native multiply order is reciprocal time * vertical difference,
            // then dt; no dt clamp or direct ground snapping.
            let amount = ((1.0 / tuning.y_shift_time) * (self.ground_position[1] - position[1])) * dt;
            let mut adjusted: [f32; 4] = std::array::from_fn(|i| amount.mul_add(up[i], position[i]));
            validate_vector(adjusted, "ground-adjusted position")?;
            if self.sliding_to_position {
                let delta = [self.sliding_position[0] - position[0], 0., self.sliding_position[2] - position[2], 0.];
                let direction = try_normalize_safe(delta, [0.; 4], numeric)
                    .map_err(|e| format!("pedestrian scripted slide: {e:?}"))?;
                let amount = self.sliding_speed * dt;
                // This branch intentionally replaces the preceding ground shift.
                adjusted = std::array::from_fn(|i| direction[i].mul_add(amount, position[i]));
            } else if apply_collision_sliding
                && (collision.required_to_begin_colliding || collision.colliding)
                && !collision.world_part_colliding && collision.skater_parts_colliding[0] {
                    let body = collision.skater_part_transform(0, world)?;
                    let body: [[f32; 4]; 4] = std::array::from_fn(|axis|
                        std::array::from_fn(|lane| body[axis * 4 + lane]));
                    for axis in body { validate_vector(axis, "collision body transform")?; }
                    let high = [tuning.sliding_clamp, 0., tuning.sliding_clamp, 0.];
                    let low = [-tuning.sliding_clamp, 0., -tuning.sliding_clamp, 0.];
                    // VMX min/max exceptional behavior follows the shared host
                    // numeric boundary; ordinary finite lane order is max/min.
                    let mut shift: [f32; 4] = std::array::from_fn(|i|
                        (body[3][i] - adjusted[i]).max(low[i]).min(high[i]));
                    if let Some(rig) = rig {
                        let squared = numeric.dot3(shift, shift)
                            .map_err(|e| format!("pedestrian slide length: {e:?}"))?;
                        if squared > 0. {
                            let (distance, _, _) = rig.distances.heading_distance(
                                rig.position, &rig.directions, shift, numeric)?;
                            let squared = numeric.dot3(shift, shift)
                                .map_err(|e| format!("pedestrian slide clearance: {e:?}"))?;
                            if squared - distance * distance > 0. { shift = [0.; 4]; }
                        }
                    }
                    self.did_slide = shift[..3].iter().any(|v| v.abs() > f32::from_bits(0x3400_0000));
                    adjusted = std::array::from_fn(|i| adjusted[i] + shift[i]);
            }
            validate_vector(adjusted, "sliding position")?;
            adjusted
        };
        // Native NavRig::IsValid return is discarded here; it does not gate
        // the write. Orientation axes remain untouched by this function.
        orientation[3] = updated;
        for axis in *orientation { validate_vector(axis, "updated orientation")?; }
        Ok(())
    }

    fn pullback_position<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, dt: f32, position: [f32; 4], speed: f32, numeric: &mut N,
    ) -> Result<[f32; 4], String>
    where N::Error: std::fmt::Debug,
    {
        let delta: [f32; 4] = std::array::from_fn(|i| self.last_known_good_ground_position[i] - position[i]);
        let direction = crate::reckoning_numeric::try_normalize_safe(delta, [0.; 4], numeric)
            .map_err(|e| format!("pedestrian pullback normalize: {e:?}"))?;
        Ok(std::array::from_fn(|i| (direction[i] * speed).mul_add(dt, position[i])))
    }

    fn update_jump<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, dt: f32, position: [f32; 4], tuning: &SpatialAdjustTuning,
        numeric: &mut N,
    ) -> Result<[f32; 4], String>
    where N::Error: std::fmt::Debug,
    {
        self.jump.elapsed_time = dt + self.jump.elapsed_time;
        let downward = self.jump.downward_motion.ok_or("pedestrian jump phase was never produced")?;
        let trajectory = self.jump.trajectory.as_ref().ok_or("pedestrian jump trajectory was never produced")?;
        let velocity = if downward { trajectory.downward_velocity } else { trajectory.initial_velocity };
        let vertical_speed = velocity[1] - self.jump.elapsed_time * 9.8000002_f32;
        let vertical = [0. * vertical_speed * dt, 1. * vertical_speed * dt, 0. * vertical_speed * dt, 0. * vertical_speed * dt];
        let horizontal = [velocity[0], 0., velocity[2], 0.];
        let updated = if self.pulling_back {
            self.pullback_position(dt, position, tuning.pullback_speed, numeric)?
        } else {
            std::array::from_fn(|i| position[i] + horizontal[i].mul_add(dt, vertical[i]))
        };
        let near = |point: [f32; 4], numeric: &mut N| -> Result<bool, String> {
            let delta = std::array::from_fn(|i| updated[i] - point[i]);
            let squared = numeric.dot3(delta, delta).map_err(|e| format!("pedestrian jump arrival: {e:?}"))?;
            Ok(squared - tuning.jump_arrival_radius * tuning.jump_arrival_radius < 0.)
        };
        if downward {
            if self.jump.elapsed_time >= self.jump.down_time
                || (tuning.jump_use_landing && near(trajectory.landing_position, numeric)?) {
                self.jump.is_jumping = false;
            }
        } else if self.jump.elapsed_time >= self.jump.up_time
            || (tuning.jump_use_ledge && near(trajectory.ledge_position, numeric)?)
            || (tuning.jump_use_landing && near(trajectory.landing_position, numeric)?) {
            self.jump.elapsed_time = 0.;
            self.jump.downward_motion = Some(true);
        }
        Ok(updated)
    }
    /// Pedestrian::ForcePositionToGroundPlane82e795e0. Submitted positioning
    /// misses still enter here with their endpoint/up normal and null owner.
    pub(crate) fn force_position_to_ground_plane(
        &mut self, position: [f32; 4], mut normal: [f32; 4], surface_id: u32,
        is_on_ground: bool, mesh: &crate::living_world_mesh_store::MeshStore,
        y_tolerance: f32,
    ) -> Result<(), String> {
        validate_vector(position, "ForcePositionToGroundPlane position")?;
        validate_vector(normal, "ForcePositionToGroundPlane normal")?;
        if position[..3].iter().any(|v| v.abs() > 1_000_000.) { return Ok(()); }
        if normal[1] < 0. { normal = normal.map(|v| v * -1.); }
        let water = surface_id & 0xf80 == 0x300;
        let usable = if is_on_ground && !water {
            mesh.try_get_cell_with_tolerance(
                crate::skeleton_root_transforms::SkeletonRootVector::known(position), y_tolerance)
                .map_err(|e| format!("pedestrian ground mesh: {e:?}"))?
                .score_with_threshold(0.) < 800.
        } else { false };
        self.ground_normal = normal;
        self.ground_position = position;
        self.has_valid_ground_position = true;
        if is_on_ground && !water && usable { self.last_known_good_ground_position = position; }
        self.ground_surface_id = surface_id;
        if water { self.pulling_back = true; }
        else if is_on_ground && usable { self.pulling_back = false; }
        Ok(())
    }
    /// cLivingWorldSpatialAdjust82e6b8b8. No fabricated collision result.
    pub(crate) fn new() -> Self {
        Self { jump: JumpInfo { trajectory: None, elapsed_time: 0., up_time: 0., down_time: 0.,
                is_jumping: false, downward_motion: None },
            ground_normal: [0., 1., 0., 0.], ground_position: [0.; 4], up_vector: [0., 1., 0., 0.],
            locked_position: [0.; 4], sliding_position: [0.; 4], last_known_good_ground_position: [0.; 4],
            ground_surface_id: 0, up_shift: 0., sliding_speed: 0., did_slide: false,
            has_valid_ground_position: false, pulling_back: false, sliding_to_position: false, position_locked: false }
    }
}

/// Native IsValid vector checks compare XYZ to themselves; W is ignored and
/// infinity is not silently rejected where the source only tests for NaN.
fn validate_vector(vector: [f32; 4], field: &str) -> Result<(), String> {
    if vector[..3].iter().any(|v| v.is_nan()) {
        Err(format!("pedestrian spatial native vector assertion: {field}"))
    } else { Ok(()) }
}
