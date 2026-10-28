//! Shared Definition/real body-session lifecycle regressions, currently UNRUN.
use super::super::{ModelCollisionDefinition, ModelCollisionPart};
use super::*;
use crate::{
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_definition::fresh_volume, skeleton_body_mass::HostSkeletonBodyMassNumeric,
};
use skate2_formats::collision::VolumeType;

pub(crate) fn definition() -> SharedCollisionDefinition {
    ModelCollisionDefinition::shared(
        vec![ModelCollisionPart::new(fresh_volume(
            VolumeType::Box,
            0.,
            [1.0_f32.to_bits(); 3],
        ))],
        Arc::new(BitTable {
            rows: 1,
            columns: 1,
            array_size: 1,
            words: vec![0],
        }),
        false,
    )
}
pub(crate) fn create(
    def: &SharedCollisionDefinition,
    backend: &mut CharacterCollisionBackend,
    mass: f32,
) -> Result<CharacterCollisionAssembly, CharacterAssemblyError<core::convert::Infallible>> {
    CharacterCollisionAssembly::try_create(
        def,
        backend,
        10,
        &[mass],
        AssemblyPartMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 0.,
        },
        &mut HostSimulationVolumeIntegralNumeric,
        &mut HostSkeletonBodyMassNumeric,
    )
}

#[test]
fn two_bodies_share_model_inertia_but_keep_separate_pool_occurrences() {
    let def = definition();
    let mut backend = CharacterCollisionBackend::try_new(2, 0, 0).unwrap();
    let mut first = create(&def, &mut backend, 2.).unwrap();
    let binding = Arc::clone(def.lock().unwrap().binding.as_ref().unwrap());
    first.parts.attach(&mut backend, 4).unwrap();
    assert_eq!(binding.inertias[0].try_read().unwrap().inverse_mass, 0.5);
    let mut second = create(&def, &mut backend, 4.).unwrap();
    assert!(Arc::ptr_eq(
        &binding,
        def.lock().unwrap().binding.as_ref().unwrap()
    ));
    assert_eq!(binding.inertias[0].try_read().unwrap().inverse_mass, 0.25);
    second.parts.attach(&mut backend, 4).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 2);
    assert_eq!(first.group, 10);
    assert_eq!(second.part_user_data, vec![10]);
    assert!(Arc::ptr_eq(&first.definition, &second.definition));
    assert!(Arc::ptr_eq(&first.culling, &second.culling));
    first.parts.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 1);
    second.parts.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
}

#[test]
fn another_backend_cannot_rewrite_shared_definition_mass() {
    let def = definition();
    let mut first_backend = CharacterCollisionBackend::try_new(1, 0, 0).unwrap();
    let _first = create(&def, &mut first_backend, 2.).unwrap();
    let mut other_backend = CharacterCollisionBackend::try_new(1, 0, 0).unwrap();
    assert!(matches!(
        create(&def, &mut other_backend, 8.),
        Err(CharacterAssemblyError::ForeignBackend)
    ));
    let guard = def.lock().unwrap();
    assert_eq!(guard.parts[0].mass.inertia.inverse_mass, 0.5);
    assert_eq!(
        guard.binding.as_ref().unwrap().inertias[0]
            .try_read()
            .unwrap()
            .inverse_mass,
        0.5
    );
}
