//! Concrete pedestrian declaration objects connected to the retained generated
//! delegate. Services borrow the actual entity's existing owners for each frame.
use super::{ActiveBehaviour, ControllerState, MotionDelegate, PedestrianMotionBackend, PedestrianMotionDelegate,
    behaviours::{self, Behaviours}, conditions::{self, Conditions}};
use crate::{generated_action_load::runtime::ActionUpdateError as Error,
    motion_graph_conditions::{HasIntentCollaborator, WillExpireCollaborator},
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken}};

pub(crate) struct MotionObjects { conditions: Conditions, behaviours: Behaviours }
impl MotionObjects {
    /// Retain once at Load; no per-frame reparse or substitute onboard graph.
    pub(crate) fn bind(delegate: MotionDelegate) -> Result<Self, String> {
        let declarations = delegate.declarations();
        Ok(Self { conditions: Conditions::bind(&declarations)?, behaviours: Behaviours::bind(&declarations)? })
    }
    /// The services adapter must borrow the SAME pedestrian as the delegate's
    /// original authority; failures retain their specific service diagnostic.
    pub(crate) fn update<K, S>(&self, delegate: &mut PedestrianMotionDelegate<K>, dt: f32,
        services: &mut S) -> Result<(), String>
    where S: conditions::Services + behaviours::Services,
          <S as HasIntentCollaborator>::Error: std::fmt::Debug,
          <S as WillExpireCollaborator>::Error: std::fmt::Debug,
    {
        let mut backend = Backend { objects: self, services, failure: None };
        delegate.update(dt, &mut backend).map_err(|error| backend.failure.take().unwrap_or_else(|| error.to_string()))
    }
}
struct Backend<'a, S> { objects: &'a MotionObjects, services: &'a mut S, failure: Option<String> }
impl<S> Backend<'_, S> {
    fn capture<T>(&mut self, result: Result<T, String>, error: Error) -> Result<T, Error> {
        result.map_err(|message| { self.failure = Some(message); error })
    }
}
impl<K, S> PedestrianMotionBackend<K> for Backend<'_, S>
where S: conditions::Services + behaviours::Services,
      <S as HasIntentCollaborator>::Error: std::fmt::Debug,
      <S as WillExpireCollaborator>::Error: std::fmt::Debug,
{
    fn condition(&mut self, _context: &BoundMotionGraphContext<K>, controller: &ControllerState<'_>,
        condition: u16, state: u16) -> Result<bool, Error>
    {
        let result = self.objects.conditions.evaluate(condition, state, controller, self.services);
        self.capture(result, Error::PendingExpression(condition))
    }
    fn enabled(&self, occurrence: u16) -> Result<bool, Error> {
        self.objects.behaviours.enabled(occurrence).map_err(|_| Error::InvalidActiveSet)
    }
    fn create_instance(&mut self, occurrence: u16) -> Result<Option<MotionGraphInstanceToken>, Error> {
        self.objects.behaviours.enabled(occurrence).map_err(|_| Error::InvalidActiveSet)?;
        Ok(None) //all14 native CreateInstance vtable slots point to82bb3da8
    }
    fn release_instance(&mut self, _occurrence: u16, instance: Option<MotionGraphInstanceToken>) -> Result<(), Error> {
        if instance.is_some() { Err(Error::InvalidActiveSet) } else { Ok(()) }
    }
    fn begin(&mut self, _context: &BoundMotionGraphContext<K>, _controller: &ControllerState<'_>,
        occurrence: u16) -> Result<(), Error>
    {
        let result = self.objects.behaviours.begin(occurrence, self.services);
        self.capture(result, Error::InvalidActiveSet)
    }
    fn end(&mut self, _context: &BoundMotionGraphContext<K>, _controller: &ControllerState<'_>,
        occurrence: u16) -> Result<(), Error>
    {
        let result = self.objects.behaviours.end(occurrence, self.services);
        self.capture(result, Error::InvalidActiveSet)
    }
    fn generated_update(&mut self, context: &mut BoundMotionGraphContext<K>, _controller: &ControllerState<'_>,
        source: u32, active: &[ActiveBehaviour]) -> Result<(), Error>
    {
        let result = self.objects.behaviours.generated_update(source, context, active, self.services);
        self.capture(result, Error::PendingStateUpdate(source))
    }
}
