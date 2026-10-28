//! Same Skeleton constructor-owned objects retained across later Body creation.
//! Completion here covers selected children, not whole Skeleton ctor/Init.
use crate::{
    attribute_runtime::AttributeRuntimeVault, skeleton_current_animation::SkeletonCurrentAnimation,
    skeleton_jump_adjustments::SkeletonJumpAdjustments,
};
use crate::{skeleton_head_tracking as head_tracking, skeleton_skateboard_wobble as wobble};
use std::sync::{Arc, Mutex, Weak};

#[path = "skeleton_init_state.rs"]
pub(crate) mod init_state;

const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;

/// Native pointers refer to the actual enclosing physical owner, its SAME
/// Processed storage and selected simulation island. No second collaborators.
pub(crate) struct SkeletonConstructorBindings {
    pub(crate) owner: Weak<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    pub(crate) processed: Arc<Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>>,
    pub(crate) island: Option<
        crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    >,
    pub(crate) actor_query:
        Option<Arc<crate::actor_construction::physical_mount::ActorPhysicalQueryBinding>>,
    /// Constructor stores NULL at82b56e50. Actor animation mount owns the later
    /// actual reference; this slot cannot be treated as a constructed bridge.
    pub(crate) animation_input:
        Option<Arc<Mutex<crate::actor_construction::anim_out_phys_in::AnimOutPhysInOwner>>>,
    pub(crate) hat_type: Option<u32>,
    pub(crate) bouncy_ragdoll: Option<u8>,
}

/// Remaining outer members without an existing mutable producer projection.
/// The root/current/general/post owners below hold the other matrices/vectors.
#[expect(
    dead_code,
    reason = "constructor-owned trajectory, error, velocity, debug and wobble fields await their original ProcessData and post-physics producers"
)]
pub(crate) struct SkeletonOuterRemainder {
    pub(crate) time_delta: f32,
    pub(crate) trajectory_transform: crate::animation::AnimationMatrix,
    pub(crate) trajectory_inverse: crate::animation::AnimationMatrix,
    pub(crate) com_to_deck_world: [f32; 4],
    pub(crate) com_to_deck_world_delta: [f32; 4],
    pub(crate) skateboard_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) centre_of_mass_local: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) skateboard_error: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) deck_local: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) hips_offset: Option<[f32; 4]>,
    pub(crate) trajectory_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    /// No constructor store for native3dc0 or3e6c; never invent defaults.
    pub(crate) com_orientation: Option<[f32; 4]>,
    pub(crate) print_bone_cursor: Option<i32>,
    pub(crate) deck_wobble_tilt: f32,
    pub(crate) deck_wobble_squish: f32,
    pub(crate) need_air_collision: Option<bool>,
    pub(crate) teleport_reset: Option<bool>,
    pub(crate) physics_animation_state: Option<i32>,
    pub(crate) use_physics_truck_tilt: Option<bool>,
}
impl SkeletonOuterRemainder {
    fn before_children() -> Self {
        let identity = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        Self {
            time_delta: 0.0,
            trajectory_transform: identity,
            trajectory_inverse: identity,
            com_to_deck_world: [0.0; 4],
            com_to_deck_world_delta: [0.0; 4],
            skateboard_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            centre_of_mass_local: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            skateboard_error: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            deck_local: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            hips_offset: Some([0.0; 4]),
            trajectory_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            com_orientation: None,
            print_bone_cursor: None,
            deck_wobble_tilt: 0.0,
            deck_wobble_squish: 0.0,
            need_air_collision: None,
            teleport_reset: None,
            physics_animation_state: Some(0),
            use_physics_truck_tilt: None,
        }
    }
}

/// The three distinct outer instances precede `SkeletonState` (82b56de0..e40).
/// A None layout is the generated wrapper's shared zero fallback; animation
/// is a plain Instance and does not install a generated-layout fallback.
pub(crate) struct SkeletonOuterAttributes {
    pub(crate) animation: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) physics_animation: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) physics_animation_layout: Option<crate::attribute_runtime::AttributeRuntimeData>,
    pub(crate) physics_skeleton: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) physics_skeleton_layout: Option<crate::attribute_runtime::AttributeRuntimeData>,
    physics_animation_transferred: bool,
}

pub(crate) struct SkeletonStateRemainder {
    #[expect(
        dead_code,
        reason = "actual State Instance retained until source State force/update attribute consumers mount"
    )]
    pub(crate) attributes: crate::attribute_runtime::AttributeRuntimeInstance,
    #[expect(
        dead_code,
        reason = "actual State cached layout retained for those same pending native consumers"
    )]
    pub(crate) layout: Option<crate::attribute_runtime::AttributeRuntimeData>,
    pub(crate) history: Option<crate::skeleton_physics_history::SkeletonPhysicsHistory>,
    pub(crate) linear_error: Option<[[f32; 4]; 24]>,
    pub(crate) proxy_target: crate::skeleton_root_transforms::SkeletonRootVector,
}

pub(crate) struct SkeletonConstructorRuntime {
    pub(crate) bindings: Option<SkeletonConstructorBindings>,
    pub(crate) outer: Option<SkeletonOuterRemainder>,
    pub(crate) root_transforms: Option<crate::skeleton_root_transforms::SkeletonRootTransforms>,
    pub(crate) general_update: Option<crate::skeleton_general_update::SkeletonGeneralUpdate>,
    pub(crate) post_physics_records:
        Option<crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords>,
    pub(crate) outer_attributes: Option<SkeletonOuterAttributes>,
    pub(crate) init: init_state::SkeletonInitState,
    pub(crate) state: Option<SkeletonStateRemainder>,
    pub(crate) head: Option<head_tracking::SkeletonHeadTracking>,
    pub(crate) wobble: Option<wobble::SkeletonSkateboardWobble>,
    pub(crate) current_animation: Option<SkeletonCurrentAnimation>,
    pub(crate) jump_adjustments: Option<SkeletonJumpAdjustments>,
    pub(crate) children_complete: bool,
    attempted: bool,
}
impl SkeletonConstructorRuntime {
    /// Transfer SAME plain Instance+4 to the continuing Skeleton query owner.
    pub(crate) fn take_animation_instance(
        &mut self,
    ) -> Option<crate::attribute_runtime::AttributeRuntimeInstance> {
        self.outer_attributes.as_mut()?.animation.take()
    }
    /// Move the actual outer+14 Instance into the continuing animation owner;
    /// a later animation activation must not bind a replacement instance.
    pub(crate) fn take_physics_animation_instance(
        &mut self,
    ) -> Option<crate::attribute_runtime::AttributeRuntimeInstance> {
        let attributes = self.outer_attributes.as_mut()?;
        let instance = attributes.physics_animation.take()?;
        attributes.physics_animation_transferred = true;
        Some(instance)
    }
    /// Host constructor/transfer proof only. Caller additionally verifies the
    /// actual owner/Processed/query identity, completed Set/history and Init
    /// suffix before publishing the Actor cached physical pointer.
    pub(crate) fn ready_after_transfer(&self) -> bool {
        self.children_complete
            && self.init.complete()
            && self.outer_attributes.as_ref().is_some_and(|attributes| {
                attributes.physics_animation_transferred
                    && attributes.animation.is_none()
                    && attributes.physics_animation.is_none()
                    && attributes.physics_skeleton.is_some()
            })
            && self.bindings.as_ref().is_some_and(|bindings| {
                bindings.owner.upgrade().is_some()
                    && bindings.island.is_some()
                    && bindings.hat_type.is_some()
                    && bindings.bouncy_ragdoll.is_some()
            })
            && self.outer.as_ref().is_some_and(|outer| {
                outer.hips_offset.is_none()
                    && outer.physics_animation_state.is_none()
                    && outer.use_physics_truck_tilt.is_none()
                    && outer.need_air_collision == Some(false)
                    && outer.teleport_reset == Some(false)
            })
            && self
                .state
                .as_ref()
                .is_some_and(|state| state.history.is_none() && state.linear_error.is_none())
            && self.head.is_some()
            && self.wobble.is_some()
            && self.current_animation.is_none()
            && self.jump_adjustments.is_none()
            && self.root_transforms.is_none()
            && self.general_update.is_none()
            && self.post_physics_records.is_none()
    }

    pub(crate) fn new() -> Self {
        Self {
            bindings: None,
            outer: None,
            root_transforms: None,
            general_update: None,
            post_physics_records: None,
            outer_attributes: None,
            init: init_state::SkeletonInitState::new(),
            state: None,
            head: None,
            wobble: None,
            current_animation: None,
            jump_adjustments: Some(SkeletonJumpAdjustments::uninitialized()),
            children_complete: false,
            attempted: false,
        }
    }
    pub(crate) fn try_begin(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        current_entity_flags: &mut impl FnMut() -> u32,
    ) -> Result<(), SkeletonConstructorRuntimeError> {
        if self.attempted {
            return Err(SkeletonConstructorRuntimeError::AlreadyAttempted);
        }
        self.attempted = true;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| SkeletonConstructorRuntimeError::OwnerPoisoned)?
                .processed(),
        );
        self.bindings = Some(SkeletonConstructorBindings {
            owner: Arc::downgrade(owner),
            processed,
            island: None,
            actor_query: None,
            animation_input: None,
            hat_type: None,
            bouncy_ragdoll: None,
        });
        let animation = vault.instance(0xbabb_3ec1_1084_9e08, DEFAULT);
        self.outer_attributes = Some(SkeletonOuterAttributes {
            animation: Some(animation),
            physics_animation: None,
            physics_animation_layout: None,
            physics_skeleton: None,
            physics_skeleton_layout: None,
            physics_animation_transferred: false,
        });
        let outer = self
            .outer_attributes
            .as_mut()
            .expect("retained animation Instance");
        outer.physics_animation = Some(vault.instance(0x74bc_9c93_affe_6aac, DEFAULT));
        outer.physics_animation_layout = outer
            .physics_animation
            .as_ref()
            .expect("retained physics_animation Instance")
            .try_cached_layout(0x220)
            .map_err(SkeletonConstructorRuntimeError::Attributes)?;
        outer.physics_skeleton = Some(vault.instance(0x9f86_2c2e_aa01_6ce5, DEFAULT));
        outer.physics_skeleton_layout = outer
            .physics_skeleton
            .as_ref()
            .expect("retained physics_skeleton Instance")
            .try_cached_layout(0x70)
            .map_err(SkeletonConstructorRuntimeError::Attributes)?;
        let crate::skeleton_state_constructor::SkeletonStateConstructorParts {
            attributes,
            layout,
            animation_record,
            history,
            linear_error,
            proxy_target,
        } = crate::skeleton_state_constructor::SkeletonStateConstructorParts::try_construct(vault)
            .map_err(SkeletonConstructorRuntimeError::State)?;
        self.state = Some(SkeletonStateRemainder {
            attributes,
            layout,
            history: Some(history),
            linear_error: Some(linear_error),
            proxy_target,
        });
        // Native82b56e64 GetPhysicsIsland performs its own current Entity flag
        // read, separate from Body construction's later island lookup.
        let flags = current_entity_flags();
        if flags & 1 == 0 {
            return Err(SkeletonConstructorRuntimeError::VisualIslandUnsupported { flags });
        }
        self.bindings
            .as_mut()
            .expect("early actual bindings")
            .island = Some(owner.manager());
        self.outer = Some(SkeletonOuterRemainder::before_children());
        self.root_transforms = Some(crate::skeleton_root_transforms::SkeletonRootTransforms::new());
        self.general_update = Some(crate::skeleton_general_update::SkeletonGeneralUpdate::new());
        self.post_physics_records =
            Some(crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords::new());
        self.current_animation =
            Some(SkeletonCurrentAnimation::before_skeleton_data_init_with_record(animation_record));
        self.head = Some(head_tracking::SkeletonHeadTracking::before_constructor(
            vault,
        ));
        self.head
            .as_mut()
            .expect("retained HeadTracking allocation")
            .try_construct()
            .map_err(SkeletonConstructorRuntimeError::Head)?;
        Ok(())
    }
    /// Caller completes actual IK geometry first, at the original5743c stage.
    pub(crate) fn try_finish_children(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), SkeletonConstructorRuntimeError> {
        if self.wobble.is_some() {
            return Err(SkeletonConstructorRuntimeError::AlreadyAttempted);
        }
        self.wobble = Some(wobble::SkeletonSkateboardWobble::before_constructor(vault));
        self.wobble
            .as_mut()
            .expect("retained SkateboardWobble allocation")
            .try_construct()
            .map_err(SkeletonConstructorRuntimeError::Wobble)?;
        self.jump_adjustments
            .as_mut()
            .expect("same early adjustment owner")
            .initialize_constructor()
            .map_err(SkeletonConstructorRuntimeError::Jump)?;
        let outer = self.outer.as_mut().expect("actual earlier outer stores");
        outer.need_air_collision = Some(false);
        // Native big-endian word1 at3e40 writes false, then truck-tilt true.
        outer.teleport_reset = Some(false);
        outer.use_physics_truck_tilt = Some(true);
        let bindings = self
            .bindings
            .as_mut()
            .expect("actual earlier pointer stores");
        let owner = bindings
            .owner
            .upgrade()
            .ok_or(SkeletonConstructorRuntimeError::OwnerExpired)?;
        bindings.bouncy_ragdoll = Some(owner.inputs().bouncy_ragdoll);
        bindings.actor_query.clone_from(
            &*owner
                .actor_query
                .lock()
                .map_err(|_| SkeletonConstructorRuntimeError::OwnerPoisoned)?,
        );
        bindings.hat_type = Some(owner.inputs().hat_type);
        self.children_complete = true;
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum SkeletonConstructorRuntimeError {
    AlreadyAttempted,
    OwnerPoisoned,
    OwnerExpired,
    VisualIslandUnsupported { flags: u32 },
    Attributes(crate::attribute_runtime::AttributeRuntimeError),
    State(crate::attribute_runtime::AttributeRuntimeError),
    Head(head_tracking::HeadTrackingConstructorError),
    Wobble(wobble::WobbleConstructorError),
    Jump(crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError),
}
impl std::fmt::Display for SkeletonConstructorRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => {
                f.write_str("Skeleton constructor children already attempted")
            }
            Self::OwnerPoisoned => f.write_str("Skeleton actual owner lock poisoned"),
            Self::OwnerExpired => f.write_str("Skeleton actual physical owner expired"),
            Self::VisualIslandUnsupported { flags } => write!(
                f,
                "Skeleton constructor needs unsupported visual island for Entity flags {flags:08x}"
            ),
            Self::Attributes(e) => write!(f, "Skeleton outer attributes: {e}"),
            Self::State(e) => write!(f, "{e}"),
            Self::Head(e) => write!(f, "{e}"),
            Self::Wobble(e) => write!(f, "{e}"),
            Self::Jump(e) => write!(f, "{e}"),
        }
    }
}
