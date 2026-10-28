//! `LivingWorldPresentation`'s restore side: the entities `DoSimSubmit` 0x8280e8b0
//! records each tick, and `cLivingWorldPresEntityManager::UpdateLivingWorldPresentationEntities`
//! 0x828133e0's blend of each one between two frames, which `DoSimRestore` 0x82813a90
//! runs every frame. The props, `tLWReplayData`'s third kind, have no drawn entity here.

use super::math::{matrix_slerp, slerp};
use crate::{
    actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::compose,
    actor_face_playback::presentation::move_root::rigid_matrix,
    animation::AnimationMatrix,
    reckoning_numeric::ReckoningHostNumericError,
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    world::try_simulation_quaternion_candidate,
};
use std::collections::BTreeMap;

/// `DoSimSubmit` 0x8280f1cc..0x8280f204's scales from radians to the stored steps.
const SPIN_STEPS: f32 = f32::from_bits(0x4622_f8e0);
const TURN_STEPS: f32 = f32::from_bits(0x4321_b390);
/// `cPresVehicle::UpdateFromReplayData`'s scales from the stored steps to radians.
const SPIN_RADIANS: f32 = f32::from_bits(0x38c9_10a4);
const TURN_RADIANS: f32 = f32::from_bits(0x3bca_a525);
/// `gIVector` and `gJVector`: the spin's and the turn's axes.
const SPIN_AXIS: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
const TURN_AXIS: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

/// A transform as a record stores it: the rotation as the inlined `QuatFromMatrix` leaves
/// it, unnormalized, and the translation row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TransformReplay {
    pub(crate) rotation: [f32; 4],
    pub(crate) translation: [f32; 4],
}

impl TransformReplay {
    /// `transform`'s x, y and z axis rows, then its translation row.
    fn new(transform: [f32; 16]) -> Self {
        let rotation = try_simulation_quaternion_candidate(transform, |magnitudes| {
            Ok::<_, std::convert::Infallible>(magnitudes.map(|value| 1.0 / value.sqrt()))
        })
        .unwrap_or_else(|never| match never {});
        Self {
            rotation,
            translation: [transform[12], transform[13], transform[14], transform[15]],
        }
    }

    /// The matrix the restore rebuilds: the rotation through the √2-scaled conversion,
    /// the translation's W set to one.
    fn matrix(&self) -> AnimationMatrix {
        rigid_matrix(self.rotation, self.translation)
    }
}

/// One entity's `tBasicEntityReplayData`: its ID and transform.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EntityReplay {
    pub(crate) id: u32,
    pub(crate) transform: TransformReplay,
}

impl EntityReplay {
    /// The entity `id` at `transform`: x, y and z axis rows, then the translation row.
    pub(crate) fn new(id: u32, transform: [f32; 16]) -> Self {
        Self {
            id,
            transform: TransformReplay::new(transform),
        }
    }
}

/// A vehicle's record: the entity, and the word packing the wheels' spin and turn above
/// the lights (`mWheelSpinAngle`, `mWheelTurnAngle`, `mLightsStatus`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct VehicleReplay {
    pub(crate) entity: EntityReplay,
    pub(crate) wheels: u32,
}

impl VehicleReplay {
    /// `DoSimSubmit`: each angle scaled to steps and truncated, keeping the spin's low
    /// sixteen bits and the turn's low eight.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "fctidz, then the stored low bits"
    )]
    pub(crate) fn new(id: u32, transform: [f32; 16], spin: f32, turn: f32, lights: u8) -> Self {
        let spin = u32::from((spin * SPIN_STEPS) as i64 as u16);
        let turn = u32::from((turn * TURN_STEPS) as i64 as u8);
        Self {
            entity: EntityReplay::new(id, transform),
            wheels: spin << 16 | turn << 8 | u32::from(lights),
        }
    }

    #[expect(clippy::cast_possible_truncation, reason = "the stored sixteen bits")]
    fn spin(&self) -> f32 {
        f32::from((self.wheels >> 16) as u16) * SPIN_RADIANS
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "the stored signed byte"
    )]
    fn turn(&self) -> f32 {
        f32::from((self.wheels >> 8) as u8 as i8) * TURN_RADIANS
    }

    #[expect(clippy::cast_possible_truncation, reason = "the stored low byte")]
    fn lights(&self) -> u8 {
        self.wheels as u8
    }

    /// The wheels a vehicle new to the presentation takes: its own record, all the way.
    pub(crate) fn wheels(&self) -> VehicleWheels {
        VehicleWheels::restore(self, self, 1.0)
    }
}

/// A dynamic object's record: the entity and its part's transform.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ObjectReplay {
    pub(crate) entity: EntityReplay,
    pub(crate) part: TransformReplay,
}

impl ObjectReplay {
    /// The object `id` at `transform`, its part at `part`.
    pub(crate) fn new(id: u32, transform: [f32; 16], part: [f32; 16]) -> Self {
        Self {
            entity: EntityReplay::new(id, transform),
            part: TransformReplay::new(part),
        }
    }
}

/// One tick's `tLWReplayData`.
#[derive(Clone, Debug, Default)]
pub(crate) struct LivingWorldSubmission {
    pub(crate) pedestrians: Vec<EntityReplay>,
    pub(crate) vehicles: Vec<VehicleReplay>,
    pub(crate) objects: Vec<ObjectReplay>,
}

/// What `cPresVehicle::UpdateFromReplayData` 0x82807e88 leaves for the wheel bones'
/// procedural rotations and the lights' shader constants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleWheels {
    /// The rear wheels: the spin about X.
    pub spin: [f32; 4],
    /// The front wheels: the turn about Y composed with the spin.
    pub front: [f32; 4],
    /// `i_lights`: brake or right, brake or left, right, left.
    pub lights: [f32; 4],
}

impl VehicleWheels {
    /// Each angle's half-angle quaternion in both frames, slerped `amount` of the way;
    /// the lights are the next frame's.
    fn restore(previous: &VehicleReplay, next: &VehicleReplay, amount: f32) -> Self {
        let quaternion = |axis: [f32; 4], angle: f32| {
            let (sine, cosine) = try_sin_cos(angle * 0.5, &mut HostReckoningSinCosNumeric)
                .unwrap_or_else(|never| match never {});
            [axis[0] * sine, axis[1] * sine, axis[2] * sine, cosine]
        };
        let spin = slerp(
            quaternion(SPIN_AXIS, previous.spin()),
            quaternion(SPIN_AXIS, next.spin()),
            amount,
        );
        let turn = slerp(
            quaternion(TURN_AXIS, previous.turn()),
            quaternion(TURN_AXIS, next.turn()),
            amount,
        );
        Self {
            spin,
            front: compose(turn, spin),
            lights: Self::lights(next.lights()),
        }
    }

    /// The light flags as `UpdateFromReplayData` 0x828084c0..0x82808588 reads them: the
    /// indicators need the hazard bit (0x80); the brake (2) lights both tails.
    fn lights(flags: u8) -> [f32; 4] {
        let left = flags & 0x80 != 0 && flags & 8 != 0;
        let right = flags & 0x80 != 0 && flags & 16 != 0;
        let brake = flags & 2 != 0;
        [brake || right, brake || left, right, left].map(|lit| f32::from(u8::from(lit)))
    }
}

/// The detail an entity's `UpdateFromReplayData` restores besides its transform.
#[derive(Clone, Copy, Debug)]
enum Detail {
    Pedestrian,
    Vehicle(VehicleWheels),
    /// `cPresDynamicObject::mPartTransforms[0]`.
    DynamicObject(AnimationMatrix),
}

/// One presentation entity as the last restore left it.
#[derive(Clone, Copy, Debug)]
struct RestoredEntity {
    /// `SetLocalToWorldMatrix`.
    transform: AnimationMatrix,
    detail: Detail,
}

/// A kind of record, restored the way its presentation entity is.
trait Replay {
    fn entity(&self) -> &EntityReplay;

    /// `UpdateFromReplayData(previous, next, amount)`.
    fn detail(
        previous: &Self,
        next: &Self,
        amount: f32,
    ) -> Result<Detail, ReckoningHostNumericError>;
}

impl Replay for EntityReplay {
    fn entity(&self) -> &EntityReplay {
        self
    }

    /// `cPresPedestrian::UpdateFromReplayData` 0x82809d98 restores the next frame's
    /// post-adjust rotations, which the tick's pose already carries.
    fn detail(_: &Self, _: &Self, _: f32) -> Result<Detail, ReckoningHostNumericError> {
        Ok(Detail::Pedestrian)
    }
}

impl Replay for VehicleReplay {
    fn entity(&self) -> &EntityReplay {
        &self.entity
    }

    fn detail(
        previous: &Self,
        next: &Self,
        amount: f32,
    ) -> Result<Detail, ReckoningHostNumericError> {
        Ok(Detail::Vehicle(VehicleWheels::restore(
            previous, next, amount,
        )))
    }
}

impl Replay for ObjectReplay {
    fn entity(&self) -> &EntityReplay {
        &self.entity
    }

    /// `cPresDynamicObject::UpdateFromReplayData` 0x8280a040: the part's matrix slerp.
    fn detail(
        previous: &Self,
        next: &Self,
        amount: f32,
    ) -> Result<Detail, ReckoningHostNumericError> {
        Ok(Detail::DynamicObject(matrix_slerp(
            &previous.part.matrix(),
            &next.part.matrix(),
            amount,
        )?))
    }
}

/// The record kinds in `tLWReplayData` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Pedestrian,
    Vehicle,
    DynamicObject,
}

/// `cLivingWorldPresEntityManager`'s presentation entities.
#[derive(Default)]
pub(crate) struct LivingWorldRestore {
    entities: BTreeMap<(Kind, u32), RestoredEntity>,
}

impl LivingWorldRestore {
    /// `UpdateLivingWorldPresentationEntities`: each entity `next` records that already has
    /// a presentation entity is blended `percent` of the way from its record in `previous`,
    /// or from `next`'s own when `previous` has none; the others are created at their
    /// `next` transform, and the entities `next` no longer records are destroyed.
    pub(crate) fn restore(
        &mut self,
        previous: Option<&LivingWorldSubmission>,
        next: &LivingWorldSubmission,
        percent: f32,
    ) -> Result<(), ReckoningHostNumericError> {
        let mut restored = BTreeMap::new();
        self.restore_kind(
            &mut restored,
            Kind::Pedestrian,
            previous.map(|frame| frame.pedestrians.as_slice()),
            &next.pedestrians,
            percent,
        )?;
        self.restore_kind(
            &mut restored,
            Kind::Vehicle,
            previous.map(|frame| frame.vehicles.as_slice()),
            &next.vehicles,
            percent,
        )?;
        self.restore_kind(
            &mut restored,
            Kind::DynamicObject,
            previous.map(|frame| frame.objects.as_slice()),
            &next.objects,
            percent,
        )?;
        self.entities = restored;
        Ok(())
    }

    /// The entity's matrix is slerped by `percent` even when it blends from its own next
    /// record; its `UpdateFromReplayData` takes one then.
    fn restore_kind<R: Replay>(
        &self,
        restored: &mut BTreeMap<(Kind, u32), RestoredEntity>,
        kind: Kind,
        previous: Option<&[R]>,
        next: &[R],
        percent: f32,
    ) -> Result<(), ReckoningHostNumericError> {
        let previous: BTreeMap<u32, &R> = previous
            .unwrap_or_default()
            .iter()
            .map(|record| (record.entity().id, record))
            .collect();
        for record in next {
            let id = record.entity().id;
            let entity = if self.entities.contains_key(&(kind, id)) {
                let (from, amount) = previous
                    .get(&id)
                    .map_or((record, 1.0), |&from| (from, percent));
                RestoredEntity {
                    transform: matrix_slerp(
                        &from.entity().transform.matrix(),
                        &record.entity().transform.matrix(),
                        percent,
                    )?,
                    detail: R::detail(from, record, amount)?,
                }
            } else {
                RestoredEntity {
                    transform: record.entity().transform.matrix(),
                    detail: R::detail(record, record, 1.0)?,
                }
            };
            restored.insert((kind, id), entity);
        }
        Ok(())
    }

    /// Forget every presentation entity.
    pub(crate) fn reset(&mut self) {
        self.entities.clear();
    }

    /// The pedestrian `id`'s transform.
    pub(crate) fn pedestrian(&self, id: u32) -> Option<AnimationMatrix> {
        self.entities
            .get(&(Kind::Pedestrian, id))
            .map(|entity| entity.transform)
    }

    /// The vehicle `id`'s transform and wheels.
    pub(crate) fn vehicle(&self, id: u32) -> Option<(AnimationMatrix, VehicleWheels)> {
        let entity = self.entities.get(&(Kind::Vehicle, id))?;
        match entity.detail {
            Detail::Vehicle(wheels) => Some((entity.transform, wheels)),
            Detail::Pedestrian | Detail::DynamicObject(_) => None,
        }
    }

    /// The dynamic object `id`'s part transform.
    pub(crate) fn dynamic_object(&self, id: u32) -> Option<AnimationMatrix> {
        match self.entities.get(&(Kind::DynamicObject, id))?.detail {
            Detail::DynamicObject(part) => Some(part),
            Detail::Pedestrian | Detail::Vehicle(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "presentation_restore_living_world_tests.rs"]
mod tests;
