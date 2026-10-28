#![expect(
    clippy::float_cmp,
    reason = "exact fixture values verify retained channels and lazy pointer sampling"
)]
use super::*;
use crate::reckoning_numeric::ReckoningNumeric;
use skate2_formats::andale::{AndaleHierarchyPart, AndaleSqt, PackedFastString};
use std::sync::Arc;
#[derive(Default)]
struct Numeric {
    estimates: Vec<f32>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2])))
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.estimates.push(v);
        Ok(v.sqrt().recip())
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(v.recip())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("read has no integer quantization")
    }
}
impl PresentationDecodeNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(a[0].mul_add(b[0], a[1].mul_add(b[1], a[2].mul_add(b[2], a[3] * b[3]))))
    }
}
#[test]
fn native_widening_observes_neighbor_sign_and_required_backing() {
    let positive = decode::translation::<()>(&[0, 0, 0, 0, 0, 0, 0, 0], 0, 0.5).unwrap();
    let negative_neighbor =
        decode::translation::<()>(&[0, 0, 0, 0, 0, 0, 0x80, 0], 0, 0.5).unwrap();
    assert_eq!(positive[2], 0.0);
    assert!(negative_neighbor[2] > 0.0);
    assert!(matches!(
        decode::translation::<()>(&[0; 6], 0, 0.5),
        Err(DecodeError::Bounds { .. })
    ));
}
#[test]
fn q3_evaluates_negative_radicand_estimate_before_zero_selection() {
    let mut numeric = Numeric::default();
    let q =
        decode::quaternion3(&[0x7f, 0xff, 0x7f, 0xff, 0x7f, 0xff, 0, 0], 0, &mut numeric).unwrap();
    assert_eq!(numeric.estimates.len(), 1);
    assert!(numeric.estimates[0] < 0.0);
    assert_eq!(q[3].to_bits(), 0);
}
fn sqt() -> AndaleSqt {
    AndaleSqt {
        scale: [2.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [5.0, 6.0, 7.0, 1.0],
    }
}
fn hierarchy() -> AndaleHierarchy {
    let names = [
        "Torso",
        "Head",
        "Arms",
        "Legs",
        "Skateboard",
        "Skateboard_Trucks",
        "LeftHand",
        "RightHand",
        "Face0",
        "Face1",
        "Face2",
    ];
    AndaleHierarchy {
        has_trajectory: false,
        parents: vec![-1; 3],
        bone_metadata: vec![0; 3],
        bone_names: vec![String::new(); 3],
        packed_bone_names: vec![PackedFastString { words: [0; 5] }; 3],
        parts: names
            .into_iter()
            .map(|name| AndaleHierarchyPart {
                name: name.to_owned(),
                packed_name: PackedFastString::encode(name).unwrap(),
                bone_count: u32::from(name == "Face0"),
                bone_offset: 0,
                is_animated: true,
                name_index: 0,
            })
            .collect(),
    }
}
struct Settings {
    hand_reads: usize,
    face_reads: usize,
}
impl PresentationReadSettings for Settings {
    type Error = &'static str;
    fn no_hand_animation(&mut self) -> Result<bool, Self::Error> {
        self.hand_reads += 1;
        Ok(false)
    }
    fn no_face_animation(&mut self) -> Result<bool, Self::Error> {
        self.face_reads += 1;
        Ok(false)
    }
}
#[test]
fn aliased_face_composes_actual_base_and_keeps_scale() {
    let h = Arc::new(hierarchy());
    let output = OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::clone(&h));
    output.extract(&[sqt(); 3]).unwrap();
    output.complete_sync().unwrap();
    let mut q = [[0.0, 0.0, 0.0, 1.0]];
    let mut t = [[1.0, 2.0, 3.0, 1.0]];
    let scratch = PresentationReadScratch {
        quaternions: &mut q,
        translations: &mut t,
    };
    let matrix = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 8.0, 9.0, 10.0, 1.0,
    ];
    let frame = PresentationReadFrame {
        read_base: &[],
        compressed_offset: 0,
        local_hips: matrix,
        local_skateboard: matrix,
    };
    let mut settings = Settings {
        hand_reads: 0,
        face_reads: 0,
    };
    let mut numeric = Numeric::default();
    output
        .try_edit_channels(&output, |edit| {
            apply_channels(
                &h,
                edit,
                &scratch,
                &frame,
                Some(1),
                Some(2),
                &mut settings,
                &mut numeric,
            )
        })
        .unwrap();
    let (_, values) = output.read().unwrap();
    assert_eq!(values[0].translation, [6.0, 8.0, 10.0, 2.0]);
    assert_eq!(values[0].scale, [2.0; 4]);
    assert_eq!(values[1].scale, [2.0; 4]);
    assert_eq!(settings.hand_reads, 0);
    assert_eq!(settings.face_reads, 1);
    assert_eq!(values[1].translation, [8.0, 9.0, 10.0, 1.0]);
}
#[test]
fn external_set_samples_retained_allocation_at_job_execution_and_supports_pending_alias() {
    use crate::{
        actor_animation_construction::{
            hands_face::sqt_jobs::{QueuedSqtProgram, SqtDispatchWork},
            initialization::bind_pose::BufferedJobPool,
        },
        animation_job_owners::{AnimationJobCode, execution::AnimationJobWork},
    };
    let (mut commands, _) = TreeCommands::fixed_test_collector();
    {
        let mut owner = commands.owner.lock().unwrap();
        owner.in_commands = 1;
        owner.in_command_set = 1;
        owner.max_set_replace_sqts = 3;
        owner.set_pool = Some(BufferedJobPool {
            jobs: vec![],
            max_dependencies: 1,
            max_work_units: 18,
            current_job: 0,
        });
    }
    let h = hierarchy();
    let output = OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::new(h.clone()));
    output.extract(&[sqt(); 3]).unwrap();
    output.complete_sync().unwrap();
    commands.set_external_sqt(&output).unwrap();
    let mut changed = sqt();
    changed.translation[0] = 42.0;
    output.extract(&[changed; 3]).unwrap();
    output.complete_sync().unwrap();
    output.begin_extract().unwrap();
    let work = SqtDispatchWork::new(vec![QueuedSqtProgram {
        commands,
        output:
            crate::actor_animation_construction::hands_face::sqt_jobs::matrix::ProgramOutput::Sqt(
                output.clone(),
            ),
        hierarchy: h.into(),
    }]);
    work.run(AnimationJobCode::SetData).unwrap();
    assert!(output.read().is_err());
    work.complete_sync().unwrap();
    assert_eq!(output.read().unwrap().1[0].translation[0], 42.0);
}
#[test]
fn live_disable_settings_are_linked_and_not_cached() {
    use crate::input_cvar_runtime::InputCvarManager;
    let refs = InputCvarReferences::new_static_initialized();
    let mut access = &refs;
    assert!(access.no_hand_animation().is_err());
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert!(!access.no_hand_animation().unwrap());
    assert!(!access.no_face_animation().unwrap());
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\ncv_NoHandAnimation=1\r\ncv_NoFaceAnimation=1\r\n",
        ))
        .unwrap();
    assert!(access.no_hand_animation().unwrap());
    assert!(access.no_face_animation().unwrap());
}
#[test]
fn external_set_checks_per_character_capacity_after_dynamic_allocation() {
    let (mut commands, _) = TreeCommands::fixed_test_collector();
    {
        let mut owner = commands.owner.lock().unwrap();
        owner.in_commands = 1;
        owner.in_command_set = 1;
        owner.max_set_replace_sqts = 1;
    }
    let source = OwnedAnimationSqtBuffer::allocate_unknown(1);
    commands.set_external_sqt(&source).unwrap();
    assert!(commands.set_external_sqt(&source).is_err());
    assert_eq!(commands.dynamic_count(), 2);
    assert_eq!(commands.external_sqts.len(), 1);
    assert_eq!(commands.owner.lock().unwrap().commands.len(), 1);
    assert_eq!(commands.output().unwrap(), 0);
}
#[test]
fn hand_gates_are_sampled_per_bone_and_later_face_failure_keeps_prefix() {
    struct Changing {
        reads: usize,
    }
    impl PresentationReadSettings for Changing {
        type Error = &'static str;
        fn no_hand_animation(&mut self) -> Result<bool, Self::Error> {
            self.reads += 1;
            Ok(self.reads == 2)
        }
        fn no_face_animation(&mut self) -> Result<bool, Self::Error> {
            Err("face cvar unavailable")
        }
    }
    let mut h = hierarchy();
    h.parts[6].bone_count = 2;
    h.parts[6].bone_offset = 1;
    let output = OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::new(h.clone()));
    output.extract(&[sqt(); 3]).unwrap();
    output.complete_sync().unwrap();
    let mut q = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut t = [[0.0; 4]];
    let scratch = PresentationReadScratch {
        quaternions: &mut q,
        translations: &mut t,
    };
    let frame = PresentationReadFrame {
        read_base: &[],
        compressed_offset: 0,
        local_hips: [0.0; 16],
        local_skateboard: [0.0; 16],
    };
    let mut settings = Changing { reads: 0 };
    let mut numeric = Numeric::default();
    let result = output.try_edit_channels(&output, |edit| {
        let result = apply_channels(
            &h,
            edit,
            &scratch,
            &frame,
            None,
            None,
            &mut settings,
            &mut numeric,
        );
        assert!(matches!(
            result,
            Err(PresentationReadError::Settings("face cvar unavailable"))
        ));
        assert_eq!(edit.destination(1).unwrap().rotation, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(edit.destination(2).unwrap().rotation, sqt().rotation);
        result
    });
    assert!(result.is_err());
    assert_eq!(settings.reads, 2);
    assert!(output.read().is_err());
}
