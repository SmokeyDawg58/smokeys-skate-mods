//! Original-data assertions around the retained production presentation runtime.
use super::selected_runtime::{
    SelectedPresentationPolicy, SelectedPresentationRuntime, SelectedPresentationStage,
};
use super::*;
use crate::{
    actor_construction::{ActorConstruction, body_session::ActorBodyAnimationServices},
    actor_face_playback::presentation::startup::SelectedPresentationStartup,
    input_cvar_runtime::InputCvarReferences,
    player::attributes_runtime::PlayerAttributeServices,
};
pub(crate) struct SelectedFrameProbe {
    runtime: SelectedPresentationRuntime,
}
impl SelectedFrameProbe {
    pub(crate) fn new(
        presentation: &SelectedPresentationStartup,
        scene: std::sync::Arc<crate::character::CharacterScene>,
    ) -> Self {
        // Explicit original-data fixture admission, never production defaults.
        Self {
            runtime: SelectedPresentationRuntime::try_new(
                presentation,
                scene,
                SelectedPresentationPolicy {
                    lod: 0,
                    submission_capacity: 65536,
                    gather_capacity: 65536,
                    provider: 0,
                    root_w: RootPositionWTransport::ReadSkaterOpaque { bits: 0 },
                },
            )
            .unwrap(),
        }
    }
    pub(crate) fn runtime_mut(&mut self) -> &mut SelectedPresentationRuntime {
        &mut self.runtime
    }
    pub(crate) fn check(
        &mut self,
        actor: &ActorConstruction,
        services: &PlayerAttributeServices,
        presentation: &SelectedPresentationStartup,
        cvars: &InputCvarReferences,
        jobs: &mut ActorBodyAnimationServices,
    ) {
        self.check_with(
            actor,
            services,
            presentation,
            cvars,
            true,
            |runtime, numeric, observer| {
                runtime.try_update(
                    presentation,
                    ActorPresentationFrameSource { actor, services },
                    &mut &*cvars,
                    numeric,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                    jobs,
                    Some(observer),
                )
            },
        )
        .unwrap();
    }
    #[expect(
        clippy::too_many_lines,
        reason = "retain original assertions at each production publication boundary"
    )]
    pub(crate) fn check_with<T, E>(
        &mut self,
        actor: &ActorConstruction,
        services: &PlayerAttributeServices,
        presentation: &SelectedPresentationStartup,
        cvars: &InputCvarReferences,
        bind_before_update: bool,
        update: impl FnOnce(
            &mut SelectedPresentationRuntime,
            &mut super::super::corrections::corpus::Numeric,
            &mut dyn FnMut(
                SelectedPresentationStage,
                &SelectedPresentationRuntime,
                &SubmittedPresentationFrame,
            ) -> Result<(), std::convert::Infallible>,
        ) -> Result<T, E>,
    ) -> Result<T, E> {
        if bind_before_update && self.runtime.materials().is_err() {
            self.runtime
                .try_bind_materials(
                    presentation
                        .material_source
                        .as_ref()
                        .expect("same finalized recipe owner"),
                    std::sync::Arc::clone(&services.vault),
                    cvars.clone(),
                )
                .unwrap();
        }
        let restored = presentation.restored.as_ref().unwrap();
        let mut base = None;
        let mut stages = Vec::new();
        let lengths = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut observer =
            |stage, runtime: &SelectedPresentationRuntime, frame: &SubmittedPresentationFrame| {
                stages.push(stage);
                match stage {
                    SelectedPresentationStage::Produced => {
                        let root = runtime.producer().root_position(frame, 0).unwrap();
                        let actual = ActorPresentationFrameSource { actor, services }
                            .root_matrix()
                            .unwrap();
                        assert_eq!(
                            root.w.map(f32::to_bits),
                            actual.columns()[3].w.map(f32::to_bits)
                        );
                        assert_eq!(
                            runtime
                                .producer()
                                .read_frame(frame, 0)
                                .unwrap()
                                .compressed_offset,
                            208
                        );
                        base = Some(restored.next.read().unwrap().1);
                    }
                    SelectedPresentationStage::Decoded => {
                        let output = restored.previous.read().unwrap().1;
                        let base = base.as_ref().unwrap();
                        assert_eq!(base.len(), output.len());
                        assert!(!output.is_empty());
                        assert_eq!(
                            &restored.next.read().unwrap().1,
                            base,
                            "authored odd/base remains the same source allocation"
                        );
                        assert!(
                            output.iter().all(|sqt| sqt
                                .rotation
                                .iter()
                                .chain(&sqt.translation)
                                .all(|v| v.is_finite())),
                            "actual compressed frame reconstructs finite full-rig channels"
                        );
                    }
                    SelectedPresentationStage::Corrected => {
                        assert_eq!(lengths.borrow().len(), 8);
                        super::super::corrections::corpus::check_corrected(presentation);
                    }
                    SelectedPresentationStage::Finalized => {
                        let matrices = presentation.animation.interpolated[0]
                            .matrices()
                            .unwrap()
                            .unwrap();
                        assert!(matrices.iter().flatten().all(|v| v.is_finite()));
                        assert_eq!(
                            matrices[0].map(f32::to_bits),
                            presentation.animation.hips_transforms.lock().unwrap()[0]
                                .unwrap()
                                .map(f32::to_bits)
                        );
                    }
                    SelectedPresentationStage::Published => {
                        let palette = runtime.palette().unwrap();
                        let mut meshes = 0;
                        for (part_index, part) in palette.skinning().parts.iter().enumerate() {
                            let lod = &palette.scene().components[part.component_index].models
                                [part.model_index]
                                .lods[part.lod_index];
                            let model = lod
                                .arena
                                .instances
                                .iter()
                                .find_map(|i| i.render_model)
                                .unwrap();
                            for mesh in 0..lod.arena.models[model].meshes.len() {
                                let publication = palette.mesh_rows(part_index, mesh).unwrap();
                                assert!(publication.rows.iter().flatten().all(|v| v.is_finite()));
                                assert!(publication.generation > 0);
                                meshes += 1;
                            }
                        }
                        assert!(meshes > 0);
                    }
                }
                Ok::<(), std::convert::Infallible>(())
            };
        let mut numeric = super::super::corrections::corpus::Numeric {
            lengths: std::rc::Rc::clone(&lengths),
        };
        let result = update(&mut self.runtime, &mut numeric, &mut observer);
        eprintln!(
            "actual original twist transverse lengths (source,destination,bits): {:x?}",
            numeric.lengths
        );
        let value = result?;
        let materials = self.runtime.materials().unwrap().lock().unwrap();
        let palette = self.runtime.palette().unwrap();
        for (part, mesh) in materials.meshes() {
            let view = materials.view(palette, part, mesh).unwrap();
            assert!(
                palette
                    .scene()
                    .textures
                    .iter()
                    .any(|t| std::ptr::eq(&raw const t.texture, view.texture))
            );
            assert!(matches!(
                view.effect,
                "defaultcharacter" | "defaulthair" | "defaultcharacter_ropa"
            ));
            assert_eq!(view.alpha.is_some(), view.effect == "defaulthair");
            if view.effect == "defaultcharacter_ropa" {
                assert_eq!(
                    view.ropa_vertex_params_x.unwrap().to_bits(),
                    1_f32.to_bits()
                );
            }
        }
        assert_eq!(
            stages,
            [
                SelectedPresentationStage::Produced,
                SelectedPresentationStage::Decoded,
                SelectedPresentationStage::Corrected,
                SelectedPresentationStage::Finalized,
                SelectedPresentationStage::Published
            ]
        );
        eprintln!(
            "actual selected Actor -> native WriteData/ReadSkater -> MoveRoot/Expressions -> final ACS -> supplied model palette; selected same-frame host admission"
        );
        Ok(value)
    }
}
