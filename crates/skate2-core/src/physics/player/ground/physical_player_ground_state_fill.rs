//! Same retained Ground instance at Update5 virtual State.Fill82e05b74.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::ground_state_fill::{
        GroundFillFlag, GroundFillScalar, GroundStateFillError, GroundStateFillSource,
    },
    physical_player_physics_state::{CurrentPhysicalState, PhysicsGroundFields},
    skateboard_force_runtime::push::attributes::PushAttributes,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum GroundStateFillRuntimeError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    UnsupportedState(CurrentPhysicalState),
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for GroundStateFillRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Ground.Fill missing {field}"),
            Self::Poisoned => f.write_str("Ground.Fill owner lock poisoned"),
            Self::ForeignOwner => f.write_str("Ground.Fill state belongs to another pair"),
            Self::UnsupportedState(state) => {
                write!(f, "Ground.Fill unsupported current state {state:?}")
            }
            Self::Attribute(error) => write!(f, "Ground.Fill attribute: {error}"),
        }
    }
}
struct Source<'a> {
    fields: &'a PhysicsGroundFields,
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    attributes: Option<&'a PushAttributes>,
}
impl GroundStateFillSource for Source<'_> {
    type Error = GroundStateFillRuntimeError;
    type Pointer = Option<AttributeRuntimeData>;
    fn flag(&mut self, flag: GroundFillFlag) -> Result<bool, Self::Error> {
        Ok(match flag {
            GroundFillFlag::PushAccelerating => self.fields.is_push_accelerating,
            GroundFillFlag::Pinning => self.fields.is_pinning,
            GroundFillFlag::WallRideExit => self.fields.exiting_wallride,
            GroundFillFlag::AntiFlipNudge => self.fields.anti_flip_nudge,
            GroundFillFlag::GrabObject => self.fields.grab_object,
            GroundFillFlag::WorldGrab => {
                self.processed
                    .lock()
                    .map_err(|_| Self::Error::Poisoned)?
                    .physics_state_fields()
                    .flags_8cc
                    .ok_or(Self::Error::Missing("processed flags8cc"))?
                    & 0x0100_0000
                    != 0
            }
        })
    }
    fn scalar(&mut self, scalar: GroundFillScalar) -> Result<f32, Self::Error> {
        Ok(match scalar {
            GroundFillScalar::TimeToSkitch => self.fields.time_to_skitch,
            GroundFillScalar::SkitchHeight => self.fields.skitch_spline_height,
            GroundFillScalar::AbsoluteSpeed => self
                .processed
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .toolkit_state()
                .abs_body_speed()
                .ok_or(Self::Error::Missing("processed abs speed948"))?,
        })
    }
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.attributes
            .ok_or(Self::Error::Missing("global physics_push.default+b0"))?
            .capture(key)
            .map_err(Self::Error::Attribute)
    }
    fn load(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        PushAttributes::load(pointer.as_ref()).map_err(Self::Error::Attribute)
    }
    fn anti_flip_torque(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let value = self
            .fields
            .anti_flip_torque
            .ok_or(Self::Error::Missing("Ground antiFlipTorque90"))?;
        Ok(value)
    }
    fn grab_type(&mut self) -> Result<i32, Self::Error> {
        Ok(self.fields.grab_spline_type)
    }
    fn grab_object(&mut self) -> Result<u32, Self::Error> {
        Ok(self.fields.grab_spline_object)
    }
}

pub(crate) fn try_fill_current_ground_state(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    attributes: Option<&PushAttributes>,
) -> Result<(), GroundStateFillError<GroundStateFillRuntimeError>> {
    use GroundStateFillError as E;
    use GroundStateFillRuntimeError as R;
    let state = owner
        .physics_state()
        .lock()
        .map_err(|_| E::Source(R::Poisoned))?;
    if state.current != CurrentPhysicalState::Ground {
        return Err(E::Source(R::UnsupportedState(state.current)));
    }
    let ground = state
        .ground
        .as_ref()
        .ok_or(E::Source(R::Missing("actual Ground instance")))?;
    if !ground.bindings.board.same_as(board)
        || !ground.bindings.skeleton.same_as(skeleton)
        || !ground
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
    {
        return Err(E::Source(R::ForeignOwner));
    }
    let processed = Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Source(R::Poisoned))?
            .processed(),
    );
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| E::Source(R::Poisoned))?;
    output.try_fill_ground_state(&mut Source {
        fields: &ground.fields,
        processed: &processed,
        attributes,
    })
}
