//! `PhysOutConditioner_Audio::Update` 0x827c6848, the fourth `UpdateAll` 0x82e29f88 child: the
//! `PhysOut_Audio` values `AudioPresentation::FillSkaterData` and pedestrian perception read,
//! and the skeleton's torso angular velocity delta.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
use super::{
    AllocationSite, Conditioners, PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut,
    SkeletonRootVector,
    simple::Audio,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_grind_manager::view::grind_type_raw,
    physical_player_phys_out::{PhysOutMotionVector, PhysOutSystemVector},
    physics_state_ids::{PHYSICS_CATEGORY_GRIND, PHYSICS_STATE_NONSPECIFIC},
    reckoning_numeric::{ReckoningNumeric, try_length},
};
use std::sync::Arc;

/// `Sk8::kMinAirFrames` (`Audio.JumpTuning`, 0..=60), `kSmallJumpDV` and `kLargeJumpDV`
/// (0..=2.65, 0..=10), and `kUpDownTolerance` (`Audio.PCFoley`, 0..=5): registered defaults.
const MIN_AIR_FRAMES: i32 = 5;
const SMALL_JUMP_DV: f32 = 0.0;
const LARGE_JUMP_DV: f32 = f32::from_bits(0x4029_999a);
const UP_DOWN_TOLERANCE: f32 = f32::from_bits(0x3d8f_5c29);

const AUD_GENERAL: u64 = 0xc183_1bdb_6cb1_b1ea;
const AUD_CONTACTS: u64 = 0xc269_49fc_b638_a2ca;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
/// `GlobalAttributeCollection` +0x18, +0x58, +0x70 and +0x78.
const CLOTHING: u64 = 0x3982_76a6_3764_95ee;
const WHEEL_SLIDE: u64 = 0xba98_37a6_cf4c_26ed;
const COM_VELOCITY: u64 = 0x1abd_2984_d724_8589;

/// `UpdateWheelSlide`'s divisor and threshold, `UpdateWheelLanding`'s impact divisor, the
/// hippy jump strength thresholds and the off-board landing thresholds.
const WHEEL_SLIDE_RANGE: u64 = 0x56d9_31d2_0288_1c2d;
const WHEEL_SLIDE_MINIMUM: u64 = 0xe6c3_ffe8_aa70_f944;
const WHEEL_IMPACT_RANGE: u64 = 0x3d39_9fe0_4952_b425;
const HIPPY_JUMP_STRENGTH: u64 = 0x4687_52b0_bee6_5cdb;
const OFF_BOARD_LANDING: u64 = 0x7385_078d_d3c0_63ba;

/// The `SurfaceID` category (bits 7..12) of stairs.
const STAIRS_CATEGORY: u16 = 8;

/// The `GlobalAttributeCollection` instances the conditioner reads.
pub(crate) struct AudioAttributes {
    contacts: AttributeRuntimeInstance,
    clothing: AttributeRuntimeInstance,
    wheel_slide: AttributeRuntimeInstance,
    com_velocity: AttributeRuntimeInstance,
}
impl AudioAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            contacts: vault.instance(AUD_CONTACTS, DEFAULT),
            clothing: vault.instance(AUD_GENERAL, CLOTHING),
            wheel_slide: vault.instance(AUD_GENERAL, WHEEL_SLIDE),
            com_velocity: vault.instance(AUD_GENERAL, COM_VELOCITY),
        }
    }
}

/// `EA::Reflection::Float`.
const FLOAT_TYPE: u64 = 0xe222_28fb_b3c2_09d8;

/// `GetAttributePointer` of a float or a float array element, or `DefaultDataArea` (0.0)
/// without the attribute.
fn scalar(instance: &AttributeRuntimeInstance, key: u64, index: u32) -> Result<f32, AttributeRuntimeError> {
    let word = match instance.try_scalar_word(key, index) {
        Err(AttributeRuntimeError::UnsupportedArray { .. }) => instance
            .try_plain_array_data(key, index, FLOAT_TYPE, 4)?
            .map(|data| u32::from_be_bytes(data.bytes()[..4].try_into().expect("four-byte float"))),
        word => word?,
    };
    Ok(f32::from_bits(word.unwrap_or(0)))
}

#[derive(Debug)]
pub(crate) enum AudioConditionerError<N> {
    ForeignOutput,
    Missing(AllocationSite),
    MissingAttributes,
    MissingOutput(PhysOutComponent),
    Access(PhysOutAccessError),
    Attribute(AttributeRuntimeError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for AudioConditionerError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("audio conditioner belongs to another PhysOut"),
            Self::Missing(site) => write!(f, "audio conditioner missing {site:?}"),
            Self::MissingAttributes => f.write_str("audio conditioner attributes not bound"),
            Self::MissingOutput(component) => write!(f, "audio conditioner missing output {component:?}"),
            Self::Access(e) => write!(f, "audio conditioner output access: {e:?}"),
            Self::Attribute(e) => write!(f, "audio conditioner attribute: {e:?}"),
            Self::Numeric(e) => write!(f, "audio conditioner numeric collaborator: {e:?}"),
        }
    }
}
type E<N> = AudioConditionerError<N>;

fn component<T, N>(value: &Option<Box<T>>, component: PhysOutComponent) -> Result<&T, E<N>> {
    value.as_deref().ok_or(E::MissingOutput(component))
}

fn component_mut<T, N>(value: &mut Option<Box<T>>, component: PhysOutComponent) -> Result<&mut T, E<N>> {
    value.as_deref_mut().ok_or(E::MissingOutput(component))
}

impl Conditioners {
    /// `UpdateAll` slot 3, after the empty MissionSystem slot.
    pub(crate) fn try_update_audio<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), AudioConditionerError<N::Error>> {
        if !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let attributes = self.audio_attributes.as_ref().ok_or(E::MissingAttributes)?;
        let audio = self
            .audio
            .published_value_mut()
            .ok_or(E::Missing(AllocationSite::Audio))?;
        if !Arc::ptr_eq(&audio.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        audio.payload.update(output, attributes, numeric)
    }
}

impl Audio {
    /// `Update` 0x827c6848.
    fn update<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        attributes: &AudioAttributes,
        numeric: &mut N,
    ) -> Result<(), E<N::Error>> {
        output.check_current_guard().map_err(E::Access)?;
        self.update_wheel_slide(output, attributes, numeric)?;
        self.update_wheel_landing(output, attributes, numeric)?;
        self.update_jump(output, numeric)?;
        self.update_angular_velocity(output)?;
        self.update_grinds(output)?;
        self.update_averaged_collision_info(output)?;
        self.update_averaged_board_collision_info(output)?;
        self.update_wipeout_status(output)?;
        self.update_prev_com_velocity(output, attributes)?;
        self.update_feet(output)?;
        self.update_hippy_jump_strength(output, attributes)
    }

    /// `UpdateWheelSlide` 0x827c4f28: the deck's sideways speed past the wheelslide threshold,
    /// and the rise of its four-frame average (carving).
    fn update_wheel_slide<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        attributes: &AudioAttributes,
        numeric: &mut N,
    ) -> Result<(), E<N::Error>> {
        let wheels = component(&output.concrete.collision, PhysOutComponent::Collision)?.wheels_in_contact;
        if wheels <= 0 {
            let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
            audio.wheel_slide = 0.0;
            audio.carving_slide = 0.0;
            return Ok(());
        }
        let range = scalar(&attributes.wheel_slide, WHEEL_SLIDE_RANGE, 0).map_err(E::Attribute)?;
        let minimum = scalar(&attributes.wheel_slide, WHEEL_SLIDE_MINIMUM, 0).map_err(E::Attribute)?;
        let right = component(&output.concrete.skateboard_reckoning, PhysOutComponent::SkateboardReckoning)?.transform.columns()[0];
        let velocity = output.motion_vector(PhysOutMotionVector::DeckVelocity).map_err(E::Access)?.value;
        let lateral = numeric.dot_vector(velocity, right).map_err(E::Numeric)?;
        let raw = if lateral >= 0.0 { lateral } else { -lateral };
        let excess = raw - minimum;
        let clipped = if excess >= 0.0 { excess } else { 0.0 };
        let mut slide = clipped / range;
        if !(slide <= 1.0) {
            slide = 1.0;
        }
        self.last_raw_wheel_slides[self.raw_wheel_slide_index as usize] = raw;
        let slides = self.last_raw_wheel_slides;
        let average = (((slides[0] + slides[1]) + slides[2]) + slides[3]) * 0.25;
        let carving = average - self.last_average_raw_wheel_slide;
        let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
        audio.wheel_slide = slide;
        audio.carving_slide = if carving >= 0.0 { carving } else { 0.0 };
        self.raw_wheel_slide_index = (self.raw_wheel_slide_index + 1) % 4;
        self.last_average_raw_wheel_slide = average;
        Ok(())
    }

    /// `UpdateWheelLanding` 0x827c5210: a wheel touching down after more than `kMinAirFrames`
    /// in the air lands with its speed into the contact normal; one in the air that long is no
    /// longer landed.
    fn update_wheel_landing<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        attributes: &AudioAttributes,
        numeric: &mut N,
    ) -> Result<(), E<N::Error>> {
        let collision = component(&output.concrete.collision, PhysOutComponent::Collision)?;
        let (contacts, normals) = (collision.wheel_in_contact, collision.wheel_contact_normal);
        let velocities = component(&output.concrete.motion, PhysOutComponent::Motion)?.wheel_linear_velocity;
        let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
        audio.prev_wheel_landing = audio.wheel_landing.clone();
        for wheel in 0..4 {
            let contact = contacts[wheel];
            let airborne = !self.wheel_in_contact[wheel] && self.frames_in_air[wheel] > MIN_AIR_FRAMES;
            if airborne {
                audio.wheel_landing.landed[wheel] = false;
            }
            if contact && airborne {
                let range = scalar(&attributes.contacts, WHEEL_IMPACT_RANGE, 0).map_err(E::Attribute)?;
                let into = numeric.dot_vector(normals[wheel], self.wheel_velocity[wheel]).map_err(E::Numeric)?;
                let speed = if into >= 0.0 { 0.0 } else { -into };
                let strength = speed / range;
                audio.wheel_landing.impact_strength[wheel] = if 1.0 - strength >= 0.0 { strength } else { 1.0 };
                audio.wheel_landing.landed[wheel] = true;
            }
            self.wheel_in_contact[wheel] = contact;
            self.wheel_velocity[wheel] = velocities[wheel];
            self.frames_in_air[wheel] = if contact { 0 } else { self.frames_in_air[wheel] + 1 };
        }
        Ok(())
    }

    /// `UpdateJump` 0x827c4c90: the jump impulse between `kSmallJumpDV` and `kLargeJumpDV`.
    fn update_jump<N: ReckoningNumeric>(&mut self, output: &mut PhysicalPlayerPhysOut, numeric: &mut N) -> Result<(), E<N::Error>> {
        let impulse = component(&output.concrete.air, PhysOutComponent::Air)?.jump_dv;
        let length = try_length(impulse, numeric).map_err(E::Numeric)?;
        let fraction = if !(length >= SMALL_JUMP_DV) {
            0.0
        } else if length > LARGE_JUMP_DV {
            1.0
        } else {
            (length - SMALL_JUMP_DV) / (LARGE_JUMP_DV - SMALL_JUMP_DV)
        };
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.jump_fraction = fraction;
        Ok(())
    }

    /// `UpdateAngularVelocity` 0x827c54e0: the torso's angular velocity change, and the deck's
    /// angular velocity in the skateboard frame.
    fn update_angular_velocity<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let omega = output.motion_vector(PhysOutMotionVector::AngularVelocity).map_err(E::Access)?.value;
        let skeleton = component_mut(&mut output.concrete.skeleton, PhysOutComponent::Skeleton)?;
        let torso = skeleton.skeleton_torso_angular_vel;
        skeleton.skeleton_torso_angular_vel_delta = torso.zip(self.torso_angular_velocity, |a, b| a - b);
        self.torso_angular_velocity = torso;
        let axes = *component(&output.concrete.skateboard_reckoning, PhysOutComponent::SkateboardReckoning)?.transform.columns();
        let [x, y, z] = omega.xyz;
        let lane = |axis: Option<&SkeletonRootVector>| {
            let [ax, ay, az] = axis.map_or([0.0; 3], |axis| axis.xyz);
            az.mul_add(z, ay.mul_add(y, ax * x))
        };
        let local = SkeletonRootVector::known([lane(Some(&axes[0])), lane(Some(&axes[1])), lane(Some(&axes[2])), lane(None)]);
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.deck_local_xyz_angular_speed = local;
        Ok(())
    }

    /// `UpdateGrinds` 0x827c3bd0: the last grind's type and surface, grinding (the grind
    /// category, or hanging on a primitive) and the last nonzero impact speed.
    fn update_grinds<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let grinds = component(&output.concrete.grinds, PhysOutComponent::Grinds)?;
        if grinds.is_grinding {
            self.grind_type = grinds.grind_type;
            self.grind_audio_surface_type = grinds.grind_audio_surface_type;
        }
        let (hanging, impact) = (grinds.hang_primitive_exists, grinds.grind_impact_speed);
        let state = component(&output.concrete.state, PhysOutComponent::State)?;
        let is_grinding = state.physics_state_category == PHYSICS_CATEGORY_GRIND
            || (state.physics_state == PHYSICS_STATE_NONSPECIFIC && hanging);
        let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
        audio.is_grinding = is_grinding;
        audio.grind_type = grind_type_raw(self.grind_type);
        audio.grind_audio_surface_type = self.grind_audio_surface_type;
        if impact <= 0.0 {
            audio.grind_impact_speed = self.last_grind_impact_speed;
        } else {
            audio.grind_impact_speed = impact;
            self.last_grind_impact_speed = impact;
        }
        Ok(())
    }

    /// `UpdateAveragedCollisionInfo` 0x827c5678: this frame's skeleton collision with each
    /// part's contact force the most of the last four frames (`GetMaxValueForPart` 0x827c3e50).
    fn update_averaged_collision_info<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let current = component(&output.concrete.collision, PhysOutComponent::Collision)?.skeleton.audio.clone();
        self.skeleton_collision[self.collision_index as usize] = current.clone();
        self.collision_index = (self.collision_index + 1) % 4;
        let mut averaged = current;
        for part in 0..8 {
            let mut most = self.skeleton_collision[0].contact_force[part];
            for frame in &self.skeleton_collision[1..] {
                if !(frame.contact_force[part] <= most) {
                    most = frame.contact_force[part];
                }
            }
            averaged.contact_force[part] = most;
        }
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.averaged_skeleton_collision_info = averaged;
        Ok(())
    }

    /// `UpdateAveragedBoardCollisionInfo` 0x827c3f28: the most deck contact force of the last
    /// four frames.
    fn update_averaged_board_collision_info<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let force = component(&output.concrete.collision, PhysOutComponent::Collision)?.deck_contact_force;
        self.board_collision[self.board_collision_index as usize] = force;
        self.board_collision_index = (self.board_collision_index + 1) % 4;
        let mut most = self.board_collision[0];
        for &force in &self.board_collision[1..] {
            if !(force <= most) {
                most = force;
            }
        }
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.averaged_deck_contact_force = most;
        Ok(())
    }

    /// `UpdateWipeoutStatus` 0x827c3ae0.
    fn update_wipeout_status<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let wiping = component(&output.concrete.state, PhysOutComponent::State)?.is_wiping_out;
        let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
        audio.just_wiped_out = !self.is_wiping_out && wiping;
        audio.just_respawned = self.is_wiping_out && !wiping;
        self.is_wiping_out = wiping;
        Ok(())
    }

    /// `UpdatePrevCOMVelocity` 0x827c65a0: the fastest fall of the centre of mass over four
    /// frames graded against the three off-board landing thresholds.
    fn update_prev_com_velocity<N>(&mut self, output: &mut PhysicalPlayerPhysOut, attributes: &AudioAttributes) -> Result<(), E<N>> {
        let velocity = output.system_vector(PhysOutSystemVector::CentreOfMassVelocity).map_err(E::Access)?.value;
        self.com_y_velocity[self.com_index as usize] = velocity.xyz[1];
        let history = self.com_y_velocity;
        let mut lowest = if -history[0] >= 0.0 { history[0] } else { 0.0 };
        for &value in &history[1..] {
            if lowest - value >= 0.0 {
                lowest = value;
            }
        }
        let fall = lowest.abs();
        let threshold = |index| scalar(&attributes.com_velocity, OFF_BOARD_LANDING, index).map_err(E::Attribute);
        let landing = if !(fall <= threshold(2)?) {
            4
        } else if !(fall <= threshold(1)?) {
            3
        } else if fall > threshold(0)? {
            2
        } else {
            1
        };
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.off_board_jump_landing = landing;
        self.com_index = if self.com_index + 1 < 4 { self.com_index + 1 } else { 0 };
        Ok(())
    }

    /// `UpdateFeet` 0x827c6208: stairs (both flags from the left foot's surface), each foot's
    /// touchdown and its ground position, and walking up or down from the height between the
    /// last touchdowns.
    fn update_feet<N>(&mut self, output: &mut PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        let skeleton = component(&output.concrete.skeleton, PhysOutComponent::Skeleton)?;
        let (left_position, right_position) = (skeleton.skeleton_foot_left_pos, skeleton.skeleton_foot_right_pos);
        let offboard = component(&output.concrete.offboard, PhysOutComponent::OffBoard)?;
        let mut right = offboard.right_foot_surface_id as u16;
        if right == 0 {
            right = self.right_foot_surface;
        }
        self.right_foot_surface = right;
        let mut left = offboard.left_foot_surface_id as u16;
        if left == 0 {
            left = self.left_foot_surface;
        }
        self.left_foot_surface = left;
        let stairs = (left >> 7) & 0x1f == STAIRS_CATEGORY;
        self.right_foot_on_stairs = stairs;
        self.left_foot_on_stairs = stairs;
        self.right_foot_was_touching = self.right_foot_touching;
        self.left_foot_was_touching = self.left_foot_touching;
        self.left_foot_touching = offboard.left_foot_down;
        self.right_foot_touching = offboard.right_foot_down;
        let right_landed = self.right_foot_touching && !self.right_foot_was_touching;
        let left_landed = self.left_foot_touching && !self.left_foot_was_touching;
        if right_landed {
            self.last_right_foot_ground_position = right_position;
        }
        if left_landed {
            self.last_left_foot_ground_position = left_position;
        }
        let audio = component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?;
        audio.is_on_stairs = stairs;
        audio.right_foot_just_landed = right_landed;
        audio.left_foot_just_landed = left_landed;
        let rise = self.last_right_foot_ground_position.xyz[1] - self.last_left_foot_ground_position.xyz[1];
        if rise.abs() <= UP_DOWN_TOLERANCE {
            audio.is_walking_up = false;
            audio.is_walking_down = false;
        } else if self.right_foot_touching {
            if rise <= 0.0 {
                audio.is_walking_down = true;
            } else {
                audio.is_walking_up = true;
            }
        } else if self.left_foot_touching {
            if rise <= 0.0 {
                audio.is_walking_up = true;
            } else {
                audio.is_walking_down = true;
            }
        }
        Ok(())
    }

    /// `UpdateHippyJumpStrength` 0x827c4dc8: the ground jump strength against the two
    /// thresholds.
    fn update_hippy_jump_strength<N>(&mut self, output: &mut PhysicalPlayerPhysOut, attributes: &AudioAttributes) -> Result<(), E<N>> {
        let strength = component(&output.concrete.ground, PhysOutComponent::Ground)?.jump_strength;
        let threshold = |index| scalar(&attributes.clothing, HIPPY_JUMP_STRENGTH, index).map_err(E::Attribute);
        let level = if !(strength <= threshold(1)?) {
            2
        } else if !(strength <= threshold(0)?) {
            1
        } else {
            0
        };
        component_mut(&mut output.concrete.audio, PhysOutComponent::Audio)?.hippy_jump_strength = level;
        Ok(())
    }
}
