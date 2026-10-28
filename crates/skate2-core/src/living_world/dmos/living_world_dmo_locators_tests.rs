#![expect(clippy::float_cmp, reason = "exact native distances and lanes")]
use super::*;
use std::collections::HashMap;

#[test]
fn rain_coordinate_removal_keeps_other_nodes_and_reuses_pool_capacity() {
    let mut tree = LocatorKdTree::try_with_blocks(4).unwrap();
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    assert!(tree.insert(10, [0., 0.]));
    assert!(tree.insert(20, [-1., 0.]));
    assert!(tree.insert(30, [1., 0.]));
    assert!(tree.insert(40, [0., 0.]));
    assert_eq!(tree.nearest([0., 0.], &mut numeric).unwrap().unwrap().0, 10);
    assert_eq!(
        tree.remove_coordinate([0., 0.], &mut numeric).unwrap(),
        Some(10)
    );
    assert_eq!(tree.nearest([0., 0.], &mut numeric).unwrap().unwrap().0, 40);
    assert!(tree.insert(50, [4., 0.]));
    assert!(!tree.insert(60, [5., 0.]));
    let mut ids: Vec<_> = tree
        .find_near_numeric([0., 0.], 10., 4, &mut numeric)
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, [20, 30, 40, 50]);
    assert_eq!(
        tree.remove_coordinate([0.00001, 0.], &mut numeric).unwrap(),
        None
    );
    tree.reset();
    assert_eq!(tree.nearest([0., 0.], &mut numeric).unwrap(), None);
    for i in 0..4 {
        assert!(tree.insert(i, [i as f32, 0.]));
    }
}

#[test]
fn streamed_removal_frees_passive_locators_but_keeps_attached_until_detach() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut objects = super::super::dmo_objects::DmoObjectMan::try_construct().unwrap();
    let mut services = Services::default();
    services.chunks.insert(
        10,
        vec![record(1, 1, [0.0; 3]), record(2, 1, [2.0, 0.0, 0.0])],
    );
    man.add_streamed_locator_data(10, 0, true, true, false, &mut services)
        .unwrap();
    man.attach_object(2, 123, &mut services).unwrap();
    man.remove_streamed_locator_data(10, 0, &mut objects, &mut services)
        .unwrap();
    assert!(man.locator(1).is_none());
    assert_eq!(man.locator(2).unwrap().user_object_id, 123);
    assert!(man.chunk_ids.is_empty());
    man.detach_object(2, &mut services).unwrap();
    assert_eq!(man.locator_count(), 0);
    for _ in 0..600 {
        man.add_streamed_locator_data(10, 0, true, true, false, &mut services)
            .unwrap();
        man.remove_streamed_locator_data(10, 0, &mut objects, &mut services)
            .unwrap();
        assert_eq!(man.locator_count(), 0);
    }
}

#[test]
fn streamed_removal_preserves_native_nonpersistent_flag_branch() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut objects = super::super::dmo_objects::DmoObjectMan::try_construct().unwrap();
    let mut services = Services::default();
    services.chunks.insert(10, vec![record(1, 1, [0.0; 3])]);
    man.add_streamed_locator_data(10, 0, false, true, false, &mut services)
        .unwrap();
    // AllowPersistence=true and locator bit0x40 clear: original keeps it.
    man.remove_streamed_locator_data(10, 0, &mut objects, &mut services)
        .unwrap();
    assert!(man.locator(1).is_some());
    man.remove_streamed_locator_data(10, 0x20, &mut objects, &mut services)
        .unwrap();
    assert!(man.locator(1).is_none());
}

#[derive(Default)]
struct Services {
    disabled: bool,
    keep_bad: bool,
    chunks: HashMap<u64, Vec<Dmo>>,
}

impl LocatorServices for Services {
    type Error = ();
    fn enable_locator_loading(&mut self) -> Result<bool, ()> {
        Ok(!self.disabled)
    }
    fn remove_bad_locators(&mut self) -> Result<bool, ()> {
        Ok(!self.keep_bad)
    }
    fn locator_chunk(&mut self, asset_id: u64) -> Result<Option<Vec<Dmo>>, ()> {
        Ok(self.chunks.get(&asset_id).cloned())
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, ()> {
        Ok(1.0 / value.sqrt())
    }
}

fn record(guid: u64, model: u64, position: [f32; 3]) -> Dmo {
    Dmo {
        transform: [
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            position[0],
            position[1],
            position[2],
            1.0,
        ],
        bounding_box_min: [0.0; 4],
        bounding_box_max: [0.0; 4],
        guid,
        local_guid: 0,
        instance_guid: model,
        serialized_instance_word: 0,
        name_offset: 0,
        name: String::new(),
    }
}

fn tree(points: &[(u64, [f32; 2])]) -> LocatorKdTree {
    let mut tree = LocatorKdTree::try_new().unwrap();
    for &(id, coord) in points {
        assert!(tree.insert(id, coord));
    }
    tree
}

fn ids(tree: &LocatorKdTree, found: &[(u16, f32)]) -> Vec<u64> {
    found
        .iter()
        .map(|&(index, _)| tree.node(index).id)
        .collect()
}

const A: (u64, [f32; 2]) = (1, [0.0, 0.0]);
const B: (u64, [f32; 2]) = (2, [-1.0, 5.0]);
const C: (u64, [f32; 2]) = (3, [1.0, -5.0]);
const D: (u64, [f32; 2]) = (4, [-2.0, -1.0]);
const E: (u64, [f32; 2]) = (5, [2.0, 3.0]);

#[test]
fn insert_alternates_dimensions_and_sends_ties_left() {
    let tree = tree(&[A, B, C, D, E, (6, [0.0, 9.0])]);
    let root = tree.root.unwrap();
    let a = *tree.node(root);
    assert_eq!(a.id, 1);
    let b = *tree.node(a.left.unwrap());
    let c = *tree.node(a.right.unwrap());
    assert_eq!((b.id, c.id), (2, 3));
    assert_eq!(tree.node(b.left.unwrap()).id, 4);
    assert_eq!(tree.node(c.right.unwrap()).id, 5);
    // x == 0 at the root descends left; y 9 > 5 at B descends right.
    assert_eq!(tree.node(b.right.unwrap()).id, 6);
}

#[test]
fn node_walk_order_limit_and_distance_multiset() {
    let tree = tree(&[A, B, C, D, E]);
    let mut services = Services::default();
    let walk = tree
        .find_near_nodes([0.0, 0.0], 10.0, 16, &mut services)
        .unwrap();
    assert_eq!(ids(&tree, &walk), [1, 2, 4, 3, 5]);
    let sorted = tree.find_near([0.0, 0.0], 10.0, 16, &mut services).unwrap();
    let order: Vec<u64> = sorted.iter().map(|&(id, _)| id).collect();
    // B and C tie at sqrt(26); the multiset keeps walk order.
    assert_eq!(order, [1, 4, 5, 2, 3]);
    assert!(sorted[3].1 == sorted[4].1);
    // The walk stops at three held nodes: E is closer than B but never seen.
    let capped = tree.find_near([0.0, 0.0], 10.0, 3, &mut services).unwrap();
    let order: Vec<u64> = capped.iter().map(|&(id, _)| id).collect();
    assert_eq!(order, [1, 4, 2]);
    assert_eq!(
        tree.find_near([0.0, 0.0], 10.0, 257, &mut services),
        Err(LocatorError::FindNearLimit { requested: 257 })
    );
}

#[test]
fn removing_the_root_copies_right_minima_down_and_frees_the_leaf() {
    let mut tree = tree(&[A, B, C, D, E]);
    let mut services = Services::default();
    let root = tree.root.unwrap();
    let c_node = tree.node(root).right.unwrap();
    let e_node = tree.node(c_node).right.unwrap();
    assert!(tree.remove(1, [0.0, 0.0], &mut services).unwrap());
    assert_eq!(tree.root, Some(root));
    assert_eq!(tree.node(root).id, 3);
    assert_eq!(tree.node(root).coord, [1.0, -5.0]);
    assert_eq!(tree.node(c_node).id, 5);
    assert_eq!(tree.node(c_node).right, None);
    assert!(tree.nodes[usize::from(e_node)].is_none());
    assert_eq!(tree.free.last(), Some(&e_node));
    assert!(!tree.remove(1, [0.0, 0.0], &mut services).unwrap());
}

#[test]
fn lone_left_subtree_moves_right_before_the_minimum_search() {
    let mut tree = tree(&[A, B, C, D, E]);
    let mut services = Services::default();
    let b_node = tree.node(tree.root.unwrap()).left.unwrap();
    assert!(tree.remove(2, [-1.0, 5.0], &mut services).unwrap());
    let b = *tree.node(b_node);
    assert_eq!((b.id, b.left, b.right), (4, None, None));
}

#[test]
fn identity_locator_keeps_the_native_axis_w_lanes() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut services = Services::default();
    services
        .chunks
        .insert(0x10, vec![record(7, 70, [3.0, 4.0, 5.0])]);
    man.add_streamed_locator_data(0x10, 0, true, true, false, &mut services)
        .unwrap();
    let locator = man.locator(7).unwrap();
    assert_eq!(
        locator.transform.map(f32::to_bits),
        [
            1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.0, 4.0, 5.0, 1.0
        ]
        .map(f32::to_bits)
    );
    assert_eq!(locator.initial_stream_transform, locator.transform);
    assert_eq!((locator.model_id, locator.attachment_time), (70, 0));
    assert_eq!(locator.flags, 0xe0);
    let near = man
        .passive_locators_within_range([3.0, 5.0], 1.0, 4, &mut services)
        .unwrap();
    assert_eq!(near, [(7, 0.0)]);
}

#[test]
fn chunk_flags_bad_ids_and_repeat_admission() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut services = Services::default();
    let bad = BAD_LOCATOR_IDS[2];
    services.chunks.insert(
        0x20,
        vec![record(1, 10, [0.0; 3]), record(bad, 11, [1.0, 0.0, 1.0])],
    );
    services
        .chunks
        .insert(0x21, vec![record(2, 12, [2.0, 0.0, 2.0])]);
    services
        .chunks
        .insert(0x22, vec![record(3, 13, [3.0, 0.0, 3.0])]);
    man.add_streamed_locator_data(0x20, 0, true, true, false, &mut services)
        .unwrap();
    man.add_streamed_locator_data(0x21, 0x10 | 0x20, true, true, false, &mut services)
        .unwrap();
    man.add_streamed_locator_data(0x22, 0x40, true, true, true, &mut services)
        .unwrap();
    assert!(man.locator(bad).is_none());
    assert_eq!(man.locator(1).unwrap().flags, 0xe0);
    assert_eq!(man.locator(2).unwrap().flags, 0xa8);
    assert_eq!(man.locator(3).unwrap().flags, 0xc8);
    man.blocks[usize::from(man.locators[&1])]
        .as_mut()
        .unwrap()
        .flags = 0x10;
    man.add_streamed_locator_data(0x20, 0, true, true, false, &mut services)
        .unwrap();
    assert_eq!(man.locator(1).unwrap().flags, 0x90);
    assert_eq!(man.chunk_ids(), [0x20, 0x21, 0x22]);
    assert_eq!(man.locator_count(), 3);
    services.keep_bad = true;
    man.add_streamed_locator_data(0x20, 0, false, true, false, &mut services)
        .unwrap();
    assert_eq!(man.locator(bad).unwrap().flags, 0xa0);
    assert_eq!(man.locator(1).unwrap().flags, 0xb0);
}

#[test]
fn moved_locator_leaves_its_old_kd_entry_and_attached_locators_stay() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut services = Services::default();
    services
        .chunks
        .insert(0x30, vec![record(9, 90, [10.0, 0.0, 10.0])]);
    services
        .chunks
        .insert(0x31, vec![record(9, 91, [20.0, 0.0, 20.0])]);
    man.add_streamed_locator_data(0x30, 0x20, true, true, false, &mut services)
        .unwrap();
    man.add_streamed_locator_data(0x31, 0x20, true, true, false, &mut services)
        .unwrap();
    assert_eq!(man.locator(9).unwrap().model_id, 91);
    let old = man
        .passive_locators_within_range([10.0, 10.0], 1.0, 8, &mut services)
        .unwrap();
    assert!(old.is_empty());
    let new = man
        .passive_locators_within_range([20.0, 20.0], 1.0, 8, &mut services)
        .unwrap();
    assert_eq!(new, [(9, 0.0)]);
    man.blocks[usize::from(man.locators[&9])]
        .as_mut()
        .unwrap()
        .user_object_id = 5;
    man.add_streamed_locator_data(0x30, 0x20, true, true, false, &mut services)
        .unwrap();
    assert_eq!(man.locator(9).unwrap().model_id, 91);
}

#[test]
fn disabled_loading_and_missing_chunks() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut services = Services {
        disabled: true,
        ..Services::default()
    };
    man.add_streamed_locator_data(0x40, 0, true, true, false, &mut services)
        .unwrap();
    assert!(man.chunk_ids().is_empty());
    services.disabled = false;
    assert_eq!(
        man.add_streamed_locator_data(0x40, 0, true, true, false, &mut services),
        Err(LocatorError::MissingChunk { asset_id: 0x40 })
    );
    let mut bad = record(4, 40, [0.0; 3]);
    bad.transform[5] = f32::NAN;
    services.chunks.insert(0x41, vec![bad]);
    assert_eq!(
        man.add_streamed_locator_data(0x41, 0, true, true, false, &mut services),
        Err(LocatorError::Assert {
            locator_id: 4,
            site: LocatorAssert::DataTransform,
        })
    );
}

#[test]
fn locator_blocks_run_out_after_1024() {
    let mut man = DynamicObjectLocatorMan::try_construct().unwrap();
    let mut services = Services::default();
    let records = (0..1025_u16)
        .map(|index| record(0x1000 + u64::from(index), 1, [f32::from(index), 0.0, 0.0]))
        .collect();
    services.chunks.insert(0x50, records);
    man.add_streamed_locator_data(0x50, 0, true, true, false, &mut services)
        .unwrap();
    assert_eq!(man.locator_count(), 1024);
    assert!(man.locator(0x1000 + 1024).is_none());
}
