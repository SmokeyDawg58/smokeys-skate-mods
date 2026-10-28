//! InitModels82e78580 and Collideable.Init82e605a8. No roster publication.
use super::PedestrianConstruction;
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    living_world_grind_services::{collideable_objects::CollideableObjectMan, dmo_models::DmoModelMan},
    living_world_pedestrian_memory::PedestrianMemory,
    pedestrian_collision::PedestrianCollisionBody,
    physical_player_assembly::PhysicalAssemblyWorld,
};
use std::sync::Arc;

impl PedestrianConstruction {
    /// LWBasicEntity.LoadAIGraph82e4c340 and generated factory82951770.
    /// Initial load uses the shared preload enablement and exact native path
    /// selector. Reload of a running controller needs End/destruction first.
    /// The factory constructs HCDelegate_Pedestrian, HCDelegate_marquee or
    /// HCDelegate_Photographer by the path's basename.
    pub(crate) fn load_graph(&mut self,
        graphs: &crate::actor_construction::graph_startup::ActorGraphStartupOwners,
        path: &[u8],
    ) -> Result<(), String> {
        use crate::generated_onboard_state_graph::{select_onboard_controller, OnboardControllerSelection};
        use crate::pedestrian_graph::AiDelegate;
        if self.ai_bindings.is_some() {
            return Err("pedestrian AI reload requires controller teardown".into());
        }
        let enabled = graphs.living_world_enablement().map_err(|e| format!("pedestrian AI preload: {e:?}"))?;
        let selected = select_onboard_controller(path, true, false, enabled).map_err(|e| e.to_string())?;
        let delegate = match selected {
            OnboardControllerSelection::GeneratedOther { delegate_index } => AiDelegate::from_delegate_index(delegate_index),
            _ => None,
        }.ok_or_else(|| format!("pedestrian AI route not implemented: {selected:?}"))?;
        let behaviours = crate::pedestrian_graph::behaviour_binding::LocalBehaviourBindings::for_delegate(delegate)?;
        // Generated constructor binds behaviours before delegate.Init.
        let conditions = match delegate {
            AiDelegate::Pedestrian => graphs.pedestrian_conditions()?,
            delegate => Arc::new(crate::pedestrian_graph::condition_binding::PedestrianConditions::for_delegate(delegate)?),
        };
        self.graph = crate::pedestrian_graph::PedestrianGraph::default();
        self.behaviours = crate::pedestrian_graph::PedestrianGraphBehaviours::default();
        self.ai_bindings = Some(crate::pedestrian_graph::PedestrianAiBindings {
            delegate, path: path.to_vec(), conditions, behaviours,
            // Load82cef218 hashes GetRootState()->GetStateName(). Root is
            // index0 (82933b10), named AI, not the Pedestrian child/filename.
            root_name_hash: skate2_formats::hash::sk8_hash_value_32(delegate.states()[0].name.as_bytes()),
        });
        Ok(())
    }
    /// `bind_graph` must destroy/load the actual AI controller using the
    /// original path and current preload delegate setting. It is fallible;
    /// constructing a default graph is not a successful load.
    /// Returns the native model-null result so Spawn can destroy the retained
    /// entity. Even that branch loads its AI graph before Spawn inspects it.
    pub(crate) fn initialize_models<F>(
        &mut self, memory: &mut PedestrianMemory, recipe: u64,
        vault: &Arc<AttributeRuntimeVault>, models: &DmoModelMan,
        assemblies: &mut CollideableObjectMan, world: &mut PhysicalAssemblyWorld,
        bind_graph: F,
    ) -> Result<bool, String>
    where F: FnOnce(&mut Self, &[u8]) -> Result<(), String>,
    {
        if self.collision.is_some() {
            return Err("pedestrian InitModels already attempted".into());
        }
        self.initialize_model_identity(recipe, vault)?;
        let model = models.character_model(recipe).cloned();
        self.create_collision(memory, vault)?;
        let collision = self.collision.as_mut().expect("collision retained");
        collision.initialize_bodies(model.clone(), self.id, world)?;
        // Raw PPC82e60624/658 selects vtable+20/+28: world then skater.
        // Register detached native Assemblies before their Island admission.
        for body in [&collision.world_body, &collision.skater_body] {
            let handle = match body {
                Some(PedestrianCollisionBody::Constructed(candidate)) => Some(candidate.handle()),
                Some(PedestrianCollisionBody::InWorld(handle)) => Some(handle),
                None => None,
            };
            if let Some(handle) = handle {
                assemblies.register(handle, self.id)
                    .map_err(|e| format!("pedestrian collision ownership: {e:?}"))?;
            }
        }
        let has_model = model.is_some();
        self.model = model;
        let path = self.resources.as_ref().ok_or("pedestrian resources absent")?
            .entity.try_get_text(0xca4d_0ad4_ea45_965b, 0)
            .map_err(|e| e.to_string())?.ok_or("pedestrian AI graph filename absent")?
            .bytes().to_vec();
        bind_graph(self, &path)?;
        Ok(has_model)
    }
}

/// StateGraph::Factory::Create82951424 uses '/' preferentially, falling back
/// to '\\' only if no '/' exists. Generated names are case-sensitive. Keep
/// the original full path available to the real cache/dynamic graph loader.
pub(crate) fn graph_basename(path: &[u8]) -> &[u8] {
    let start = path.iter().rposition(|c| *c == b'/')
        .or_else(|| path.iter().rposition(|c| *c == b'\\')).map_or(0, |i| i + 1);
    &path[start..]
}
