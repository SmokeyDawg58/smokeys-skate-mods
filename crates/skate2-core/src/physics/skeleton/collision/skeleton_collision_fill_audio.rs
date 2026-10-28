//! `CollisionData` child: mandatory audio collision data at82b3efc0.
use super::{SkeletonCollisionData, SkeletonCollisionVector};
use crate::{reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector};

pub(crate) const AUDIO_KEYS: [u64; 4] = [
    0x06c1_0e1f_5e1b_4730,
    0xad7e_cc44_569e_bef9,
    0xfbf5_cd77_34e8_ae5a,
    0x1430_bd50_f0a3_3475,
];
pub(crate) trait SkeletonAudioAttributes {
    type Error;
    type Global;
    type Pointer;
    fn capture(&mut self) -> Result<Self::Global, Self::Error>;
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
}
pub(crate) trait SkeletonAudioServices {
    type Error;
    fn velocity_delta(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn mass(&mut self, part: usize) -> Result<f32, Self::Error>;
    fn draw_audio_info(&mut self) -> Result<bool, Self::Error>;
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioFlag {
    Unrideable,
    PartialRagdoll,
    Impaled,
    Face,
    Groin,
    RecentGroin,
    RecentFace,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioScalar {
    ContactTotal,
    SlideTotal,
    TimeWipingOut,
    Vehicle,
    Skater,
    Pedestrian,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioBoneVector {
    Position,
    Normal,
    Slide,
    SubNormal,
    SubSlide,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioBoneScalar {
    Acceleration,
    Force,
    SubAcceleration,
    SubForce,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioBoneFlag {
    Vehicle,
    Pedestrian,
    Skater,
    World,
}
/// A borrow of the actual Collision output pointer captured once atf17c.
/// Constant bounded indices and writes cannot replace or re-guard that pointer.
pub(crate) trait SkeletonAudioRecord {
    fn flag(&mut self, field: AudioFlag, value: u8);
    fn scalar(&mut self, field: AudioScalar, value: f32);
    fn bone_vector(&mut self, part: usize, field: AudioBoneVector, value: SkeletonRootVector);
    fn bone_scalar(&mut self, part: usize, field: AudioBoneScalar, value: f32);
    fn bone_surface(&mut self, part: usize, sub: bool, value: u32);
    fn bone_flag(&mut self, part: usize, field: AudioBoneFlag, value: u8);
    fn slide_speed(&mut self, category: usize, value: f32);
    fn contact_force(&mut self, category: usize, value: f32);
    fn surface(&mut self, category: usize, value: u32);
}
pub(crate) trait SkeletonAudioOutput {
    type Error;
    type Guard;
    type Record<'a>: SkeletonAudioRecord
    where
        Self: 'a;
    fn sample_guard(&mut self) -> Result<Self::Guard, Self::Error>;
    fn begin(&mut self, guard: Self::Guard) -> Result<Self::Record<'_>, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkeletonAudioError<S, A, O, N> {
    Source(S),
    Attribute(A),
    Output(O),
    Numeric(N),
    UnwrittenSpecificContact { recent: bool, index: usize },
}
impl<S: std::fmt::Debug, A: std::fmt::Debug, O: std::fmt::Debug, N: std::fmt::Debug>
    std::fmt::Display for SkeletonAudioError<S, A, O, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Skeleton audio data source: {e:?}"),
            Self::Attribute(e) => write!(f, "Skeleton audio data attribute: {e:?}"),
            Self::Output(e) => write!(f, "Skeleton audio data output: {e:?}"),
            Self::Numeric(e) => write!(f, "Skeleton audio data numeric: {e:?}"),
            Self::UnwrittenSpecificContact { recent, index } => write!(
                f,
                "Skeleton audio data unwritten contact {index}, recent={recent}"
            ),
        }
    }
}
fn vector(v: SkeletonCollisionVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: *v.xyz(),
        w: v.known_lanes().map(|a| a[3]),
    }
}
fn absolute(v: f32) -> f32 {
    f32::from_bits(v.to_bits() & 0x7fff_ffff)
}
fn bound_total(v: f32) -> f32 {
    let low = if -v >= 0.0 { 0.0 } else { v };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}
fn bound_force(v: f32) -> f32 {
    let minimum = f32::from_bits(0x3a83_126f);
    let low = if minimum - v >= 0.0 { minimum } else { v };
    bound_total(low)
}

impl SkeletonCollisionData {
    #[expect(
        clippy::type_complexity,
        clippy::too_many_lines,
        clippy::similar_names,
        reason = "retain distinct source scalars, gates and accumulator rates in ordered publication"
    )]
    pub(crate) fn try_fill_audio_collision<
        A: SkeletonAudioAttributes,
        S: SkeletonAudioServices,
        O: SkeletonAudioOutput,
        N: ReckoningNumeric,
    >(
        &mut self,
        attributes: &mut A,
        services: &mut S,
        output: &mut O,
        numeric: &mut N,
    ) -> Result<(), SkeletonAudioError<S::Error, A::Error, O::Error, N::Error>> {
        use SkeletonAudioError as E;
        let global = attributes.capture().map_err(E::Attribute)?;
        let p = attributes
            .lookup(&global, AUDIO_KEYS[0])
            .map_err(E::Attribute)?;
        let global = attributes.capture().map_err(E::Attribute)?;
        let lifetime = attributes.float(&p).map_err(E::Attribute)?;
        let p = attributes
            .lookup(&global, AUDIO_KEYS[1])
            .map_err(E::Attribute)?;
        let global = attributes.capture().map_err(E::Attribute)?;
        let contact_scalar = attributes.float(&p).map_err(E::Attribute)?;
        let p = attributes
            .lookup(&global, AUDIO_KEYS[2])
            .map_err(E::Attribute)?;
        let global = attributes.capture().map_err(E::Attribute)?;
        let slide_scalar = attributes.float(&p).map_err(E::Attribute)?;
        let p = attributes
            .lookup(&global, AUDIO_KEYS[3])
            .map_err(E::Attribute)?;
        let guard = output.sample_guard().map_err(E::Output)?; //f154 before fourth float
        let force_scalar = attributes.float(&p).map_err(E::Attribute)?;
        let mut out = output.begin(guard).map_err(E::Output)?;
        out.flag(AudioFlag::Unrideable, self.unrideable_contact);
        out.flag(AudioFlag::PartialRagdoll, self.compliant_active);
        out.flag(AudioFlag::Impaled, self.opposing_contacts_on_plane);
        for part in 0..24 {
            let main_normal = vector(self.bone_data[part].contact_normal);
            let delta = services.velocity_delta(part).map_err(E::Source)?;
            let main_acceleration = numeric.dot_vector(delta, main_normal).map_err(E::Numeric)?;
            let sub_normal = vector(self.bone_data[part].sub_part_contact_normal);
            let position = services.position(part).map_err(E::Source)?;
            let sub_acceleration = numeric.dot_vector(delta, sub_normal).map_err(E::Numeric)?;
            out.bone_vector(part, AudioBoneVector::Position, position);
            out.bone_scalar(
                part,
                AudioBoneScalar::Acceleration,
                absolute(main_acceleration),
            );
            out.bone_vector(
                part,
                AudioBoneVector::Normal,
                vector(self.bone_data[part].contact_normal),
            );
            out.bone_vector(
                part,
                AudioBoneVector::Slide,
                vector(self.bone_data[part].slide_vector),
            );
            out.bone_scalar(
                part,
                AudioBoneScalar::Force,
                self.bone_data[part].contact_force,
            );
            out.bone_surface(part, false, self.bone_data[part].contact_surface_id);
            out.bone_scalar(
                part,
                AudioBoneScalar::SubAcceleration,
                absolute(sub_acceleration),
            );
            out.bone_vector(
                part,
                AudioBoneVector::SubNormal,
                vector(self.bone_data[part].sub_part_contact_normal),
            );
            out.bone_vector(
                part,
                AudioBoneVector::SubSlide,
                vector(self.bone_data[part].sub_part_slide_vector),
            );
            out.bone_scalar(
                part,
                AudioBoneScalar::SubForce,
                self.bone_data[part].sub_part_contact_force,
            );
            out.bone_surface(part, true, self.bone_data[part].sub_part_contact_surface_id);
            out.bone_flag(
                part,
                AudioBoneFlag::Vehicle,
                self.bone_data[part].hit_vehicle,
            );
            out.bone_flag(
                part,
                AudioBoneFlag::Pedestrian,
                self.bone_data[part].hit_pedestrian,
            );
            out.bone_flag(part, AudioBoneFlag::Skater, self.bone_data[part].hit_skater);
            out.bone_flag(part, AudioBoneFlag::World, self.bone_data[part].hit_world);
        }
        for category in 0..8 {
            let part = self.contact_bone[category];
            out.slide_speed(category, 0.0);
            out.contact_force(category, 0.0);
            if let Some(part) = part {
                let speed = self.slide_speed[category];
                let normal = vector(self.contact_normal[category]);
                let delta = services.velocity_delta(part.index()).map_err(E::Source)?;
                let square = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
                let mass = services.mass(part.index()).map_err(E::Source)?;
                // Both raw BLE are BO4/BI25, i.e. NOTGT including unordered.
                let contact_gate = if square > f32::from_bits(0x3dcc_cccd) {
                    1.0
                } else {
                    0.0
                };
                let slide_gate = if speed > 0.5 { 1.0 } else { 0.0 };
                let inverse_lifetime = 1.0 / lifetime; //native FDIVS, no estimate reuse
                let acceleration = numeric.dot_vector(delta, normal).map_err(E::Numeric)?;
                let old_contact = self.contact_force_total_during_wipeout;
                let old_slide = self.slide_force_total_during_wipeout;
                let slide_rate = inverse_lifetime * slide_scalar;
                let contact_rate = inverse_lifetime * contact_scalar;
                let slide_rate = slide_rate * slide_gate;
                let acceleration = absolute(acceleration);
                let weighted = acceleration * mass;
                let force = weighted * force_scalar;
                let contact_rate = contact_rate * acceleration;
                self.slide_force_total_during_wipeout = slide_rate.mul_add(speed, old_slide); //f434
                self.contact_force_total_during_wipeout =
                    contact_rate.mul_add(contact_gate, old_contact); //f440
                out.slide_speed(category, speed);
                out.contact_force(category, bound_force(force));
                out.surface(category, self.audio_surface_type[category]);
            }
            if services.draw_audio_info().map_err(E::Source)? {
                for _ in 0..2 {
                    services.debug_overlay_noop().map_err(E::Source)?;
                }
            }
        }
        // f5ac captures face BEFORE both accumulator clamp stores.
        let face = self.specific_parts.contact[1].ok_or(E::UnwrittenSpecificContact {
            recent: false,
            index: 1,
        })?;
        self.contact_force_total_during_wipeout =
            bound_total(self.contact_force_total_during_wipeout);
        self.slide_force_total_during_wipeout = bound_total(self.slide_force_total_during_wipeout);
        out.flag(AudioFlag::Face, face);
        out.flag(
            AudioFlag::Groin,
            self.specific_parts.contact[0].ok_or(E::UnwrittenSpecificContact {
                recent: false,
                index: 0,
            })?,
        );
        out.flag(
            AudioFlag::RecentGroin,
            self.specific_parts.recent_contact[0].ok_or(E::UnwrittenSpecificContact {
                recent: true,
                index: 0,
            })?,
        );
        out.flag(
            AudioFlag::RecentFace,
            self.specific_parts.recent_contact[1].ok_or(E::UnwrittenSpecificContact {
                recent: true,
                index: 1,
            })?,
        );
        out.scalar(
            AudioScalar::ContactTotal,
            self.contact_force_total_during_wipeout,
        );
        out.scalar(
            AudioScalar::SlideTotal,
            self.slide_force_total_during_wipeout,
        );
        out.scalar(AudioScalar::TimeWipingOut, self.time_in_wipeout);
        out.scalar(AudioScalar::Vehicle, self.vehicle_contact);
        out.scalar(AudioScalar::Skater, self.skater_contact);
        out.scalar(AudioScalar::Pedestrian, self.pedestrian_contact);
        if services.draw_audio_info().map_err(E::Source)? {
            for _ in 0..4 {
                services.debug_overlay_noop().map_err(E::Source)?;
            }
            for _ in 0..24 {
                services.debug_overlay_noop().map_err(E::Source)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "skeleton_collision_fill_audio_tests.rs"]
mod tests;
