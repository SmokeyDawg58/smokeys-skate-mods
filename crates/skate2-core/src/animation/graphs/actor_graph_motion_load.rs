//! Generated Motion route of wrapper Load82cef180 and `InitPlayer`826000b0.
use super::load::ActorGraphLoadError;
use super::{ActorGraphAllocation, ActorHierarchicalWrapper, RetainedHierarchicalController};
use crate::{
    generated_motion_load::{GeneratedMotionStatics, controller::GeneratedMotionController},
    generated_onboard_state_graph::{
        GeneratedOnboardEnablement, GeneratedOnboardKind, OnboardControllerSelection,
        select_onboard_controller,
    },
};

impl<K> ActorHierarchicalWrapper<K> {
    /// Like the Action route, release the dormant previous controller first;
    /// a failed or NULL Create leaves its existing root-name hash unchanged.
    pub(crate) fn try_load_motion(
        &mut self,
        filename: &[u8],
        enablement: GeneratedOnboardEnablement,
        statics: &GeneratedMotionStatics,
        allocation: ActorGraphAllocation,
    ) -> Result<bool, ActorGraphLoadError> {
        self.abstract_controller = None;
        let selected = select_onboard_controller(filename, true, false, enablement)
            .expect("native wrapper does not require generated selection");
        if selected != OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Motion) {
            return Err(ActorGraphLoadError::UnsupportedSelectedController(selected));
        }
        if allocation == ActorGraphAllocation::Null {
            return Ok(false);
        }
        let controller = GeneratedMotionController::try_load(&self.context, statics, true)
            .map_err(ActorGraphLoadError::Motion)?;
        let root = controller.root_state().name;
        self.abstract_controller = Some(RetainedHierarchicalController::Motion(
            std::sync::Arc::new(std::sync::Mutex::new(controller)),
        ));
        self.root_name_hash = skate2_formats::hash::sk8_hash_value_32(root.as_bytes());
        Ok(true)
    }

    pub(crate) fn generated_motion(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<GeneratedMotionController<K>>>> {
        match self.abstract_controller.as_ref() {
            Some(RetainedHierarchicalController::Motion(controller)) => {
                Some(std::sync::Arc::clone(controller))
            }
            _ => None,
        }
    }
}
