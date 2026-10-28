//! Ground.Post82d8e9b4 detector82de09c0. Retained callback operations, no snapshots.
#[path = "physical_player_ground_wipeout_attributes.rs"]
pub(crate) mod attributes;
#[path = "physical_player_ground_wipeout_leaves.rs"]
pub(crate) mod leaves;
#[cfg(test)]
#[path = "physical_player_ground_wipeout_tests.rs"]
mod tests;
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
use attributes::Attribute;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scalar {
    TimeDelta,
    SkaterContact,
    VehicleContact,
    MaxDisplacement,
    DeckSpeed,
    UpY,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionFlag {
    HitVehicle,
    CompliantJustCollided,
    OpposingContacts,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Vector {
    HookToHips,
    MaxContactVelocity,
    MostVerticalContactNormal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Matrix {
    BoardTransform,
    ProcessedSkateTransform,
    WorldToAnimation,
}
pub(crate) trait GroundWipeoutSource {
    type Error;
    type AttributePointer;
    type DrawTarget;
    fn state(&mut self) -> Result<i32, Self::Error>;
    fn set_state(&mut self, state: i32) -> Result<(), Self::Error>;
    fn off_balance_total(&mut self) -> Result<f32, Self::Error>;
    fn set_off_balance_total(&mut self, total: f32) -> Result<(), Self::Error>;
    fn invincible_timer(&mut self) -> Result<f32, Self::Error>;
    fn set_invincible_timer(&mut self, timer: f32) -> Result<(), Self::Error>;
    fn scalar(&mut self, field: Scalar) -> Result<f32, Self::Error>;
    /// Require only the requested known bits, on the actual retained Processed.
    fn processed_flag(&mut self, offset: u32, mask: u32) -> Result<bool, Self::Error>;
    fn collision_flag(&mut self, flag: CollisionFlag) -> Result<bool, Self::Error>;
    fn board_contact_flag(&mut self, mask: u32) -> Result<bool, Self::Error>;
    fn num_parts_in_contact(&mut self) -> Result<u8, Self::Error>;
    fn vector(&mut self, field: Vector) -> Result<SkeletonRootVector, Self::Error>;
    fn matrix(&mut self, field: Matrix) -> Result<SkeletonRootMatrix, Self::Error>;
    fn contact_force(&mut self, index: usize) -> Result<f32, Self::Error>;
    /// Each lookup requires the actual `GlobalAttributeCollection` binding.
    fn attribute(&mut self, attribute: Attribute) -> Result<Self::AttributePointer, Self::Error>;
    fn attribute_value(&mut self, pointer: &Self::AttributePointer) -> Result<f32, Self::Error>;
    fn capture_draw_target(&mut self) -> Result<Self::DrawTarget, Self::Error>;
    fn read_draw_target(&mut self, target: &Self::DrawTarget) -> Result<bool, Self::Error>;
    fn wipeout_on_collision(&mut self) -> Result<bool, Self::Error>;
    /// Native debug branches still require their owners/read payloads. Runtime
    /// adapter must implement or return explicit pending, never silently skip.
    fn draw_squashed(&mut self) -> Result<(), Self::Error>;
    fn draw_displacement(&mut self) -> Result<(), Self::Error>;
    fn draw_contact_force(&mut self, body: f32, arms: f32) -> Result<(), Self::Error>;
    fn require_positive_balance_speed(&mut self, speed: f32) -> Result<(), Self::Error>;
    /// Reuse existing source numerical services (dot3, two-refinement length,
    /// `XMVectorACos` polynomial); preserve their explicit host parity boundary.
    fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, Self::Error>;
    fn length(&mut self, vector: SkeletonRootVector) -> Result<f32, Self::Error>;
    fn acos(&mut self, scalar: f32) -> Result<f32, Self::Error>;
    fn minimum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error>;
    fn maximum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error>;
    /// Same initialized Wipeout ledger: set request[index], value=+0, then
    /// wrapping event-count increment even for duplicate types. No enable gate.
    fn request(&mut self, index: usize) -> Result<(), Self::Error>;
}
fn read_attribute<S: GroundWipeoutSource>(
    source: &mut S,
    attribute: Attribute,
) -> Result<f32, S::Error> {
    let pointer = source.attribute(attribute)?;
    source.attribute_value(&pointer)
}
/// Only an ordered positive timer subtracts actual dt and returns. No clamp.
/// Raw82de0a04=40990020 branches on CR6.GT clear, including unordered.
#[expect(
    clippy::assign_op_pattern,
    reason = "preserve native floating-point operand order"
)]
pub(crate) fn try_check_ground<S: GroundWipeoutSource>(s: &mut S) -> Result<(), S::Error> {
    use Attribute as A;
    if s.state()? != 1 {
        s.set_off_balance_total(0.0)?;
        s.set_state(1)?;
    }
    let timer = s.invincible_timer()?;
    if timer > 0.0 {
        let dt = s.scalar(Scalar::TimeDelta)?;
        s.set_invincible_timer(timer - dt)?;
        return Ok(());
    }
    let special = s.processed_flag(0x8c8, 1)?;
    let squash = read_attribute(s, if special { A::SquashSpecial } else { A::Squash })?;
    let mut skater = 1.0;
    if !s.processed_flag(0x8c8, 0x4000_0000)? && s.scalar(Scalar::SkaterContact)? > 0.0 {
        skater = read_attribute(s, A::SkaterScalar)?;
    }
    // Native lhz+8c8 &1: big-endian upper halfword, NOT word bit0.
    let trick = if s.processed_flag(0x8c8, 0x0001_0000)? {
        f32::from_bits(0x3e99_999a)
    } else {
        1.0
    };
    let vehicle = if s.collision_flag(CollisionFlag::HitVehicle)? {
        read_attribute(s, A::VehicleScalar)?
    } else {
        1.0
    };
    let (mut contact, mut arms) = (1.0, 1.0);
    if s.processed_flag(0x8cc, 0x10)? {
        contact = read_attribute(s, A::SkitchContactScalar)?;
        arms = read_attribute(s, A::SkitchArmScalar)?;
    }
    let common = ((vehicle * trick) * contact) * skater;
    if leaves::squashed(s, vehicle * squash)? {
        s.request(18)?;
    } else {
        let displacement = read_attribute(s, A::GroundDisplacement)?;
        if leaves::displacement(s, displacement * common)? {
            s.request(1)?;
        } else {
            let arm_pointer = s.attribute(A::GroundArms)?;
            let body_pointer = s.attribute(A::GroundContact)?;
            let arm_value = s.attribute_value(&arm_pointer)?;
            let body_value = s.attribute_value(&body_pointer)?;
            if leaves::contact_force(s, body_value * common, arms * arm_value)? {
                s.request(0)?;
            }
        }
    }
    leaves::vehicle(s)?;
    let angle = read_attribute(s, A::GroundFlip)?;
    leaves::deck_flipped(s, angle)?;
    let (xz, y) = if s.processed_flag(0x8c8, 0x0001_0000)? {
        let y_pointer = s.attribute(A::TrickY)?;
        let xz_pointer = s.attribute(A::TrickXZ)?;
        let y = s.attribute_value(&y_pointer)?;
        let xz = s.attribute_value(&xz_pointer)?;
        (xz, y)
    } else {
        let mut xz = read_attribute(s, A::GroundXZ)?;
        let mut y = read_attribute(s, A::GroundY)?;
        if s.board_contact_flag(0x4000_0000)? {
            let ai = s.processed_flag(0x8c8, 0x4000_0000)?;
            let scalar = read_attribute(
                s,
                if ai {
                    A::DeckAiScalar
                } else {
                    A::DeckPlayerScalar
                },
            )?;
            xz = scalar * xz;
            y = scalar * y;
        }
        if s.board_contact_flag(0x2000_0000)? {
            xz *= read_attribute(s, A::LightObjectScalar)?;
        }
        if s.processed_flag(0x8cc, 0x10)? {
            let scalar = read_attribute(s, A::DeckSkitchScalar)?;
            xz = scalar * xz;
            y = scalar * y;
        }
        (xz, y)
    };
    leaves::deck_acceleration(s, xz, y)?;
    leaves::off_balance(s)?;
    if s.collision_flag(CollisionFlag::OpposingContacts)? {
        s.request(16)?;
    }
    leaves::leaning(s)
}
