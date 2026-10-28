use super::construct::{BehaviourPayload, ComparisonOp, ConditionMask, ConditionPayload, Filter};
use super::*;
use crate::play_animation::{FastString30, FastString36};

fn context(key: u32) -> BoundMotionGraphContext<u32> {
    BoundMotionGraphContext::new(
        Some(MotionGraphControllerToken::issue()),
        Some(MotionGraphEntityAuthority::issue(key)),
        None,
        None,
        None,
        None,
    )
}
fn factories() -> GeneratedActionFactories {
    let mut factories = GeneratedActionFactories::new();
    factories.register_skater_classes().unwrap();
    factories
}
#[test]
fn complete_source_load_constructs_all_families_and_keeps_controller_unentered() {
    let context = context(7);
    let statics = GeneratedActionStatics::new();
    let controller =
        GeneratedActionController::try_load(&context, &statics, &factories(), true).unwrap();
    let shared = statics.inner.lock().unwrap();
    assert!(shared.printed && shared.behaviours_initialized && shared.conditions_initialized);
    assert_eq!(shared.behaviours.iter().flatten().count(), 311);
    assert_eq!(shared.conditions.iter().flatten().count(), 146);
    let mut behaviours = BTreeMap::new();
    for object in shared.behaviours.iter().flatten() {
        *behaviours.entry(object.class.binding().name).or_insert(0) += 1;
        assert!(object.base.parent.lock().unwrap().is_none());
        assert!(object.base.enabled && object.base.cast_parent);
        assert!(Arc::ptr_eq(&object.base.name, &object.base.path));
    }
    assert_eq!(
        behaviours,
        BTreeMap::from([
            ("JuiceHook", 1),
            ("BodyFlippingSignal", 1),
            ("PrintText2D", 1),
            ("CreateConstMGIntent", 115),
            ("CreateMGTimeIntentFromAGIntent", 3),
            ("CreateMGIntentFromAGIntent", 177),
            ("CreateTrickIntentFromGesture", 7),
            ("BoardAdjust", 6),
        ])
    );
    let mut conditions = BTreeMap::new();
    for object in shared.conditions.iter().flatten() {
        *conditions.entry(object.class.binding().name).or_insert(0) += 1;
        assert!(object.base.parent.lock().unwrap().is_none());
        assert!(object.base.enabled && object.base.cast_parent);
        assert_eq!(object.mask, ConditionMask::Precondition);
    }
    assert_eq!(conditions.len(), 24);
    assert_eq!(conditions["HasAGIntent"], 60);
    assert_eq!(conditions["IsGrinding"], 35);
    assert_eq!(controller.current_state, NULL_INDEX);
    assert_eq!(controller.last_state, NULL_INDEX);
    assert_eq!(controller.delta_time.to_bits(), 0);
    assert!(controller.time_in_state.is_empty() && controller.current_behaviours.is_empty());
    assert!(
        controller
            .instance_slots
            .iter()
            .all(|slot| slot.behaviour.is_none() && slot.instance.is_none())
    );
    assert!(controller.hook.is_none());
    assert_eq!(controller.condition_active, [0; 5]);
    assert_eq!(controller.condition_values, [None; 146]);
    assert!(controller.call_behaviours && !controller.allow_evaluations);
}
#[test]
fn all_native_hierarchy_intervals_and_transition_indirection_are_retained() {
    let context = context(0);
    let controller = GeneratedActionController::try_load(
        &context,
        &GeneratedActionStatics::new(),
        &factories(),
        false,
    )
    .unwrap();
    assert_eq!(controller.root_state().name, "Action");
    assert_eq!(controller.states().len(), 224);
    assert_eq!(controller.transitions().len(), 129);
    assert_eq!(data::ATTRIBUTES.len(), 457);
    assert_eq!(controller.transition_indices().len(), 129);
    for (index, state) in controller.states().iter().enumerate() {
        assert_eq!(state.trailing, 0);
        if state.children.count != 0 {
            let start = usize::from(state.children.index);
            for child in &controller.states()[start..start + usize::from(state.children.count)] {
                assert_eq!(usize::from(child.parent), index);
            }
        }
        if state.behaviours.count != 0 {
            let start = usize::from(state.behaviours.index);
            for behaviour in &data::BEHAVIOURS[start..start + usize::from(state.behaviours.count)] {
                assert_eq!(usize::from(behaviour.parent), index);
                assert_eq!(behaviour.parameters.count, 0);
            }
        }
        if state.transitions.count != 0 {
            let start = usize::from(state.transitions.index);
            for &transition in &controller.transition_indices()
                [start..start + usize::from(state.transitions.count)]
            {
                let transition = &controller.transitions()[usize::from(transition)];
                assert!(usize::from(transition.target) < controller.states().len());
                assert_eq!(transition.hooks.count, 0);
            }
        }
    }
    let mut ancestry = Vec::new();
    let mut current = 94;
    while current != NULL_INDEX {
        ancestry.push(current);
        current = controller.states()[usize::from(current)].parent;
    }
    assert_eq!(ancestry, [94, 51, 19, 6, 1, 0]);
    // These declarations are conditional alternatives, not a chosen default.
    assert_eq!(controller.states()[4].expression, 37);
    assert_eq!(controller.states()[5].expression, 38);
    assert_eq!(controller.states()[19].expression, 56);
    assert_eq!(controller.states()[0].update_address, 0x828e_cbc0);
    assert!(!controller.states()[2].active);
}
#[test]
fn original_constructor_fields_distinguish_filters_defaults_and_required_names() {
    let statics = GeneratedActionStatics::new();
    let _controller =
        GeneratedActionController::try_load(&context(0), &statics, &factories(), true).unwrap();
    let text = statics.behaviour(38).unwrap().unwrap();
    let BehaviourPayload::PrintText { x, y, frames, text } = &text.payload else {
        panic!("PrintText2D");
    };
    assert_eq!((*x, *y, *frames), (200, 200, 100));
    assert!(text.starts_with(b"WARNING: No Action Graph state"));
    assert_eq!(text[1023], 0);
    let board = statics.behaviour(230).unwrap().unwrap();
    let BehaviourPayload::BoardAdjust {
        magnitude,
        angle,
        filter,
        negate_on_mirror,
    } = &board.payload
    else {
        panic!("BoardAdjust");
    };
    assert_eq!(
        *magnitude,
        FastString36::encode("LeftBoardAdjustMag").unwrap()
    );
    assert_eq!(
        *angle,
        FastString36::encode("LeftBoardAdjustAngle").unwrap()
    );
    assert_eq!(*filter, Filter::AngleRotN90);
    assert!(!negate_on_mirror);
    let timer = statics.behaviour(48).unwrap().unwrap();
    let BehaviourPayload::TimeIntent { motion, action } = &timer.payload else {
        panic!("time intent");
    };
    assert_eq!(*motion, FastString36::encode("ManualEngageTime").unwrap());
    assert_eq!(*action, FastString36::encode("Manual").unwrap());
    let speed = statics.condition(101).unwrap().unwrap();
    let ConditionPayload::Speed {
        comparison,
        along_skate_z,
    } = &speed.payload
    else {
        panic!("speed comparison");
    };
    assert_eq!(comparison.op, ComparisonOp::Less);
    assert_eq!(comparison.reference.to_bits(), 4f32.to_bits());
    assert!(comparison.absolute && !along_skate_z);
    let animation = statics.condition(132).unwrap().unwrap();
    let ConditionPayload::AnimationAttribute {
        comparison,
        attribute,
        sequence,
    } = &animation.payload
    else {
        panic!("animation attribute");
    };
    assert_eq!(*attribute, FastString30::encode("IdleDismounting").unwrap());
    assert_eq!(*sequence, -1);
    assert_eq!(comparison.op, ComparisonOp::Nop);
}
#[test]
fn second_controller_reuses_shared_objects_and_preserves_distinct_context_identity() {
    let statics = GeneratedActionStatics::new();
    let factories = factories();
    let first_context = context(7);
    let mut first =
        GeneratedActionController::try_load(&first_context, &statics, &factories, true).unwrap();
    let object = statics.behaviour(1).unwrap().unwrap();
    first.condition_active[0] = 7;
    first.current_state = 94;
    let second_context = context(7);
    let second = GeneratedActionController::try_load_with_allocations(
        &second_context,
        &statics,
        &factories,
        false,
        &mut |_| panic!("native initialized statics must not allocate again"),
    )
    .unwrap();
    assert!(Arc::ptr_eq(
        &object,
        &second.statics.behaviour(1).unwrap().unwrap()
    ));
    assert_eq!(second.condition_active, [0; 5]);
    assert_eq!(second.current_state, NULL_INDEX);
    assert!(!second.call_behaviours);
    assert_eq!(
        first.validate_context(&second_context),
        Err(GeneratedActionLoadError::ForeignContext)
    );
    assert!(first.validate_context(&first_context).is_ok());
}
#[test]
fn native_null_factory_result_is_retained_until_actual_callback_boundary() {
    let context = context(0);
    let statics = GeneratedActionStatics::new();
    let controller = GeneratedActionController::try_load_with_allocations(
        &context,
        &statics,
        &factories(),
        true,
        &mut |site| {
            Ok(if site.index == 0 {
                GeneratedActionAllocation::Null
            } else {
                GeneratedActionAllocation::Host
            })
        },
    )
    .unwrap();
    assert!(statics.behaviour(0).unwrap().is_none());
    assert!(statics.condition(0).unwrap().is_none());
    assert_eq!(
        controller.try_create_behaviour_instance(&context, 0),
        Err(GeneratedActionLoadError::NullObject {
            domain: ObjectDomain::Behaviour,
            index: 0
        })
    );
    assert_eq!(
        controller.try_condition_activation(&context, 0),
        Err(GeneratedActionLoadError::NullObject {
            domain: ObjectDomain::Condition,
            index: 0
        })
    );
    assert_eq!(
        controller.try_create_behaviour_instance(&context, 1),
        Err(GeneratedActionLoadError::UnrecoveredCallback {
            domain: ObjectDomain::Behaviour,
            index: 1,
            address: 0x82cb_a730
        })
    );
}
#[test]
fn failure_prefix_respects_different_native_static_init_store_positions() {
    let context = context(0);
    let statics = GeneratedActionStatics::new();
    let factories = factories();
    assert!(matches!(
        GeneratedActionController::try_load_with_allocations(
            &context,
            &statics,
            &factories,
            true,
            &mut |site| if site.domain == ObjectDomain::Behaviour && site.index == 2 {
                Err(GeneratedActionLoadError::Allocation)
            } else {
                Ok(GeneratedActionAllocation::Host)
            },
        ),
        Err(GeneratedActionLoadError::Allocation)
    ));
    let first_behaviour = statics.behaviour(0).unwrap().unwrap();
    assert!(statics.behaviour(2).unwrap().is_none());
    // sInit was already set by behaviour0, so another ctor skips this loop.
    assert!(matches!(
        GeneratedActionController::try_load_with_allocations(
            &context,
            &statics,
            &factories,
            true,
            &mut |site| {
                assert_eq!(site.domain, ObjectDomain::Condition);
                if site.index == 2 {
                    Err(GeneratedActionLoadError::Allocation)
                } else {
                    Ok(GeneratedActionAllocation::Host)
                }
            },
        ),
        Err(GeneratedActionLoadError::Allocation)
    ));
    let first_condition = statics.condition(0).unwrap().unwrap();
    let _controller =
        GeneratedActionController::try_load(&context, &statics, &factories, true).unwrap();
    assert!(Arc::ptr_eq(
        &first_behaviour,
        &statics.behaviour(0).unwrap().unwrap()
    ));
    assert!(statics.behaviour(2).unwrap().is_none());
    assert!(!Arc::ptr_eq(
        &first_condition,
        &statics.condition(0).unwrap().unwrap()
    ));
    assert!(statics.condition(145).unwrap().is_some());
}
#[test]
fn missing_factory_does_not_publish_fake_objects_and_unrecovered_activation_names_real_entry() {
    let context = context(0);
    let statics = GeneratedActionStatics::new();
    assert!(
        matches!(GeneratedActionController::try_load(&context, &statics, &GeneratedActionFactories::new(), true),
        Err(GeneratedActionLoadError::UnregisteredClass(name)) if name == "JuiceHook")
    );
    assert!(statics.behaviour(0).unwrap().is_none());
    let controller =
        GeneratedActionController::try_load(&context, &statics, &factories(), true).unwrap();
    assert_eq!(
        controller.try_condition_activation(&context, 0),
        Err(GeneratedActionLoadError::UnrecoveredCallback {
            domain: ObjectDomain::Condition,
            index: 0,
            address: 0x82cc_1748
        })
    );
    assert_eq!(
        ConditionClass::HasAGIntent.binding().factory_helper,
        0x82cb_6018
    );
    assert_eq!(ConditionClass::HasAGIntent.binding().allocation_size, 0x40);
    assert_eq!(ConditionClass::HasAGIntent.binding().vtable, 0x8230_1df4);
    assert_eq!(
        controller.try_condition_activation(&context, 146),
        Err(GeneratedActionLoadError::DescriptorBounds)
    );
}

#[test]
fn board_adjust_instance_tracks_source_reversal_and_missing_intent_reset() {
    use std::f32::consts::PI;
    let mut instance = BoardAdjustInstance::new();
    assert_eq!(
        instance.update_angle(1.0),
        Err(BoardAdjustStateError::UninitializedLastAngle)
    );
    instance.begin();
    assert_eq!(instance.update_angle(2.0), Ok(2.0));
    assert_eq!(instance.update_angle(-2.0), Ok(PI));
    assert_eq!(instance.state, 2);
    assert_eq!(instance.last_angle, Some(-2.0));
    assert_eq!(instance.update_angle(2.0), Ok(2.0));
    assert_eq!(instance.state, 0);
    instance.reset_after_missing_intent();
    assert_eq!(instance.state, 0);
    assert_eq!(instance.last_angle, Some(0.0));
    assert_eq!(instance.update_angle(-2.0), Ok(-2.0));
    assert_eq!(instance.update_angle(2.0), Ok(-PI));
    assert_eq!(instance.state, 1);
}
