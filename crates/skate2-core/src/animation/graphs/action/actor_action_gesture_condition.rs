//! Actual retained Action condition `HasGestureIntent82cc1870`.
use super::{
    ActorConstruction, ActorIdentity, Arc, BoundMotionGraphContext, GestureGroup,
    GestureRuntimeError, GestureTrickRuntimeServices, GestureTrickSource, LiveGestureMappingDebug,
    Source,
};

impl ActorConstruction {
    /// Resolve Action and its map before requiring the manager mapping. The
    /// native condition passes a NULL pair destination to the shared lookup.
    pub(crate) fn action_runtime_has_gesture(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        group: GestureGroup,
        services: Option<&Arc<GestureTrickRuntimeServices>>,
    ) -> Result<bool, GestureRuntimeError> {
        self.check_action_runtime_context(context)
            .map_err(GestureRuntimeError::Intent)?;
        self.runtime_has_gesture_from_checked_context(group, services)
    }

    /// Motion's shared HasGestureIntent condition receives the Motion controller
    /// context. Its caller validates that context before reading the same Action
    /// graph and manager mapping used by the Action condition.
    pub(crate) fn runtime_has_gesture_from_checked_context(
        &self,
        group: GestureGroup,
        services: Option<&Arc<GestureTrickRuntimeServices>>,
    ) -> Result<bool, GestureRuntimeError> {
        let mut source = Source {
            actor: self,
            services,
        };
        let action = source.resolve_action()?;
        let intents = source.action_map(&action)?;
        let mapping = source.mapping()?;
        let mut debug = LiveGestureMappingDebug {
            switch: || mapping.cvars.try_gesture_mapping_debug(),
            overlay: &mapping.overlay,
            position: &mapping.position,
        };
        source.action_read(&intents, |intents| {
            mapping
                .mapping
                .try_gesture_intents(intents, group, None, &mut debug)
                .map_err(GestureRuntimeError::Mapping)
        })
    }
}
