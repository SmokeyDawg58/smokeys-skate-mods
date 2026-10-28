//! Synthetic source discriminators, not real skater tuning or native traces.
#![expect(
    clippy::float_cmp,
    reason = "exact source branch and f32 discriminator tests"
)]
use super::*;
use crate::{
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_attributes::{SkeletonDriveAttributes, SkeletonJointAttributes, SkeletonMaterial},
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
    skeleton_hat::HostHatNumeric,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Float(Route, u64),
    Byte(Route, u64),
    Instance,
    Mode,
    Factor,
    Drop,
}
struct Reader {
    events: Rc<RefCell<Vec<Event>>>,
    mode: u32,
    factor: f32,
    numerator: f32,
    joints: u8,
}
impl Reader {
    fn new(mode: u32) -> Self {
        Self {
            events: Rc::default(),
            mode,
            factor: 2.0,
            numerator: 3.0,
            joints: 0x80,
        }
    }
}
struct DebugInstance {
    events: Rc<RefCell<Vec<Event>>>,
    mode: u32,
    factor: f32,
}
impl Drop for DebugInstance {
    fn drop(&mut self) {
        self.events.borrow_mut().push(Event::Drop);
    }
}
impl SkeletonBodyDebugRead for DebugInstance {
    type Error = &'static str;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        self.events.borrow_mut().push(Event::Mode);
        Ok(self.mode)
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Factor);
        Ok(self.factor)
    }
}
impl SkeletonBodyAttributeRead for Reader {
    type Error = &'static str;
    type DebugInstance = DebugInstance;
    fn float(&mut self, route: Route, key: u64) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Float(route, key));
        Ok(match (route, key) {
            (Route::LocalBones, keys::CAPSULE_RADIUS_SCALE) => 0.5,
            (Route::GlobalWipeout, keys::RAGDOLL_INVERSE_MASS_NUMERATOR) => self.numerator,
            (Route::GlobalSkeleton, keys::CAPSULE_LENGTH_SCALE) => 2.0,
            (Route::LocalSkeleton, keys::PART_MASS_SCALE) => 3.0,
            (Route::LocalSkeleton, keys::ROOT_HALF_LENGTH) => 0.25,
            (Route::LocalSkeleton, keys::ROOT_RADIUS) => 0.125,
            _ => return Err("unexpected scalar route"),
        })
    }
    fn byte(&mut self, route: Route, key: u64) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push(Event::Byte(route, key));
        assert_eq!((route, key), (Route::GlobalAnimation, keys::ENABLE_JOINTS));
        Ok(self.joints)
    }
    fn debug_instance(&mut self) -> Result<Self::DebugInstance, Self::Error> {
        self.events.borrow_mut().push(Event::Instance);
        Ok(DebugInstance {
            events: Rc::clone(&self.events),
            mode: self.mode,
            factor: self.factor,
        })
    }
}

pub(crate) fn inputs() -> (Arc<SkeletonData>, SkeletonBodyAttributeData) {
    let (hierarchy, mut params, pose) = crate::skeleton_data_tests::fixture();
    for bone in &mut params.bones {
        bone.size = [2.0, 4.0, 8.0, 16.0];
    }
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, params, &pose).unwrap());
    let material = SkeletonMaterial {
        static_friction: 0.25,
        dynamic_friction: 0.75,
        restitution: 0.125,
    };
    let attributes = SkeletonBodyAttributeData {
        normal: material,
        ragdoll: material,
        ragdoll_head: material,
        drives: [SkeletonDriveAttributes::constructor_value(); 24],
        bones: std::array::from_fn(|index| SkeletonBoneAttributes {
            mass_factor: 4.0,
            ragdoll_mass_factor: 2.0,
            has_collision: 0,
            use_root_drive: 0,
            padding: [0; 2],
            volume_type: u32::try_from(index % 3).unwrap(),
            volume_scalar: 1.5,
            num_parents: 0,
        }),
        joints: [SkeletonJointAttributes {
            ball_joint: 0,
            padding: [0; 3],
            swing_angle: 0.0,
            twist_angle: 0.0,
            swing_ragdoll: 0.0,
            twist_ragdoll: 0.0,
        }; 22],
    };
    (data, attributes)
}

fn build(
    data: Arc<SkeletonData>,
    attributes: SkeletonBodyAttributeData,
    reader: &mut Reader,
) -> SkeletonBodyDefinition {
    SkeletonBodyDefinition::try_create(
        data,
        Arc::new(attributes),
        reader,
        0,
        None,
        &mut || panic!("None hat must not sample the CVar"),
        &mut HostSkeletonBodyMassNumeric,
        &mut HostSimulationVolumeIntegralNumeric,
        &mut HostHatNumeric,
    )
    .unwrap()
}

#[test]
fn actual24_owned_shapes_use_source_size_mass_and_both_distinct_caches() {
    let (data, mut attributes) = inputs();
    attributes.bones[0].volume_type = u32::MAX; // Root bypasses the ordinary switch.
    let body = build(Arc::clone(&data), attributes, &mut Reader::new(0));
    assert!(Arc::ptr_eq(&body.data, &data));
    assert_eq!(body.parts.len(), 24);
    assert_eq!(body.allocated_joint_count, 22);
    assert_eq!(body.skateboard_part, SkeletonPartIndex::SkateboardRoot);
    assert_eq!(body.hat_type, 0);
    for (index, part) in body.parts.iter().enumerate() {
        let SkeletonPartGeometry::Primitive(volume) = &part.geometry else {
            panic!("unexpected hat")
        };
        assert_eq!(volume.transform, FRESH_VOLUME_TRANSFORM);
        assert_eq!(volume.flags, u32::from(index != 0)); // Bone.has_collision is not sampled here.
        assert_eq!((volume.group_id, volume.surface_id), (0, 0));
        assert_eq!(part.material.static_friction, 0.75);
        assert_eq!(part.material.dynamic_friction, 0.75);
        assert_eq!(part.material.restitution, 0.125);
        assert_eq!(part.mass.inertia.inverse_mass, part.animated.inverse_mass);
        assert_eq!(
            part.ragdoll.inverse_tensor[3],
            part.ragdoll.inverse_tensor[0]
        );
        assert_eq!(
            part.animated.inverse_tensor[3],
            part.animated.inverse_tensor[0]
        );
        if index != 0 {
            assert_eq!(part.unscaled_part_mass, 64.0);
            assert_eq!(part.effective_mass, 192.0);
            assert_eq!(part.ragdoll.inverse_mass, 3.0 / 384.0);
            assert_eq!(part.animated.inverse_mass, 1.0 / 768.0);
        }
    }
    assert_eq!(body.parts[0].unscaled_part_mass.to_bits(), 0);
    assert_eq!(body.parts[0].effective_mass.to_bits(), 0x3a83_126f);
    assert_eq!(
        body.parts[1].debug,
        SkeletonPartDebugShape::Sphere { radius: 3.0 }
    ); // NOT halfZ6.
    assert_eq!(
        body.parts[2].debug,
        SkeletonPartDebugShape::Capsule {
            radius: 0.75,
            half_length: 10.5
        }
    );
    assert_eq!(
        body.parts[3].debug,
        SkeletonPartDebugShape::Box {
            half_size: [1.5, 3.0, 6.0, 12.0]
        }
    );
    assert_eq!(body.attributes.normal.static_friction, 0.25); // No mutation of the source normal material.
}

#[test]
fn attribute_chronology_keeps_preloop_reads_root_lookup_order_and_fresh24_debug_lifetimes() {
    let (data, attributes) = inputs();
    let mut reader = Reader::new(17); // Unknown mode skips the factor, not an error/default mode1.
    reader.joints = 0;
    let body = build(data, attributes, &mut reader);
    assert_eq!(body.allocated_joint_count, 0);
    let mut expected = vec![
        Event::Float(Route::LocalBones, keys::CAPSULE_RADIUS_SCALE),
        Event::Byte(Route::GlobalAnimation, keys::ENABLE_JOINTS),
        Event::Float(Route::GlobalWipeout, keys::RAGDOLL_INVERSE_MASS_NUMERATOR),
    ];
    for index in 0..24 {
        if index == 0 {
            expected.push(Event::Float(Route::LocalSkeleton, keys::ROOT_HALF_LENGTH));
            expected.push(Event::Float(Route::LocalSkeleton, keys::ROOT_RADIUS));
        } else {
            if index % 3 == 2 {
                expected.push(Event::Float(
                    Route::GlobalSkeleton,
                    keys::CAPSULE_LENGTH_SCALE,
                ));
            }
            expected.push(Event::Float(Route::LocalSkeleton, keys::PART_MASS_SCALE));
        }
        expected.extend([Event::Instance, Event::Mode, Event::Drop]);
    }
    assert_eq!(*reader.events.borrow(), expected);
}

#[test]
fn debug_mutates_only_animated_tensor_after_ragdoll_cache_and_keeps_inverse_mass() {
    let (data, attributes) = inputs();
    let plain = build(Arc::clone(&data), attributes.clone(), &mut Reader::new(0));
    for mode in [1, 2] {
        let mut reader = Reader::new(mode);
        let modified = build(Arc::clone(&data), attributes.clone(), &mut reader);
        assert_eq!(
            reader
                .events
                .borrow()
                .iter()
                .filter(|event| **event == Event::Factor)
                .count(),
            24
        );
        for (before, after) in plain.parts.iter().zip(&modified.parts) {
            assert_eq!(before.ragdoll, after.ragdoll);
            assert_eq!(before.animated.inverse_mass, after.animated.inverse_mass);
            if mode == 1 {
                assert_eq!(
                    after.animated.inverse_tensor,
                    before.animated.inverse_tensor.map(|v| v * 0.5)
                );
            } else {
                assert_eq!(after.animated.inverse_tensor[3].to_bits(), 0);
                assert_eq!(
                    after.animated.inverse_tensor[0],
                    after.animated.inverse_tensor[2]
                );
            }
        }
    }
}

#[test]
fn invalid_wipeout_numerator_including_nan_stops_before_any_part_reads() {
    for numerator in [0.0, -0.0, -1.0, f32::NAN] {
        let (data, attributes) = inputs();
        let mut reader = Reader::new(0);
        reader.numerator = numerator;
        let result = SkeletonBodyDefinition::try_create(
            data,
            Arc::new(attributes),
            &mut reader,
            0,
            None,
            &mut || true,
            &mut HostSkeletonBodyMassNumeric,
            &mut HostSimulationVolumeIntegralNumeric,
            &mut HostHatNumeric,
        );
        assert!(
            matches!(result, Err(SkeletonBodyDefinitionError::InvalidRagdollNumerator { bits }) if bits == numerator.to_bits())
        );
        assert_eq!(reader.events.borrow().len(), 3);
    }
}

#[test]
fn capsule_clamp_and_sphere_unordered_selection_do_not_use_generic_maximum() {
    let (_, mut attributes) = inputs();
    let bone = &mut attributes.bones[2];
    bone.volume_scalar = 1.0;
    let (_, shape) = ordinary_volume([4.0, 99.0, 1.0, 0.0], bone, 1.0, &mut Reader::new(0))
        .ok()
        .unwrap();
    assert_eq!(
        shape,
        SkeletonPartDebugShape::Capsule {
            radius: 2.0,
            half_length: 0.0
        }
    );
    assert_eq!(sphere_radius([f32::NAN, 4.0, 100.0, 0.0], 1.5), 3.0);
    assert_eq!(
        sphere_radius([-0.0, 0.0, 1.0, 0.0], 1.0).to_bits(),
        0x8000_0000
    );
}

#[test]
fn unsupported_shape_and_nonpositive_part_mass_refuse_without_tuning_fallback() {
    let (data, mut attributes) = inputs();
    attributes.bones[1].volume_type = 7;
    let result = SkeletonBodyDefinition::try_create(
        data,
        Arc::new(attributes),
        &mut Reader::new(0),
        0,
        None,
        &mut || true,
        &mut HostSkeletonBodyMassNumeric,
        &mut HostSimulationVolumeIntegralNumeric,
        &mut HostHatNumeric,
    );
    assert!(matches!(
        result,
        Err(SkeletonBodyDefinitionError::UnsupportedBoneVolume {
            part: SkeletonPartIndex::Neck1,
            volume_type: 7
        })
    ));
    let (hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
    parameters.bones[1].size[0] = 0.0;
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    let (_, attributes) = inputs();
    let result = SkeletonBodyDefinition::try_create(
        data,
        Arc::new(attributes),
        &mut Reader::new(0),
        0,
        None,
        &mut || true,
        &mut HostSkeletonBodyMassNumeric,
        &mut HostSimulationVolumeIntegralNumeric,
        &mut HostHatNumeric,
    );
    assert!(matches!(
        result,
        Err(SkeletonBodyDefinitionError::InvalidPartMass {
            part: SkeletonPartIndex::Neck1,
            bits: 0
        })
    ));
}

#[test]
fn matching_root_bone_id_not_part_ordinal_selects_every_duplicate_root() {
    let (mut hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
    hierarchy.parents.fill(-1);
    parameters.bones[23].name = parameters.bones[0].name.clone();
    parameters.bones[23].packed_name = parameters.bones[0].packed_name;
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    let (_, attributes) = inputs();
    let body = build(data, attributes, &mut Reader::new(0));
    assert_eq!(body.skateboard_part, SkeletonPartIndex::Hips);
    assert_eq!(body.parts[23].effective_mass.to_bits(), 0x3a83_126f);
    assert_eq!(body.parts[23].unscaled_part_mass.to_bits(), 0);
    let SkeletonPartGeometry::Primitive(volume) = &body.parts[23].geometry else {
        panic!("root capsule")
    };
    assert_eq!(volume.volume_type(), Some(VolumeType::Capsule));
    assert_eq!(volume.flags, 0);
}
