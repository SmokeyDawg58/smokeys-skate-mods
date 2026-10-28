use super::*;
use crate::{animation_job_owners::AnimationJobManager, input_cvar_runtime::InputCvarManager};

fn recipe(customizations: bool) -> CharacterRecipe {
    CharacterRecipe {
        version: 7,
        schema_name: "test".to_owned(),
        category: 3,
        memory_category: 0,
        clone_type: 0,
        components: vec![],
        has_customizations: customizations,
        customizations: None,
        packed_size: 0,
    }
}
#[test]
fn restore_samples_live_cvar_and_only_actual_null_customizations_produce_male() {
    let cvars = InputCvarReferences::new_static_initialized();
    let mut source = PresentationRestoreServices(&cvars);
    assert!(matches!(
        source.individual_full_bind_pose(),
        Err(PresentationRestoreServiceError::Cvar(_))
    ));
    let mut manager = InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\ncv_IndividualFullbindpose=0\r\n"))
        .unwrap();
    assert!(!source.individual_full_bind_pose().unwrap());
    manager
        .try_deserialize_startup(Some(b"[Config]\r\ncv_IndividualFullbindpose=1\r\n"))
        .unwrap();
    assert!(source.individual_full_bind_pose().unwrap());
    assert!(source.is_male(&recipe(false)).unwrap());
    assert_eq!(
        source.is_male(&recipe(true)),
        Err(PresentationRestoreServiceError::CustomizationsOwnerRequired)
    );
}
#[test]
fn presentation_uses_distinct_registries_and_the_same_global_database_owner() {
    let simulation = FaceAnimationManagers::new();
    let presentation = FaceAnimationManagers::new_with_database(Arc::clone(&simulation.database));
    assert!(Arc::ptr_eq(&simulation.database, &presentation.database));
    assert!(!Arc::ptr_eq(&simulation.command, &presentation.command));
    assert!(!Arc::ptr_eq(&simulation.playback, &presentation.playback));
}
struct MissingData;
impl AnimationStartupSource for MissingData {
    type Error = &'static str;
    fn read_animation(&mut self, path: &str) -> Result<Box<[u8]>, Self::Error> {
        assert_eq!(path, "data\\anim/SkaterPresentationDB.abin");
        Err("mounted data missing")
    }
    fn current_thread_id(&mut self) -> Result<u64, Self::Error> {
        panic!("no new attribute pool")
    }
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error> {
        panic!("presentation is authored RAW")
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        panic!("load must fail first")
    }
}
impl ActorAnimationInitializeSource for MissingData {
    type Error = &'static str;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        panic!("no dispatch")
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        panic!("no dispatch")
    }
}
#[test]
fn first_mount_failure_retains_database_slot_and_prevents_constructor_retry() {
    let mut startup = SelectedPresentationStartup::new(FaceAnimationManagers::new());
    let cvars = InputCvarReferences::new_static_initialized();
    let result = startup.try_initialize_and_restore(
        Arc::new(recipe(false)),
        &mut MissingData,
        &cvars,
        &mut MissingData,
    );
    assert!(matches!(
        result,
        Err(PresentationStartupError::Initialize(
            PresentationAnimationError::Source("mounted data missing")
        ))
    ));
    assert_eq!(startup.animation.databases.len(), 1);
    assert!(!startup.animation.initialized);
    assert!(startup.restored.is_none());
    let retained = startup.animation.databases.first().unwrap();
    let slot = usize::from(retained.lock().unwrap().slot);
    assert!(Arc::ptr_eq(
        retained,
        startup.animation.managers.database.lock().unwrap().slots[slot]
            .as_ref()
            .unwrap()
    ));
    assert!(matches!(
        startup.try_initialize_and_restore(
            Arc::new(recipe(false)),
            &mut MissingData,
            &cvars,
            &mut MissingData
        ),
        Err(PresentationStartupError::Initialize(
            PresentationAnimationError::AlreadyAttempted
        ))
    ));
}

#[test]
fn twist_lookup_preserves_native_missing_name_sentinel() {
    use skate2_formats::andale::{AndaleHierarchy, PackedFastString};
    let hierarchy = AndaleHierarchy {
        has_trajectory: false,
        parents: vec![-1],
        bone_metadata: vec![0],
        parts: vec![],
        bone_names: vec!["LeftArm".to_owned()],
        packed_bone_names: vec![PackedFastString::encode("LeftArm").unwrap()],
    };
    assert_eq!(super::super::native_bone(&hierarchy, "LeftArm").unwrap(), 0);
    assert_eq!(
        super::super::native_bone(&hierarchy, "LeftArmTwist").unwrap(),
        -1
    );
}
