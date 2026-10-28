use super::*;
use crate::{
    actor_graph_construction::{ActorGraphComponents, ActorGraphInterface, ActorGraphRegistry},
    motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority},
};
use std::sync::Arc;
#[test]
fn comparison_raw_ordered_and_unordered_branches_are_distinguished() {
    let operations = [
        ComparisonOp::Equal,
        ComparisonOp::NotEqual,
        ComparisonOp::Greater,
        ComparisonOp::Less,
        ComparisonOp::GreaterEqual,
        ComparisonOp::LessEqual,
        ComparisonOp::Nop,
    ];
    let observed = |value, reference| {
        operations.map(|op| {
            compare(
                Comparison {
                    op,
                    absolute: false,
                    reference,
                },
                value,
            )
        })
    };
    assert_eq!(
        observed(2.0, 1.0),
        [false, true, true, false, true, false, false]
    );
    assert_eq!(
        observed(1.0, 2.0),
        [false, true, false, true, false, true, false]
    );
    assert_eq!(
        observed(-0.0, 0.0),
        [true, false, false, false, true, true, false]
    );
    assert_eq!(
        observed(f32::NAN, 1.0),
        [false, true, false, false, true, true, false]
    );
    assert_eq!(
        observed(1.0, f32::NAN),
        [false, true, false, false, true, true, false]
    );
    assert!(compare(
        Comparison {
            op: ComparisonOp::Greater,
            absolute: true,
            reference: 2.0
        },
        -3.0
    ));
    assert!(!compare(
        Comparison {
            op: ComparisonOp::Greater,
            absolute: false,
            reference: 2.0
        },
        -3.0
    ));
}
struct Registry {
    entity: MotionGraphEntityAuthority<u32>,
    entries: Vec<(ActorGraphInterface, Arc<()>)>,
}
impl ActorGraphRegistry<u32> for Registry {
    type Error = ();
    fn register_graph_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<u32>,
        kind: ActorGraphInterface,
        occurrence: &Arc<()>,
    ) -> Result<(), Self::Error> {
        assert!(self.entity.is_same_issuance(entity));
        self.entries.push((kind, Arc::clone(occurrence)));
        Ok(())
    }
}
#[test]
fn has_intent_reads_the_constructed_same_action_map_and_distinguishes_zero_from_absence() {
    let authority = ActorGraphAuthority::new(9);
    let mut registry = Registry {
        entity: authority.entity_handle(),
        entries: Vec::new(),
    };
    let mut graphs = ActorGraphComponents::new(&authority);
    graphs.try_construct_action(&mut registry).unwrap();
    let action = graphs.action_mut().unwrap();
    assert!(Arc::ptr_eq(&registry.entries[0].1, action.occurrence()));
    let intents = action.intents.as_mut().unwrap();
    let name = ActorGraphIntents::native_name(b"ForceBrake");
    let present = Comparison {
        op: ComparisonOp::Nop,
        absolute: false,
        reference: 0.0,
    };
    assert!(!has_action_intent(present, &name, intents));
    intents.insert_unique(name, 0.0).unwrap();
    assert!(has_action_intent(present, &name, intents));
    assert!(!has_action_intent(
        Comparison {
            op: ComparisonOp::Greater,
            ..present
        },
        &name,
        intents
    ));
    intents.insert_or_assign(name, f32::NAN).unwrap();
    assert!(has_action_intent(present, &name, intents));
    assert!(has_action_intent(
        Comparison {
            op: ComparisonOp::GreaterEqual,
            ..present
        },
        &name,
        intents
    ));
    intents.clear();
    assert!(!has_action_intent(
        Comparison {
            op: ComparisonOp::GreaterEqual,
            ..present
        },
        &name,
        intents
    ));
}
