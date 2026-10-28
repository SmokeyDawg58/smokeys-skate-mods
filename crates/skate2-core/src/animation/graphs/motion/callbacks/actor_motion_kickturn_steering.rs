//! Retained Actor owners for `KickTurnSteering` Begin `0x82c95428` and
//! Update `0x82cc5860`. The graph instance is GlobalAttributeCollection+0x144.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
    play_animation::PlayAnimationActor,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::MotionLoadError,
    input_cvar_runtime::InputCvarReferences,
    motion_kickturn_steering::{
        self as kernel, KickTurnAttribute, KickTurnGraph, KickTurnSteeringSource,
    },
    physics::PointGraph,
    play_animation::{FastString30, FastString36},
    play_animation_lifecycle::{
        PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution, PlayAnimationLifecycleBackend,
    },
};
use std::sync::{Arc, Mutex};

/// The native `anim_motion_kickturn` wrapper allocated at `0x828b2edc` and
/// stored in GlobalAttributeCollection+0x144 at `0x828b2f14`.
pub(crate) struct KickTurnMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl KickTurnMotionAttributes {
    /// The generated `anim_motion` constructor `0x828b1590` installs a
    /// 0x5c0-byte default layout when the collection has no authored layout.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        // 828b2ef8..2f04: r4 holds the low word; insrdi inserts r9 high.
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xa168_8979_1975_2875);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }

    pub(crate) fn check_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(Error::ForeignOwner("Global anim_motion_kickturn Vault"))
        }
    }

    /// PDB `anim_motion::_LayoutStruct`: eight-point `kickturn_spin` at +0xa0
    /// and `kickturn_balance` at +0xf0, each with X+0x10 and Y+0x30.
    pub(crate) fn graph(&self, graph: KickTurnGraph, input: f32) -> Result<f32, Error> {
        let base = match graph {
            KickTurnGraph::Balance => 0xf0,
            KickTurnGraph::Spin => 0xa0,
        };
        let layout = self
            .instance
            .try_cached_layout(0x5c0)
            .map_err(attribute_error)?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("validated anim_motion layout"),
                )
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(base + 0x10 + i * 4)),
            y: std::array::from_fn(|i| word(base + 0x30 + i * 4)),
        }
        .evaluate(input))
    }
}

fn attribute_error(error: AttributeRuntimeError) -> Error {
    Error::Load(MotionLoadError::AttributeRuntime(error))
}

struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a KickTurnMotionAttributes>,
    cvars: Option<&'a InputCvarReferences>,
    dt: f32,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("KickTurnSteering captured IAnimatable"))
    }
    fn cvars(&self) -> Result<&InputCvarReferences, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("KickTurnSteering live cvars"))
    }
    fn attributes(&self) -> Result<&KickTurnMotionAttributes, Error> {
        let attributes = self
            .attributes
            .ok_or(Error::MissingOwner("Global anim_motion_kickturn+144"))?;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(Error::Physical)?;
        attributes.check_vault(&physical.owner().inputs().vault)?;
        Ok(attributes)
    }
    fn with_motion_state<T>(
        &self,
        apply: impl FnOnce(
            &mut crate::actor_graph_construction::ActorSkaterGraphState,
        ) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .ok_or(Error::MissingOwner("KickTurnSteering captured Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|captured| Arc::ptr_eq(captured, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("KickTurnSteering captured Motion"));
        }
        apply(
            motion
                .state
                .as_mut()
                .ok_or(Error::MissingOwner("Motion state"))?,
        )
    }
}
impl KickTurnSteeringSource for Source<'_> {
    type Error = Error;

    fn resolve_interfaces(&mut self) -> Result<(), Error> {
        // Native resolves IAnimatable, then IMotionGraph, then
        // ISkaterMotionGraph. PlayAnimationActor checks the first interface's
        // entity issuance and UCOM registration against the same Actor.
        let mut animation_source = PlayAnimationActor {
            actor: self.actor,
            context: self.context,
        };
        if animation_source.resolve_interface(PlayAnimationInterfaceKind::Animatable)?
            != PlayAnimationInterfaceResolution::Resolved
        {
            return Err(Error::ForeignOwner("KickTurnSteering IAnimatable"));
        }
        let animation = animation_source.animation()?;
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner("KickTurnSteering published Motion"))?;
        for (index, name) in [
            (1, "KickTurnSteering IMotionGraph"),
            (2, "KickTurnSteering ISkaterMotionGraph"),
        ] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
            {
                return Err(Error::ForeignOwner(name));
            }
        }
        self.animation = Some(animation);
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }

    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("KickTurnSteering instance")
    }

    fn set_animation_attributes(&mut self) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skater_set_attributes(self.cvars()?)
            .map_err(Error::AnimationPhase)
    }

    fn base_animation_length(&mut self) -> Result<f32, Error> {
        let animation = self.animation()?.lock().map_err(|_| Error::Lock)?;
        let base = animation
            .base()
            .ok_or(Error::MissingOwner("KickTurnSteering Animatable"))?;
        let root = base
            .body_root
            .as_ref()
            .ok_or(Error::MissingOwner("KickTurnSteering base animation"))?;
        animation
            .body_reference_clock(root, false)
            .map_err(Error::AnimationBody)
    }

    fn invalid_animation_length(&mut self, _length: f32) -> Error {
        // MotionGraphBehaviours.cpp line 2501 asserts the just-stored length.
        Error::MissingOwner("KickTurnSteering positive base animation length")
    }

    fn delta_time(&mut self) -> Result<f32, Error> {
        if self.context.controller().is_none() {
            return Err(Error::MissingOwner("KickTurnSteering controller"));
        }
        Ok(self.dt)
    }

    fn graph(&mut self, graph: KickTurnGraph, x: f32) -> Result<f32, Error> {
        self.attributes()?.graph(graph, x)
    }

    fn smoothing(&mut self) -> Result<f32, Error> {
        self.cvars()?.try_kickturn_smoothing().map_err(Error::Cvar)
    }

    fn kickturn_intent(&mut self) -> Result<Option<f32>, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .ok_or(Error::MissingOwner(
                "KickTurnSteering captured IMotionGraph",
            ))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|captured| Arc::ptr_eq(captured, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("KickTurnSteering captured intents"));
        }
        let intents = motion
            .intents
            .as_ref()
            .ok_or(Error::MissingOwner("Motion intents"))?;
        Ok(intents.get(&FastString36::encode("KickTurn").expect("source intent name")))
    }

    fn use_fine_kickturn(&mut self) -> Result<bool, Error> {
        self.cvars()?.try_use_fine_kickturn().map_err(Error::Cvar)
    }

    fn add_motion_attribute(
        &mut self,
        attribute: KickTurnAttribute,
        value: f32,
    ) -> Result<(), Error> {
        let name = match attribute {
            // BalanceAtt/SpinAtt are C++ global symbols; their FastString
            // initializers store these literals for AddMgAttribute.
            KickTurnAttribute::Balance => "Balance",
            KickTurnAttribute::Spin => "Spin",
        };
        let name = FastString30::encode(name).expect("source motion attribute name");
        self.with_motion_state(|state| state.try_append_motion_attribute(name, value))
    }
}

impl ActorConstruction {
    pub(crate) fn motion_runtime_kickturn_steering_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::KickTurnSteeringInstance>,
        attributes: Option<&KickTurnMotionAttributes>,
    ) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        kernel::begin(
            &mut Source {
                actor: self,
                context,
                attributes,
                cvars: None,
                dt: 0.0,
                animation: None,
                motion: None,
            },
            instance,
        )
    }

    pub(crate) fn motion_runtime_kickturn_steering_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::KickTurnSteeringInstance>,
        max_height: f32,
        spin_scale: f32,
        attributes: Option<&KickTurnMotionAttributes>,
        cvars: Option<&InputCvarReferences>,
        dt: f32,
    ) -> Result<(), Error> {
        kernel::update(
            &mut Source {
                actor: self,
                context,
                attributes,
                cvars,
                dt,
                animation: None,
                motion: None,
            },
            instance,
            max_height,
            spin_scale,
        )
    }
}
