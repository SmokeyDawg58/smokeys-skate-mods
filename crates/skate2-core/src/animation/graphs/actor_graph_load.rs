//! Selected wrapper Load82cef180, with real generated Action construction.
use super::{ActorGraphAllocation, ActorHierarchicalWrapper};
use crate::{
    generated_action_load::{
        GeneratedActionController, GeneratedActionFactories, GeneratedActionLoadError,
        GeneratedActionStatics,
    },
    generated_onboard_state_graph::{
        GeneratedOnboardEnablement, GeneratedOnboardKind, OnboardControllerSelection,
        select_onboard_controller,
    },
};

#[derive(Debug)]
pub(crate) enum ActorGraphLoadError {
    Motion(crate::generated_motion_load::MotionLoadError),
    UnsupportedSelectedController(OnboardControllerSelection),
    Generated(GeneratedActionLoadError),
}
impl std::fmt::Display for ActorGraphLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Motion(error) => write!(f, "generated Motion load: {error}"),
            Self::UnsupportedSelectedController(kind) => {
                write!(f, "selected graph loader pending: {kind:?}")
            }
            Self::Generated(error) => write!(f, "generated Action load: {error}"),
        }
    }
}
impl<K> ActorHierarchicalWrapper<K> {
    /// Enable bytes come from the actual retained boot owner. Native ignores
    /// its allowGenerated parameter here; Create always gets true,false,true.
    /// Only currently constructible generated Action is admitted. Selected
    /// dynamic/Motion routes remain checked rather than silently redirected.
    pub(crate) fn try_load(
        &mut self,
        filename: &[u8],
        enablement: GeneratedOnboardEnablement,
        statics: &GeneratedActionStatics,
        factories: &GeneratedActionFactories,
        allocation: ActorGraphAllocation,
    ) -> Result<bool, ActorGraphLoadError> {
        // No instance can yet exist in this loader; releasing these dormant
        // controller owners is its destructor path. Shared statics stay alive.
        self.abstract_controller = None;
        let selected = select_onboard_controller(filename, true, false, enablement)
            .expect("generated not required in native wrapper Load");
        if selected != OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Action) {
            return Err(ActorGraphLoadError::UnsupportedSelectedController(selected));
        }
        if allocation == ActorGraphAllocation::Null {
            return Ok(false);
        }
        let controller =
            GeneratedActionController::try_load(&self.context, statics, factories, true)
                .map_err(ActorGraphLoadError::Generated)?;
        let root_name = controller.root_state().name;
        self.abstract_controller = Some(super::RetainedHierarchicalController::Action(
            std::sync::Arc::new(std::sync::Mutex::new(controller)),
        ));
        self.root_name_hash = skate2_formats::hash::sk8_hash_value_32(root_name.as_bytes());
        Ok(true)
    }
    pub(crate) fn generated_action(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<GeneratedActionController<K>>>> {
        match self.abstract_controller.as_ref() {
            Some(super::RetainedHierarchicalController::Action(controller)) => {
                Some(std::sync::Arc::clone(controller))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion_graph_entity_authority::ActorGraphAuthority;
    #[test]
    fn same_wrapper_load_publishes_hash_and_null_reload_keeps_hash_and_statics() {
        let authority = ActorGraphAuthority::new(7_u32);
        let mut wrapper = ActorHierarchicalWrapper::new(&authority.entity_handle());
        let statics = GeneratedActionStatics::new();
        let mut factories = GeneratedActionFactories::new();
        factories.register_skater_classes().unwrap();
        let enabled = GeneratedOnboardEnablement::onboard(1, 0);
        assert!(
            wrapper
                .try_load(
                    b"state/ActionGraph_OnBoard.xml",
                    enabled,
                    &statics,
                    &factories,
                    ActorGraphAllocation::Host
                )
                .unwrap()
        );
        wrapper
            .generated_action()
            .unwrap()
            .lock()
            .unwrap()
            .validate_context(wrapper.context())
            .unwrap();
        let hash = wrapper.root_name_hash();
        assert_eq!(hash, skate2_formats::hash::sk8_hash_value_32(b"Action"));
        let first = statics.behaviour(0).unwrap().unwrap();
        assert!(
            !wrapper
                .try_load(
                    b"state/ActionGraph_OnBoard.xml",
                    enabled,
                    &statics,
                    &factories,
                    ActorGraphAllocation::Null
                )
                .unwrap()
        );
        assert!(wrapper.is_unloaded());
        assert_eq!(wrapper.root_name_hash(), hash);
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &statics.behaviour(0).unwrap().unwrap()
        ));
        assert!(matches!(
            wrapper.try_load(
                b"state/ActionGraph_OnBoard.xml",
                GeneratedOnboardEnablement::onboard(0, 0),
                &statics,
                &factories,
                ActorGraphAllocation::Host
            ),
            Err(ActorGraphLoadError::UnsupportedSelectedController(
                OnboardControllerSelection::DynamicFallback
            ))
        ));
        assert_eq!(wrapper.root_name_hash(), hash);
    }
}
