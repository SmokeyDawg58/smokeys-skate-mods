use super::*;

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Foreign,
    Duplicate,
    AfterAdd,
    PlayerQuery,
}
struct Registry {
    entity: MotionGraphEntityAuthority<u32>,
    entries: Vec<(ActorAnimationInterface, Arc<()>)>,
    fail_after: Option<ActorAnimationInterface>,
    fail_query: bool,
    query_count: usize,
}
impl Registry {
    fn new(authority: &ActorGraphAuthority<u32>) -> Self {
        Self {
            entity: authority.entity_handle(),
            entries: Vec::new(),
            fail_after: None,
            fail_query: false,
            query_count: 0,
        }
    }
}
impl ActorAnimationRegistry<u32> for Registry {
    type Error = Failure;
    fn register_animation_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<u32>,
        kind: ActorAnimationInterface,
        occurrence: &Arc<()>,
    ) -> Result<(), Failure> {
        if !self.entity.is_same_issuance(entity) {
            return Err(Failure::Foreign);
        }
        if self.entries.iter().any(|(stored, _)| *stored == kind) {
            return Err(Failure::Duplicate);
        }
        self.entries.push((kind, Arc::clone(occurrence)));
        if self.fail_after == Some(kind) {
            return Err(Failure::AfterAdd);
        }
        Ok(())
    }
    fn actor_player_index(
        &mut self,
        entity: &MotionGraphEntityAuthority<u32>,
    ) -> Result<u32, Failure> {
        if !self.entity.is_same_issuance(entity) {
            return Err(Failure::Foreign);
        }
        self.query_count += 1;
        if self.fail_query {
            return Err(Failure::PlayerQuery);
        }
        Ok(*entity.key())
    }
}
// Actual simulation world is supplied only by root's mounted path. This test
// stores an observable owned fixture, not a claimed native IWorld implementation.
struct WorldFixture {
    marker: u32,
}
fn construction(
    authority: &ActorGraphAuthority<u32>,
    low: bool,
) -> ActorAnimationConstruction<u32, WorldFixture> {
    ActorAnimationConstruction::new(
        authority,
        ActorAnimationInputs {
            physics_world: Arc::new(WorldFixture { marker: 27 }),
            life: AnimationLifetime::Transient,
            use_low_blend_cull_threshold: low,
            face_name: Some(Arc::from(&b"Face\0retained"[..])),
        },
    )
}

#[test]
fn same_actor_and_component_reach_vault_without_publishing_animation() {
    let authority = ActorGraphAuthority::new(3);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, true);
    assert_eq!(anim.phase(), ActorAnimationPhase::Unattempted);
    anim.try_construct_prefix(&mut registry).unwrap();
    assert_eq!(anim.phase(), ActorAnimationPhase::VaultRequired);
    assert_eq!(
        registry
            .entries
            .iter()
            .map(|(kind, _)| kind.source_name())
            .collect::<Vec<_>>(),
        [0x825e_c080, 0x825e_cbb0, 0x824b_0488]
    );
    for (_, occurrence) in &registry.entries {
        assert!(Arc::ptr_eq(occurrence, anim.occurrence()));
    }
    assert!(anim.entity().is_same_issuance(&authority.entity_handle()));
    assert!(Arc::ptr_eq(
        &anim.inputs().physics_world,
        &anim.skater().unwrap().physics_world
    ));
    assert_eq!(anim.skater().unwrap().physics_world.marker, 27);
    assert_eq!(
        anim.inputs().face_name.as_deref(),
        Some(&b"Face\0retained"[..])
    );
    assert_eq!(registry.query_count, 1);
    assert_eq!(anim.player().unwrap().player_index, 3);
    assert_eq!(
        anim.try_construct_prefix(&mut registry),
        Err(ActorAnimationConstructionError::AlreadyAttempted)
    );
    assert_eq!(registry.entries.len(), 3);
    assert_eq!(registry.query_count, 1);
}

#[test]
fn every_failed_interface_keeps_prior_constructed_objects_and_actual_add() {
    let kinds = [
        ActorAnimationInterface::Animatable,
        ActorAnimationInterface::ChannelAnimatable,
        ActorAnimationInterface::SkaterAnim,
    ];
    for (index, kind) in kinds.into_iter().enumerate() {
        let authority = ActorGraphAuthority::new(0);
        let mut registry = Registry::new(&authority);
        registry.fail_after = Some(kind);
        let mut anim = construction(&authority, false);
        assert_eq!(
            anim.try_construct_prefix(&mut registry),
            Err(ActorAnimationConstructionError::Registry(Failure::AfterAdd))
        );
        assert_eq!(registry.entries.len(), index + 1);
        assert_eq!(anim.base().is_some(), index >= 1);
        assert_eq!(anim.channel().is_some(), index >= 2);
        assert!(anim.skater().is_none());
        assert!(anim.player().is_none());
        assert_eq!(registry.query_count, 0);
        assert!(Arc::ptr_eq(&registry.entries[index].1, anim.occurrence()));
        assert_eq!(
            anim.try_construct_prefix(&mut registry),
            Err(ActorAnimationConstructionError::AlreadyAttempted)
        );
    }
}

#[test]
fn failed_late_player_query_keeps_trajectory_world_and_unknown_player_fields() {
    let authority = ActorGraphAuthority::new(8);
    let mut registry = Registry::new(&authority);
    registry.fail_query = true;
    let mut anim = construction(&authority, false);
    assert_eq!(
        anim.try_construct_prefix(&mut registry),
        Err(ActorAnimationConstructionError::Registry(
            Failure::PlayerQuery
        ))
    );
    assert_eq!(anim.phase(), ActorAnimationPhase::QueryingPlayerIndex);
    assert_eq!(registry.entries.len(), 3);
    assert!(anim.base().is_some() && anim.channel().is_some() && anim.skater().is_some());
    assert!(anim.player().is_none());
    assert!(Arc::ptr_eq(
        &anim.inputs().physics_world,
        &anim.skater().unwrap().physics_world
    ));
}

#[test]
fn equal_value_foreign_actor_is_rejected_before_any_base_initialization() {
    let authority = ActorGraphAuthority::new(0);
    let foreign = ActorGraphAuthority::new(0);
    let mut registry = Registry::new(&foreign);
    let mut anim = construction(&authority, false);
    assert_eq!(
        anim.try_construct_prefix(&mut registry),
        Err(ActorAnimationConstructionError::Registry(Failure::Foreign))
    );
    assert!(registry.entries.is_empty());
    assert!(anim.base().is_none());
}

#[test]
fn actual_inline_store_order_and_unknown_pointer_members_survive_construction() {
    let authority = ActorGraphAuthority::new(1);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, false);
    anim.try_construct_prefix(&mut registry).unwrap();
    let base = anim.base().unwrap();
    assert_eq!(
        (
            base.settable_attributes.len(),
            base.settable_attributes.maximum()
        ),
        (0, 150)
    );
    assert_eq!(
        (base.tree_attributes.len(), base.tree_attributes.maximum()),
        (0, 120)
    );
    assert_eq!(
        (base.construction_map.len(), base.construction_map.maximum()),
        (0, 6)
    );
    assert_eq!(base.tree_store.free_order, [0, 1, 2, 3]);
    assert_eq!(base.tree_store.trees, [AnimationPoolStorage::empty(); 4]);
    let store = &base.transition_store;
    assert_eq!(store.free_blends, [0, 1, 2]);
    assert_eq!(store.free_sequences, [0, 1, 2]);
    assert_eq!(
        (store.transition_order.len(), store.blend_order.len()),
        (0, 0)
    );
    for blend in &store.blends {
        assert_eq!(blend.flags_known_zero_mask, 0xe000_0000);
        assert_eq!(blend.to_animation, AnimationPointerState::Unwritten);
        assert_eq!(blend.from_animation, AnimationPointerState::Unwritten);
        assert_eq!(blend.time.to_bits(), 0);
    }
    assert_eq!(base.global_pose, AnimationPointerState::Null);
    assert_eq!(base.global_hierarchy, AnimationPointerState::Unwritten);
    assert_eq!(base.settable_animation, AnimationPointerState::Unwritten);
    assert!(base.trajectory.is_none() && base.max_clips_used.is_none());
    assert!(anim.channel().unwrap().max_channels.is_none());
    assert_eq!(
        base.expired_or_looped_by_time.to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(base.time_to_expire_or_loop.to_bits(), (-1.0_f32).to_bits());
    assert_eq!(base.life.source_value(), 0);
    assert_eq!(AnimationLifetime::Permanent.source_value(), 1);
}

#[test]
fn foot_reset_preserves_source_zero_normal_positive_z_stride_and_invalid_state() {
    let authority = ActorGraphAuthority::new(2);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, true);
    anim.try_construct_prefix(&mut registry).unwrap();
    let skater = anim.skater().unwrap();
    let trajectory = &skater.trajectory;
    for foot in trajectory
        .input_feet
        .iter()
        .chain(&trajectory.physical_feet)
    {
        assert_eq!(
            foot.data.normal.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            foot.stride.known_lanes().unwrap().map(f32::to_bits),
            [0, 0, 1.0_f32.to_bits(), 0]
        );
        assert!(!foot.initialized && !foot.stride_initialized);
        for state in foot.states {
            assert_eq!(state.action, -1);
            assert_eq!(
                state.vectors,
                [crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]); 4]
            );
            assert_eq!(state.debug_flags, 0);
        }
    }
    assert_eq!(trajectory.current_state, -1);
    assert!(trajectory.first_update);
    assert_eq!(trajectory.physical_foot_index, 0);
    assert!(
        skater
            .step_results
            .iter()
            .all(|r| !r.result.valid && !r.ready)
    );
    let player = anim.player().unwrap();
    assert_eq!(player.flags_known_mask, 0xfc60_0000);
    assert_eq!(player.flags_bits, 0x0800_0000);
    assert_eq!(player.gestures, [15, 12, 13, 14]);
    // Native assigns "Default"; FastString's alphabet folds case.
    assert_eq!(player.pro_skater.decode().unwrap(), "default");
}

#[test]
fn obair_begin_end_touch_only_skater_animation_landing_bit() {
    let authority = ActorGraphAuthority::new(2);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, true);
    anim.try_construct_prefix(&mut registry).unwrap();
    let before = anim.player().unwrap().flags_bits;
    let before_known = anim.player().unwrap().flags_known_mask;

    anim.try_set_obair_landing_bit(true).unwrap();
    assert_eq!(anim.player().unwrap().flags_bits, before | 0x8000);
    assert_eq!(
        anim.player().unwrap().flags_known_mask,
        before_known | 0x8000
    );

    anim.try_set_obair_landing_bit(false).unwrap();
    assert_eq!(anim.player().unwrap().flags_bits, before & !0x8000);
    assert_eq!(
        anim.player().unwrap().flags_known_mask,
        before_known | 0x8000
    );
}

struct SprintCvar {
    result: Result<u8, Failure>,
    reads: usize,
}
impl ActorAnimationCvars for SprintCvar {
    type Error = Failure;
    fn use_sprint_button(&mut self) -> Result<u8, Failure> {
        self.reads += 1;
        self.result
            .as_ref()
            .copied()
            .map_err(|_| Failure::PlayerQuery)
    }
}
fn offboard_vault(present: bool) -> Arc<AttributeRuntimeVault> {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    let mut vault = graph(ACTOR_ANIMATION_OFFBOARD_CLASS, vec![definition(7, 4, 4)]);
    if present {
        collection(
            &mut vault,
            ACTOR_ANIMATION_OFFBOARD_CLASS,
            ACTOR_ANIMATION_OFFBOARD_COLLECTION,
            0,
            None,
            vec![node(7, 0x40, 19)],
        );
    }
    Arc::new(vault)
}

#[test]
fn vault_suffix_binds_real_instance_and_samples_sprint_byte_once() {
    for byte in [0, 1, 255] {
        let authority = ActorGraphAuthority::new(0);
        let mut registry = Registry::new(&authority);
        let mut anim = construction(&authority, true);
        anim.try_construct_prefix(&mut registry).unwrap();
        let vault = offboard_vault(true);
        let owner_count = Arc::strong_count(&vault);
        let mut cvar = SprintCvar {
            result: Ok(byte),
            reads: 0,
        };
        anim.try_construct_vault_suffix(Arc::clone(&vault), &mut cvar)
            .unwrap();
        // Offboard and Global locomotion instances retain the same real Vault.
        assert_eq!(Arc::strong_count(&vault), owner_count + 2);
        assert_eq!(cvar.reads, 1);
        assert_eq!(anim.phase(), ActorAnimationPhase::FaceSetupRequired);
        assert_eq!(
            anim.offboard_attributes()
                .unwrap()
                .try_scalar_word(7, 0)
                .unwrap(),
            Some(19)
        );
        let tail = anim.before_face().unwrap();
        assert_eq!(
            tail.speed_clamp.to_bits(),
            if byte == 0 { 0x3f80_0000 } else { 0x3f4c_cccd }
        );
        assert_eq!(
            tail.world_position
                .columns()
                .map(|v| v.known_lanes().unwrap()),
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0; 4]
            ]
        );
        assert_eq!(
            tail.extra_y_rotation,
            tail.world_position
                .columns()
                .map(|v| v.known_lanes().unwrap())
        );
        assert_eq!(
            tail.applied_extra_y_rotation,
            tail.world_position
                .columns()
                .map(|v| v.known_lanes().unwrap())
        );
        assert!(tail.collision_response_turn.is_none());
        assert_eq!(
            tail.last_collision_normal
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        let player = anim.player().unwrap();
        assert_eq!(player.flags_known_mask, u32::MAX);
        assert_eq!(player.flags_bits, 0x0804_0010);
        assert_eq!(anim.before_sprint().unwrap().second_flags_bits, 0x8000_0000);
        assert_eq!(
            anim.try_construct_vault_suffix(vault, &mut cvar),
            Err(ActorAnimationConstructionError::VaultPrefixRequired)
        );
        assert_eq!(cvar.reads, 1);
    }
}

#[test]
fn null_vault_binding_is_legal_and_failed_cvar_preserves_preceding_stores() {
    let authority = ActorGraphAuthority::new(0);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, false);
    let vault = offboard_vault(false);
    let mut cvar = SprintCvar {
        result: Err(Failure::PlayerQuery),
        reads: 0,
    };
    assert_eq!(
        anim.try_construct_vault_suffix(Arc::clone(&vault), &mut cvar),
        Err(ActorAnimationConstructionError::VaultPrefixRequired)
    );
    assert_eq!(cvar.reads, 0);
    assert!(anim.offboard_attributes().is_none());
    anim.try_construct_prefix(&mut registry).unwrap();
    assert_eq!(
        anim.try_construct_vault_suffix(Arc::clone(&vault), &mut cvar),
        Err(ActorAnimationConstructionError::Cvar(Failure::PlayerQuery))
    );
    assert_eq!(anim.phase(), ActorAnimationPhase::SamplingSprintCvar);
    assert!(anim.offboard_attributes().unwrap().is_null());
    assert_eq!(anim.before_sprint().unwrap().foot_down, -1);
    assert_eq!(
        anim.before_sprint().unwrap().moving_objects.flags_known_mask,
        0x80
    );
    assert_eq!(anim.before_sprint().unwrap().moving_objects.flags_bits, 0);
    assert_eq!(anim.player().unwrap().flags_bits, 0x0004_0010);
    assert_eq!(anim.player().unwrap().flags_known_mask, u32::MAX);
    assert!(anim.before_face().is_none());
    assert_eq!(cvar.reads, 1);
    assert_eq!(
        anim.try_construct_vault_suffix(vault, &mut cvar),
        Err(ActorAnimationConstructionError::VaultPrefixRequired)
    );
    assert_eq!(cvar.reads, 1);
}

#[test]
fn only_actual_null_face_argument_finishes_without_a_face_manager() {
    for face in [
        None,
        Some(Arc::from(&b""[..])),
        Some(Arc::from(&b"\0"[..])),
        Some(Arc::from(&b"tutorial_male"[..])),
    ] {
        let authority = ActorGraphAuthority::new(0);
        let mut registry = Registry::new(&authority);
        let mut anim = construction(&authority, false);
        anim.inputs.face_name = face;
        assert_eq!(
            anim.try_finish_null_face(),
            Err(ActorAnimationConstructionError::FacePrefixRequired)
        );
        anim.try_construct_prefix(&mut registry).unwrap();
        let mut cvar = SprintCvar {
            result: Ok(1),
            reads: 0,
        };
        anim.try_construct_vault_suffix(offboard_vault(false), &mut cvar)
            .unwrap();
        let prior_flags = anim.player().unwrap().flags_bits;
        if anim.inputs.face_name.is_none() {
            anim.try_finish_null_face().unwrap();
            assert_eq!(anim.phase(), ActorAnimationPhase::ConstructorReturned);
            assert_eq!(
                anim.try_finish_null_face(),
                Err(ActorAnimationConstructionError::FacePrefixRequired)
            );
        } else {
            assert_eq!(
                anim.try_finish_null_face(),
                Err(ActorAnimationConstructionError::FaceManagerRequired)
            );
            assert_eq!(anim.phase(), ActorAnimationPhase::FaceSetupRequired);
        }
        assert_eq!(anim.player().unwrap().flags_bits, prior_flags);
        assert_eq!(anim.skater().unwrap().face_pose_name, EMPTY_FAST_STRING_36);
        assert_eq!(
            anim.skater().unwrap().face_playback,
            AnimationPointerState::Null
        );
        assert_eq!(
            anim.skater().unwrap().database_manager,
            AnimationPointerState::Null
        );
    }
}

#[test]
fn collision_turn_uses_real_owner_and_lazy_cvars() {
    use crate::{
        input_cvar_runtime::{InputCvarManager, InputCvarReferences},
        reckoning_numeric::HostReckoningNumeric,
        reckoning_sin_cos::HostReckoningSinCosNumeric,
        skeleton_root_transforms::SkeletonRootVector as V,
    };
    let authority = ActorGraphAuthority::new(0);
    let mut registry = Registry::new(&authority);
    let mut anim = construction(&authority, false);
    anim.try_construct_prefix(&mut registry).unwrap();
    anim.try_construct_vault_suffix(
        offboard_vault(false),
        &mut SprintCvar {
            result: Ok(1),
            reads: 0,
        },
    )
    .unwrap();
    anim.before_face.as_mut().unwrap().last_collision_normal = V {
        xyz: [0.0, 0.0, 1.0],
        w: None,
    };
    let refs = InputCvarReferences::new_static_initialized();
    assert_eq!(
        anim.try_calculate_ob_collision_turn(
            V::known([0.0, 0.0, 1.0, 0.0]),
            false,
            &refs,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric
        )
        .unwrap(),
        0.0
    );
    assert!(
        anim.try_calculate_ob_collision_turn(
            V::known([0.0, 0.0, -1.0, 0.0]),
            false,
            &refs,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric
        )
        .is_err()
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let result = anim
        .try_calculate_ob_collision_turn(
            V::known([0.0, 0.0, -1.0, 0.0]),
            false,
            &refs,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(result, 0.0);
    assert_ne!(anim.player.as_ref().unwrap().flags_bits & 4, 0);
    assert_eq!(
        anim.before_face.as_ref().unwrap().last_collision_normal.w,
        None
    );
    // Missing source attribute falls back to the native zero80-byte graph.
    assert_eq!(
        anim.try_calculate_ob_step_turn(-1.0).unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
}
