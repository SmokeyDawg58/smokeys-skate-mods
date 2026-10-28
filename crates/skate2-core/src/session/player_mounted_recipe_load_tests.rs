use super::*;
#[test]
fn local_runtime_recipe_overrides_serialized_category_and_clone_before_publication() {
    let recipe = MountedPlayerRecipeLoad::retain_runtime_recipe(CharacterRecipe {
        version: 7,
        schema_name: "fixture".into(),
        category: 9,
        memory_category: 17,
        clone_type: 2,
        components: vec![],
        has_customizations: false,
        customizations: None,
        packed_size: 23,
    });
    assert_eq!(recipe.category, 0);
    assert_eq!(recipe.clone_type, 3);
    assert_eq!(recipe.memory_category, 17);
    assert_eq!(recipe.packed_size, 23);
    let mut registry = MountedRecipeRegistry::construct_fields();
    registry.publish_parsed(42, Arc::clone(&recipe)).unwrap();
    let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get(&42) else {
        panic!("runtime recipe published")
    };
    assert!(Arc::ptr_eq(&entry.recipe, &recipe));
    assert_eq!((entry.recipe.category, entry.recipe.clone_type), (0, 3));
}
#[test]
fn session_registry_retains_parsed_prefix_and_rejects_real_occupants() {
    let mut registry = MountedRecipeRegistry::construct_fields();
    let recipe = Arc::new(CharacterRecipe {
        version: 7,
        schema_name: "fixture".into(),
        category: 0,
        memory_category: 0,
        clone_type: 3,
        components: vec![],
        has_customizations: false,
        customizations: None,
        packed_size: 0,
    });
    let id = 0x1234_5678_1111_2222;
    registry.require_miss(id).unwrap();
    registry.publish_parsed(id, Arc::clone(&recipe)).unwrap();
    let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get(&id) else {
        panic!("published parsed recipe")
    };
    assert!(Arc::ptr_eq(&entry.recipe, &recipe));
    assert!(entry.scene.is_none());
    assert!(Arc::ptr_eq(&registry.tracked[0], &recipe));
    assert!(
        matches!(registry.require_miss(id),Err(MountedRecipeError::RecipeCacheHitPending(actual)) if actual==id)
    );
    registry
        .retain_other_asset(0xabcd_9876_1111_2222, 0xeeee)
        .unwrap();
    assert!(matches!(
        registry.require_miss(0xabcd_9876_1111_2222),
        Err(MountedRecipeError::RegistryAssetType {
            asset_type: 0xeeee,
            ..
        })
    ));
    assert!(
        matches!(registry.retain_other_asset(id,0xffff),Err(MountedRecipeError::RegistryOccupied(actual)) if actual==id)
    );
    let scene = Arc::new(CharacterScene {
        customizations: None, schema_name: "fixture".into(),
        category: 0,
        components: vec![],
        textures: vec![],
    });
    assert!(
        matches!(registry.retain_scene(id,&Arc::new((*recipe).clone()),Arc::clone(&scene)),Err(MountedRecipeError::RegistryIdentity(actual)) if actual==id)
    );
    registry
        .retain_scene(id, &recipe, Arc::clone(&scene))
        .unwrap();
    let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get(&id) else {
        panic!("same registry entry")
    };
    assert!(Arc::ptr_eq(entry.scene.as_ref().unwrap(), &scene));
}
/// `RequestState_LoadSchemaFromFile`'s cache hit: a second skater load of a registered schema
/// shares its recipe and scene and holds a reference; a LivingWorld entry is not shared.
#[test]
fn registered_skater_schema_is_shared_with_a_reference() {
    let mut registry = MountedRecipeRegistry::construct_fields();
    let recipe = Arc::new(CharacterRecipe {
        version: 7,
        schema_name: "fixture".into(),
        category: 0,
        memory_category: 0,
        clone_type: 3,
        components: vec![],
        has_customizations: false,
        customizations: None,
        packed_size: 0,
    });
    let scene = Arc::new(CharacterScene {
        customizations: None,
        schema_name: "fixture".into(),
        category: 0,
        components: vec![],
        textures: vec![],
    });
    let id = 0x1234_5678_1111_2222;
    assert!(registry.try_share(id).unwrap().is_none());
    registry.publish_parsed(id, Arc::clone(&recipe)).unwrap();
    assert!(matches!(
        registry.try_share(id),
        Err(MountedRecipeError::Missing("registered scene"))
    ));
    registry
        .retain_scene(id, &recipe, Arc::clone(&scene))
        .unwrap();
    let (shared_recipe, shared_scene) = registry.try_share(id).unwrap().unwrap();
    assert!(Arc::ptr_eq(&shared_recipe, &recipe));
    assert!(Arc::ptr_eq(&shared_scene, &scene));
    let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get_mut(&id) else {
        panic!("shared registry entry")
    };
    assert_eq!(entry.references, 2);
    assert_eq!(registry.tracked.len(), 1);
    let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get_mut(&id) else {
        panic!("shared registry entry")
    };
    entry.ambient_assets = Some(Arc::new(ambient::AmbientRecipeAssets {
        finalized: Arc::new(FinalizedRecipeScene::new(Arc::clone(&scene))),
        collision: BTreeMap::new(),
        embedded_splines: BTreeMap::new(),
    }));
    assert!(matches!(
        registry.try_share(id),
        Err(MountedRecipeError::RecipeCacheHitPending(actual)) if actual == id
    ));
}
#[test]
fn real_face_owner_completion_routes_exact_retained_recipe_identity() {
    let mut load = MountedPlayerRecipeLoad::new();
    load.asset_id = Some(0xfedc_ba98_1234_5678);
    load.phase = MountedRecipePhase::WaitingFace;
    let callback = u64::try_from(Arc::as_ptr(&load.callback) as usize).unwrap();
    let mut faces = CharacterFaceManager::new();
    let ticket = faces
        .begin_load(b"fixture", load.asset_id.unwrap(), callback)
        .load
        .unwrap();
    let mut delivered = false;
    faces
        .dispatch_finished::<MountedRecipeError>(|_| {
            delivered = true;
            Ok(())
        })
        .unwrap();
    assert!(!delivered);
    assert_eq!(load.phase, MountedRecipePhase::WaitingFace);
    assert!(matches!(
        load.face_callback(7, callback),
        Err(MountedRecipeError::FaceCallbackMismatch)
    ));
    assert_eq!(load.phase, MountedRecipePhase::WaitingFace);
    faces
        .complete_load(&ticket, true, vec![1, 2, 3].into_boxed_slice())
        .unwrap();
    faces
        .dispatch_finished(|request| load.face_callback(request.asset_id, request.callback))
        .unwrap();
    assert_eq!(load.phase, MountedRecipePhase::Loaded);
    assert!(faces.has_face_data(b"fixture"));
    assert!(matches!(
        load.face_callback(load.asset_id.unwrap(), callback),
        Err(MountedRecipeError::FaceCallbackMismatch)
    ));
}
