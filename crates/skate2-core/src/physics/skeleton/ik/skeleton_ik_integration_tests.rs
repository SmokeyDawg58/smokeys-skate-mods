use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node},
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skeleton_ik::{SkeletonIkDataIn, SkeletonIkUpdateError},
    skeleton_ik_geometry::{SkeletonIkGeometryError, SkeletonIkGeometryProgress},
    skeleton_root_transforms::SkeletonRootVector,
};

struct Deck {
    reads: usize,
    fail: Option<usize>,
}
impl SkateboardAttributeRead for Deck {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        assert_eq!(class, SkateboardAttributeClass::Deck);
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.reads += 1;
        if self.fail == Some(self.reads) {
            return Err("deck read");
        }
        Ok(match key {
            0xd76d_9db5_ee1f_d008 => 0.5,
            0xc94d_6509_1dcb_828b => 0.75,
            0x1d53_bc53_9321_90e7 => 0.125,
            0x8020_3de9_f9b7_32ca => 30.0,
            _ => panic!("unexpected deck key"),
        })
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("no byte")
    }
    fn integer(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no integer")
    }
}
fn vault(malformed_last: bool) -> Arc<AttributeRuntimeVault> {
    let class = 0xc8be_3349_f2bd_493e;
    let on_deck = 0x39fa_8c07_e835_6284;
    let external = 0x094a_ce5d_e068_51fc;
    let mut vault = graph(
        class,
        vec![
            definition(on_deck, 4, 4),
            definition(external, 4, if malformed_last { 1 } else { 4 }),
        ],
    );
    collection(
        &mut vault,
        class,
        0xd7ed_bd36_2d7d_2152,
        0,
        None,
        vec![
            node(on_deck, 0x40, 0.25_f32.to_bits()),
            node(external, 0x40, 0.125_f32.to_bits()),
        ],
    );
    Arc::new(vault)
}
fn data(seed: f32) -> SkeletonIkDataIn {
    let matrix = |offset| {
        SkeletonRootMatrix::known(std::array::from_fn(|i| {
            seed + offset + f32::from(u16::try_from(i).unwrap())
        }))
    };
    SkeletonIkDataIn {
        animation_to_world: matrix(0.0),
        world_to_animation: matrix(20.0),
        physics_skateboard: matrix(40.0),
        inverse_skateboard: matrix(60.0),
        hips: matrix(80.0),
        skateboard: matrix(100.0),
        toe_contact_normal: SkeletonRootVector {
            xyz: [2.0, 3.0, 5.0],
            w: None,
        },
        hips_offset: SkeletonRootVector::known([7.0, 11.0, 13.0, -0.0]),
    }
}
fn matrices(data: &SkeletonIkDataIn) -> [[u32; 16]; 6] {
    [
        data.animation_to_world,
        data.world_to_animation,
        data.physics_skateboard,
        data.inverse_skateboard,
        data.hips,
        data.skateboard,
    ]
    .map(|m| m.known_lanes().unwrap().map(f32::to_bits))
}

#[test]
fn failed_constructor_retains_reset_on_same_targets_before_partial_deck_geometry() {
    let mut f = Fixture::new();
    f.pose[24][12] = 31.0;
    f.process().unwrap();
    assert_ne!(f.current.ik().targets()[0][12].to_bits(), 0);
    let ids = *f.current.ik().target_bone_ids();
    let owner = Arc::clone(f.current.ik().data());
    let mut deck = Deck {
        reads: 0,
        fail: Some(2),
    };
    assert!(matches!(
        f.current.try_construct_ik(
            &vault(false),
            &mut deck,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonIkGeometryError::Deck("deck read"))
    ));
    let runtime = f.current.ik_runtime().unwrap();
    assert_eq!(
        runtime.geometry().progress(),
        SkeletonIkGeometryProgress::BoxHalfLength
    );
    assert_eq!(
        runtime.geometry().box_half_width().unwrap().to_bits(),
        0.25_f32.to_bits()
    );
    assert!(runtime.geometry().box_half_length().is_none());
    assert_eq!(runtime.state().modes(), &[0; 4]);
    assert!(runtime.state().do_ik());
    assert_eq!(matrices(runtime.data_in()), [[0; 16]; 6]);
    assert_eq!(
        runtime.data_in().toe_contact_normal,
        SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(
        runtime.data_in().hips_offset,
        SkeletonRootVector::known([0.0; 4])
    );
    assert!(Arc::ptr_eq(f.current.ik().data(), &owner));
    assert_eq!(f.current.ik().target_bone_ids(), &ids);
    for matrix in f.current.ik().targets() {
        assert_eq!(
            matrix.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
    }
    assert!(matches!(
        f.current.try_construct_ik(
            &vault(false),
            &mut deck,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonIkGeometryError::AlreadyAttempted(
            SkeletonIkGeometryProgress::BoxHalfLength
        ))
    ));
    assert_eq!(deck.reads, 2);
    assert!(matches!(
        f.current.try_update_ik_prefix(data(1.0), 0, 0),
        Err(SkeletonIkUpdateError::IncompleteConstruction(
            SkeletonIkGeometryProgress::BoxHalfLength
        ))
    ));
    assert_eq!(
        matrices(f.current.ik_runtime().unwrap().data_in()),
        [[0; 16]; 6]
    );
}

#[test]
fn teleport_clears_live_ik_histories_but_keeps_data_geometry_instance_and_mapping() {
    let mut f = Fixture::new();
    let mut deck = Deck {
        reads: 0,
        fail: None,
    };
    f.current
        .try_construct_ik(
            &vault(false),
            &mut deck,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    f.current
        .try_update_ik_prefix(data(17.0), 1 << 10, 0)
        .unwrap();
    let ids = *f.current.ik().target_bone_ids();
    let runtime = f.current.ik_runtime().unwrap();
    assert_eq!(runtime.state().modes(), &[1, 1, 0, 1]);
    assert_eq!(
        runtime.state().on_deck_blends().map(f32::to_bits),
        [0.25_f32.to_bits(), 0.25_f32.to_bits(), 0, 0]
    );
    let before = matrices(runtime.data_in());
    let slope = runtime.geometry().end_slope().unwrap().to_bits();
    f.current.reset_teleport_animation_projection();
    let runtime = f.current.ik_runtime().unwrap();
    assert_eq!(runtime.state().modes(), &[0; 4]);
    assert_eq!(runtime.state().on_deck_blends().map(f32::to_bits), [0; 4]);
    assert_eq!(matrices(runtime.data_in()), before);
    assert_eq!(runtime.data_in().toe_contact_normal.w, None);
    assert_eq!(
        runtime.data_in().hips_offset.w.unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(runtime.geometry().end_slope().unwrap().to_bits(), slope);
    assert_eq!(
        runtime.geometry().total_half_length().unwrap().to_bits(),
        0.5_f32.to_bits()
    );
    assert_eq!(f.current.ik().target_bone_ids(), &ids);
    f.current.try_update_ik_prefix(data(29.0), 0, 0).unwrap();
    assert_eq!(
        f.current.ik_runtime().unwrap().state().on_deck_blends()[0].to_bits(),
        0.25_f32.to_bits(),
        "same retained instance remains readable after reset"
    );
    assert_eq!(deck.reads, 4);
}

#[test]
fn late_malformed_blend_keeps_data_assignment_and_status_before_any_blend_store() {
    let mut f = Fixture::new();
    let mut deck = Deck {
        reads: 0,
        fail: None,
    };
    f.current
        .try_construct_ik(
            &vault(true),
            &mut deck,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert!(matches!(
        f.current.try_update_ik_prefix(data(43.0), 1 << 9, 0),
        Err(SkeletonIkUpdateError::Blend(
            crate::skeleton_ik_state::SkeletonIkBlendError::Attribute(
                AttributeRuntimeError::PayloadSize {
                    key: 0x094a_ce5d_e068_51fc,
                    expected: 4,
                    actual: 1
                }
            )
        ))
    ));
    let runtime = f.current.ik_runtime().unwrap();
    assert_eq!(matrices(runtime.data_in()), matrices(&data(43.0)));
    assert_eq!(runtime.state().modes(), &[1, 1, 1, 0]);
    assert_eq!(runtime.state().on_deck_blends().map(f32::to_bits), [0; 4]);
    assert_eq!(runtime.state().external_blends().map(f32::to_bits), [0; 4]);
}

#[test]
fn wipeout_data_only_and_failed_status_preserve_real_adjustment_prefix() {
    let mut f = Fixture::new();
    let mut deck = Deck {
        reads: 0,
        fail: None,
    };
    let class = 0xc8be_3349_f2bd_493e;
    let key = 0x54c3_5670_4afa_0a9a;
    let mut attributes = graph(class, vec![definition(key, 4, 4)]);
    collection(
        &mut attributes,
        class,
        0xd7ed_bd36_2d7d_2152,
        0,
        None,
        vec![node(key, 0x40, 2.0_f32.to_bits())],
    );
    f.current
        .try_construct_ik(
            &Arc::new(attributes),
            &mut deck,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    let before = *f.current.ik().targets();
    let prior_modes = *f.current.ik_runtime().unwrap().state().modes();
    f.current.try_update_ik_data_only(data(19.0)).unwrap();
    assert_eq!(
        f.current.ik_runtime().unwrap().state().modes(),
        &prior_modes
    );
    assert_eq!(
        matrices(f.current.ik_runtime().unwrap().data_in()),
        matrices(&data(19.0))
    );
    let result=f.current.try_begin_wipeout_ik_adjust(||Err(crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError::Missing("status")),||panic!("blend read after failed status"));
    assert!(matches!(result, Err(SkeletonIkUpdateError::Input(_))));
    assert!(f.current.ik_runtime().unwrap().state().do_ik());
    assert_eq!(
        f.current
            .ik_runtime()
            .unwrap()
            .state()
            .on_deck_blends()
            .map(f32::to_bits),
        [1.0_f32.to_bits(), 1.0_f32.to_bits(), 0, 0]
    );
    for (before, after) in before.iter().zip(f.current.ik().targets()) {
        for lane in 0..16 {
            let expected = if lane == 13 {
                before[lane] + 2.0
            } else {
                before[lane]
            };
            assert_eq!(after[lane].to_bits(), expected.to_bits());
        }
    }
}
