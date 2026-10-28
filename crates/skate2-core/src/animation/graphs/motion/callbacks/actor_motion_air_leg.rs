//! Actual shared Actor/animation/PhysOut adapter for `ControlAirLegExtension`.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::reckoning_numeric::ReckoningVector;
use crate::{
    actor_construction::{
        action_runtime_session::ActionRuntimeServices, animation_mount::MountedActorAnimation,
    },
    generated_motion_load::construct::LegExtensionBone,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::PhysOutSystemVector,
    play_animation::FastString30,
    reckoning_numeric::host_dot3,
};
use std::sync::{Arc, Mutex};
#[path = "motion_air_leg.rs"]
pub(crate) mod kernel;
use kernel::{AirLegInstance, Cvar, FullValue, Resolve, Source, Tuning, Value};
struct Live<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    services: Option<&'a ActionRuntimeServices>,
    cvars: Option<&'a InputCvarReferences>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    drive: FastString30,
    bone: LegExtensionBone,
    dt: f32,
}
impl Live<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, E> {
        self.physical
            .as_ref()
            .ok_or(E::MissingOwner("Air leg IPhysical"))
    }
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, E> {
        self.animation
            .as_ref()
            .ok_or(E::MissingOwner("Air leg IAnimatable"))
    }
    fn services(&self) -> Result<&ActionRuntimeServices, E> {
        self.services
            .ok_or(E::MissingOwner("Air leg global services"))
    }
    fn cvars(&self) -> Result<&InputCvarReferences, E> {
        self.cvars.ok_or(E::MissingOwner("Air leg cvars"))
    }
    fn attribute(&self, name: FastString30) -> Result<Option<f32>, E> {
        self.actor
            .motion_runtime_tree_attribute(self.context, name)?
            .map(|a| {
                a.scalar_value().ok_or(E::PendingAnimationOperation(
                    "Air leg scalar attribute type",
                ))
            })
            .transpose()
    }
    fn geometric_distance(&self, right: bool) -> Result<f32, E> {
        let physical = self.physical()?;
        let bone = {
            let output = physical.phys_out().lock().map_err(|_| E::Lock)?;
            let skeleton = output.try_skeleton_output().map_err(E::PhysOut)?;
            if right {
                skeleton.skeleton_foot_right_bone_pos
            } else {
                skeleton.skeleton_foot_left_bone_pos
            }
        };
        let com = physical
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_landing_system_vector(PhysOutSystemVector::CentreOfMass)
            .map_err(E::PhysOut)?;
        let up = physical
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_landing_system_vector(PhysOutSystemVector::Up)
            .map_err(E::PhysOut)?;
        Ok(host_dot3(
            std::array::from_fn(|i| com.operand()[i] - bone.operand()[i]),
            up.operand(),
        ))
    }
    fn write(&self, name: FastString30, value: f32) -> Result<(), E> {
        self.animation()?
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(E::AnimationWrite)
    }
}
impl Source for Live<'_> {
    type Error = E;
    fn resolve(&mut self, stage: Resolve) -> Result<(), E> {
        self.actor.check_motion_runtime_context(self.context)?;
        if stage != Resolve::Update && stage != Resolve::Preland {
            self.physical = Some(Arc::clone(
                self.actor
                    .cached_physical_allocation()
                    .map_err(E::Physical)?
                    .owner(),
            ));
        }
        self.animation = Some(self.actor.lifecycle_animation(0)?);
        if stage == Resolve::Update {
            self.physical = Some(Arc::clone(
                self.actor
                    .cached_physical_allocation()
                    .map_err(E::Physical)?
                    .owner(),
            ));
        }
        if matches!(stage, Resolve::Update | Resolve::Down | Resolve::Prepare) {
            let actor = self.actor.prefix().map_err(E::Actor)?;
            let graph = actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .filter(|_| actor.motion_graph_published)
                .ok_or(E::MissingOwner("Air leg ISkaterMotionGraph"))?;
            if !actor.graph_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, graph.occurrence()))
            {
                return Err(E::ForeignOwner("Air leg ISkaterMotionGraph"));
            }
        }
        Ok(())
    }
    fn value(&mut self, field: Value) -> Result<f32, E> {
        let out = self.physical()?.phys_out().lock().map_err(|_| E::Lock)?;
        Ok(match field {
            Value::VelocityY => out
                .try_landing_system_vector(PhysOutSystemVector::CentreOfMassVelocity)
                .map_err(E::PhysOut)?
                .operand()[1],
            Value::ComToDeck => {
                out.try_animation_output()
                    .map_err(E::PhysOut)?
                    .centre_of_mass_to_deck
            }
            Value::CollisionTime => {
                out.try_air_output()
                    .map_err(E::PhysOut)?
                    .traj_collision_time
            }
        })
    }
    fn drive_attribute(&mut self) -> Result<Option<f32>, E> {
        self.attribute(self.drive)
    }
    fn initial_distance(&mut self, data: &mut AirLegInstance) -> Result<(), E> {
        let can_land = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .can_land_on_skateboard;
        if can_land {
            data.distance = self.geometric_distance(true)?;
        } else {
            data.distance = self.value(Value::ComToDeck)?;
            match self.bone {
                LegExtensionBone::SkateboardRoot => {}
                LegExtensionBone::RightToeBase => data.distance = self.geometric_distance(true)?,
                LegExtensionBone::LeftToeBase => data.distance = self.geometric_distance(false)?,
            }
        }
        Ok(())
    }
    fn tuning(&mut self, field: Tuning) -> Result<f32, E> {
        let attrs = &self.services()?.prepare_land;
        attrs.check_vault(&self.physical()?.inputs().vault)?;
        attrs.air_leg_scalar(field)
    }
    fn cvar(&mut self, field: Cvar) -> Result<bool, E> {
        self.cvars()?.try_air_leg_boolean(field).map_err(E::Cvar)
    }
    fn delta_time(&mut self) -> Result<f32, E> {
        Ok(self.dt)
    }
    fn downward_projection(&mut self) -> Result<(f32, f32), E> {
        let p = self.physical()?;
        let velocity = p
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_landing_system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .map_err(E::PhysOut)?;
        let up = p
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_landing_system_vector(PhysOutSystemVector::Up)
            .map_err(E::PhysOut)?;
        Ok((
            f32::from_bits(host_dot3(velocity.operand(), up.operand()).to_bits() ^ 0x8000_0000),
            f32::from_bits(host_dot3(up.operand(), [0.0, 1.0, 0.0, 0.0]).to_bits() & 0x7fff_ffff),
        ))
    }
    fn prepare_land(&mut self) -> Result<bool, E> {
        self.actor.motion_time_to_prepare_land(
            self.context,
            super::prepare_land::PrepareLandRuntime {
                attributes: self.services.map(|s| &s.prepare_land),
                body_tilt: self.services.map(|s| &s.body_tilt),
                cvars: self.cvars,
            },
        )
    }
    fn full_extension(&mut self) -> Result<Option<FullValue>, E> {
        use skate2_formats::andale::AndaleAttributeValue as A;
        Ok(self
            .actor
            .motion_runtime_tree_attribute(
                self.context,
                FastString30::encode("FullExtension").expect("source name"),
            )?
            .map(|a| match a.value {
                A::Float(value) => FullValue::Known(value),
                A::Vector(value) => FullValue::Known(value[0]),
                A::Bytes24(bytes) => FullValue::Known(f32::from_be_bytes(
                    bytes[..4].try_into().expect("24 byte payload"),
                )),
                A::Unsupported { bytes, .. } => {
                    bytes.get(..4).map_or(FullValue::Unavailable, |word| {
                        FullValue::Known(f32::from_be_bytes(word.try_into().expect("four bytes")))
                    })
                }
            }))
    }
    fn custom_earlier(&mut self) -> Result<f32, E> {
        self.cvars()?.try_air_leg_custom_earlier().map_err(E::Cvar)
    }
    fn offboard_bone(&self) -> bool {
        self.bone != LegExtensionBone::SkateboardRoot
    }
    fn write_drive(&mut self, value: f32) -> Result<(), E> {
        self.write(self.drive, value)
    }
    fn extend_graph(&mut self, time: f32) -> Result<f32, E> {
        let attrs = &self.services()?.prepare_land;
        attrs.check_vault(&self.physical()?.inputs().vault)?;
        attrs.air_leg_extend(time)
    }
    fn write_extend(&mut self, value: f32) -> Result<(), E> {
        self.write(FastString30::encode("extend").expect("source name"), value)
    }
    fn require_overlay(&mut self) -> Result<(), E> {
        self.services()?
            .overlay
            .require()
            .map_err(E::AirLegOverlay)?;
        Ok(())
    }
    fn missing_attribute_word(&self) -> E {
        E::MissingOwner("FullExtension payload word")
    }
    fn missing_instance(&self) -> E {
        E::MissingOwner("Air leg instance")
    }
}
impl ActorConstruction {
    pub(crate) fn motion_air_leg_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: Option<&mut AirLegInstance>,
        drive: FastString30,
        bone: LegExtensionBone,
        dt: f32,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<(), E> {
        let services = self.action_runtime_services().map_err(E::Actor)?;
        let mut data = data;
        let result = kernel::update(
            data.as_deref_mut(),
            &mut Live {
                actor: self,
                context,
                services: services.as_deref(),
                cvars,
                physical: None,
                animation: None,
                drive,
                bone,
                dt,
            },
        );
        if std::env::var_os("SKATE2_TRACE_OFFBOARD_AIR").is_some()
            && let Some(data) = data.as_deref()
        {
            eprintln!(
                "offboard-air leg: bone={bone:?} phase={} distance={} down_time={}",
                data.phase, data.distance, data.down_time
            );
        }
        result
    }
}
