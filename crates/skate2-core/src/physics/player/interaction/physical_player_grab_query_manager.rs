//! World-owned `GrabSplineQueryManager`827afa90, initialized at827b1268.
use super::{GrabConstructorError, GrabQueryFuture};
use crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority;
use crate::{
    living_world_grind_services::grab_provider::GrabBoxQuery,
    physical_player_grind_manager::{math::GrindNum, view::GrindError},
};
use std::sync::{Arc, Mutex};

/// Box arm of GrabSplineInternalQuery: queryType, max, position, box,
/// end exclusion and both angle limits. Radius queries are a separate arm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeferredGrabBoxQuery {
    pub(crate) query_type: u32,
    pub(crate) maximum: i32,
    pub(crate) geometry: GrabBoxQuery,
}
struct DeferredQuery {
    output: Arc<Mutex<GrabQueryFuture>>,
    request: DeferredGrabQuery,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum DeferredGrabQuery {
    Box(DeferredGrabBoxQuery),
    Radius {
        query_type: u32,
        maximum: i32,
        position: [f32; 4],
        radius: f32,
    },
}
impl DeferredGrabQuery {
    pub(crate) fn query_type(&self) -> u32 {
        match self {
            Self::Box(q) => q.query_type,
            Self::Radius { query_type, .. } => *query_type,
        }
    }
    fn position(&self) -> [f32; 4] {
        match self {
            Self::Box(q) => q.geometry.reach_start,
            Self::Radius { position, .. } => *position,
        }
    }
    fn maximum(&self) -> i32 {
        match self {
            Self::Box(q) => q.maximum,
            Self::Radius { maximum, .. } => *maximum,
        }
    }
}
#[cfg(test)]
impl DeferredQuery {
    fn identity_fixture(output: Arc<Mutex<GrabQueryFuture>>) -> Self {
        Self {
            output,
            request: DeferredGrabQuery::Box(DeferredGrabBoxQuery {
                query_type: 0,
                maximum: 0,
                geometry: GrabBoxQuery {
                    reach_start: [0.0; 4],
                    bounds: super::geometry::GrabBox {
                        transform: [[0.0; 4]; 4],
                        half_dimensions: [0.0; 4],
                    },
                    end_exclusion: 0.0,
                    angle_limit: 0.0,
                    horizontal_limit: 0.0,
                },
            }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum GrabMessageKind {
    Activate,
    Freeze,
    ApplyForce {
        force: crate::skeleton_root_transforms::SkeletonRootVector,
        position: crate::skeleton_root_transforms::SkeletonRootVector,
        torque: crate::skeleton_root_transforms::SkeletonRootVector,
    },
    NotifyGrabbing,
    NotifyLocalPlayer,
}

/// Native descriptor at +0 and message kind at +40 are separate fields.
/// In particular ApplyForceDeferred does NOT change descriptor.type to 3:
/// raw827b09b0/0a08 copies both descriptor words, 0a24 stores message type.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrabMessage {
    pub(crate) descriptor: crate::physical_player_assembly::remaining_states::GrabDescriptor,
    pub(crate) kind: GrabMessageKind,
}
#[path = "physical_player_grab_interactable.rs"]
mod interactable;
struct QueueState {
    queries: Vec<DeferredQuery>,
    messages: Vec<GrabMessage>,
    data_requests: Vec<Arc<Mutex<super::GrabSplineFuture>>>,
    interactable_queries: Vec<Arc<Mutex<super::InteractableFuture>>>,
    line_batch: Option<interactable::InteractableBatch>,
}
pub(crate) struct GrabQueryManager {
    world: SimulationManagerContactAuthority,
    state: Mutex<QueueState>,
}
impl GrabQueryManager {
    /// Deferred producers827b0980/0a50/0b10/0c28/0d40 copy one message into
    /// fixed16 under the same manager lock. Duplicate descriptors are retained.
    pub(crate) fn defer_message(&self, message: GrabMessage) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        if state.messages.len() == 16 {
            return Err(GrindError("native grab message capacity exceeded".into()));
        }
        state.messages.push(message);
        Ok(())
    }

    /// UpdateDeferredGrabSplineMessages827ae780. The dispatch callback must
    /// not reenter this nonrecursive host mutex; native leaf handlers reenter
    /// a recursive critical section. The queue clears after the whole batch.
    pub(crate) fn update_deferred_messages(
        &self,
        mut dispatch: impl FnMut(&GrabMessage) -> Result<(), GrindError>,
    ) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        for message in &state.messages {
            match message.kind {
                // Actual native handlers only enter/leave the lock. Waking
                // occurs in Body::AddLocalForce/AddTorque, not here.
                GrabMessageKind::Activate | GrabMessageKind::Freeze => {}
                _ => dispatch(message)?,
            }
        }
        state.messages.clear();
        Ok(())
    }

    /// RequestGrabSplineData827b1d50: clear readiness, append the actual future
    /// pointer to fixed16. Repeated requests are not deduplicated.
    pub(crate) fn request_spline_data(
        &self,
        output: &Arc<Mutex<super::GrabSplineFuture>>,
    ) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        output
            .lock()
            .map_err(|_| GrindError("grab data lock poisoned".into()))?
            .ready = false;
        if state.data_requests.len() == 16 {
            return Err(GrindError(
                "native grab data-request capacity exceeded".into(),
            ));
        }
        state.data_requests.push(Arc::clone(output));
        Ok(())
    }

    /// UpdateDeferredGrabSplineDataRequests827ae8d8. GetGrabSpline's success
    /// return is ignored; UpdateGrabSpline vtable+44 is the empty825187c0.
    /// Even an unsuccessful lookup publishes readiness, with no result reset.
    pub(crate) fn update_deferred_data(
        &self,
        mut refresh: impl FnMut(&mut super::GrabSpline, &mut GrindNum<'_>) -> Result<(), GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        for output in &state.data_requests {
            let mut output = output
                .lock()
                .map_err(|_| GrindError("grab data lock poisoned".into()))?;
            refresh(&mut output.result, num)?;
            output.ready = true;
        }
        state.data_requests.clear();
        Ok(())
    }
    /// AddDeferredGrabSplineQuery(box) 827b13a0: same manager lock,
    /// output readiness cleared, then copied request appended to fixed16.
    pub(crate) fn add_deferred_box(
        &self,
        request: DeferredGrabBoxQuery,
        output: &Arc<Mutex<GrabQueryFuture>>,
    ) -> Result<(), GrindError> {
        self.add_deferred_query(DeferredGrabQuery::Box(request), output)
    }

    /// Shared fixed16 queue. Radius arm827b12b8 has no box; it uses position,
    /// radius and the same future lifecycle, sorting and maximum as the box arm.
    pub(crate) fn add_deferred_query(
        &self,
        request: DeferredGrabQuery,
        output: &Arc<Mutex<GrabQueryFuture>>,
    ) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        output
            .lock()
            .map_err(|_| GrindError("grab output lock poisoned".into()))?
            .ready = false;
        if state.queries.len() == 16 {
            return Err(GrindError(
                "native deferred grab query capacity exceeded".into(),
            ));
        }
        state.queries.push(DeferredQuery {
            output: Arc::clone(output),
            request,
        });
        Ok(())
    }

    /// UpdateDeferredGrabSplineQueries 827b23e0, with the provider section
    /// of GrabSplineQuery 827b1ed8 supplied by the same World owner.
    pub(crate) fn update_deferred_queries(
        &self,
        mut feed: impl FnMut(
            &DeferredGrabQuery,
            &mut super::GrabQueryBuffer<32>,
            &mut GrindNum<'_>,
        ) -> Result<(), GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrindError("grab manager lock poisoned".into()))?;
        for query in &state.queries {
            let mut output = query
                .output
                .lock()
                .map_err(|_| GrindError("grab output lock poisoned".into()))?;
            if output.ready {
                return Err(GrindError(
                    "deferred grab output already ready (native assertion)".into(),
                ));
            }
            output.result.result_count = 0;
            output.result.cursor = 0;
            feed(&query.request, &mut output.result, num)?;
            output
                .result
                .sort_by_distance(query.request.position(), num)?;
            let maximum = query.request.maximum() as u32;
            if output.result.result_count > maximum {
                output.result.result_count = maximum;
                if output.result.cursor > maximum {
                    output.result.cursor = maximum;
                }
            }
            output.ready = true;
        }
        state.queries.clear();
        Ok(())
    }

    /// Four genuinely empty fixed vectors, capacities16/16/16/1. No pending
    /// query is dropped or fabricated by this selected World-init entry.
    pub(crate) fn new(world: SimulationManagerContactAuthority) -> Self {
        Self {
            world,
            state: Mutex::new(QueueState {
                queries: Vec::with_capacity(16),
                messages: Vec::with_capacity(16),
                data_requests: Vec::with_capacity(16),
                interactable_queries: Vec::with_capacity(1),
                line_batch: None,
            }),
        }
    }
    pub(crate) fn belongs_to(&self, world: SimulationManagerContactAuthority) -> bool {
        self.world == world
    }
    /// `HasDeferredGrabSplineQuery`827ac080: lock, ordered actual output-pointer
    /// search, unlock. A false result follows from this owned queue's contents.
    pub(crate) fn has_deferred(
        &self,
        output: &Arc<Mutex<GrabQueryFuture>>,
    ) -> Result<bool, GrabConstructorError> {
        let state = self
            .state
            .lock()
            .map_err(|_| GrabConstructorError::ManagerPoisoned)?;
        Ok(state
            .queries
            .iter()
            .any(|query| Arc::ptr_eq(&query.output, output)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn messages_keep_order_duplicates_descriptor_and_native_noop_arms() {
        use crate::physical_player_assembly::remaining_states::GrabDescriptor;
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let manager = GrabQueryManager::new(owner.manager());
        for kind in [
            GrabMessageKind::Activate,
            GrabMessageKind::NotifyLocalPlayer,
            GrabMessageKind::NotifyLocalPlayer,
            GrabMessageKind::Freeze,
            GrabMessageKind::NotifyGrabbing,
        ] {
            manager
                .defer_message(GrabMessage {
                    descriptor: GrabDescriptor {
                        object_type: 2,
                        object_id: -1,
                    },
                    kind,
                })
                .unwrap();
        }
        let mut visits = Vec::new();
        manager
            .update_deferred_messages(|message| {
                assert_eq!(message.descriptor.object_type, 2);
                assert_eq!(message.descriptor.object_id, -1);
                visits.push(match message.kind {
                    GrabMessageKind::NotifyLocalPlayer => 1,
                    GrabMessageKind::NotifyGrabbing => 2,
                    _ => panic!("native no-op should not call provider"),
                });
                Ok(())
            })
            .unwrap();
        assert_eq!(visits, [1, 1, 2]);
        assert!(manager.state.lock().unwrap().messages.is_empty());
        for _ in 0..16 {
            manager
                .defer_message(GrabMessage {
                    descriptor: GrabDescriptor {
                        object_type: 2,
                        object_id: 77,
                    },
                    kind: GrabMessageKind::NotifyLocalPlayer,
                })
                .unwrap();
        }
        assert!(
            manager
                .defer_message(GrabMessage {
                    descriptor: GrabDescriptor {
                        object_type: 2,
                        object_id: 88
                    },
                    kind: GrabMessageKind::Freeze,
                })
                .is_err()
        );
        assert_eq!(manager.state.lock().unwrap().messages.len(), 16);
    }

    #[test]
    fn data_queue_keeps_duplicate_future_identity_and_publishes_failed_lookup() {
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let manager = GrabQueryManager::new(owner.manager());
        let output = Arc::new(Mutex::new(
            super::super::GrabSplineFuture::in_sim_allocation(),
        ));
        output.lock().unwrap().result.init_descriptor(2, 99);
        output.lock().unwrap().ready = true;
        manager.request_spline_data(&output).unwrap();
        manager.request_spline_data(&output).unwrap();
        assert!(!output.lock().unwrap().ready);
        let mut visits = 0;
        manager
            .update_deferred_data(
                |spline, _| {
                    assert_eq!((spline.descriptor_type, spline.descriptor_object), (2, 99));
                    assert!(spline.spline_data.is_none());
                    visits += 1;
                    Ok(()) // Failed GetGrabSpline leaves its output untouched.
                },
                &mut GrindNum {
                    n: &mut crate::reckoning_numeric::HostReckoningNumeric,
                    s: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                },
            )
            .unwrap();
        assert_eq!(visits, 2);
        assert!(output.lock().unwrap().ready);
        assert!(manager.state.lock().unwrap().data_requests.is_empty());
    }
    #[test]
    fn completed_query_sorts_then_limits_before_publishing_readiness() {
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let manager = GrabQueryManager::new(owner.manager());
        let future = Arc::new(Mutex::new(GrabQueryFuture {
            result: super::super::GrabQueryBuffer::in_sim_allocation(),
            ready: true,
        }));
        let DeferredGrabQuery::Box(mut request) =
            DeferredQuery::identity_fixture(Arc::clone(&future)).request
        else {
            unreachable!()
        };
        request.maximum = 2;
        manager.add_deferred_box(request, &future).unwrap();
        assert!(!future.lock().unwrap().ready);
        let mut n = crate::reckoning_numeric::HostReckoningNumeric;
        let mut s = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
        manager
            .update_deferred_queries(
                |_, output, _| {
                    assert_eq!(output.result_count, 0);
                    for (index, z) in [3.0, 1.0, 2.0].into_iter().enumerate() {
                        output.splines[index].p1 = [-1.0, 0.0, z, 0.0];
                        output.splines[index].p2 = [1.0, 0.0, z, 0.0];
                        output.splines[index].descriptor_object = index as i32;
                    }
                    output.result_count = 3;
                    Ok(())
                },
                &mut GrindNum {
                    n: &mut n,
                    s: &mut s,
                },
            )
            .unwrap();
        assert!(!manager.has_deferred(&future).unwrap());
        let output = future.lock().unwrap();
        assert!(output.ready);
        assert_eq!(output.result.result_count, 2);
        assert_eq!(output.result.splines[0].descriptor_object, 1);
        assert_eq!(output.result.splines[1].descriptor_object, 2);
    }

    #[test]
    fn deferred_lookup_compares_actual_output_occurrences_under_owned_queue() {
        let mut world =
            crate::physical_player_assembly::PhysicalAssemblyWorld::try_new(1, 0, 0, 0).unwrap();
        assert!(world.grab_query_manager().is_none());
        world.try_initialize_grab_query_manager(true).unwrap();
        let manager = world.grab_query_manager().unwrap();
        let future = || {
            Arc::new(Mutex::new(GrabQueryFuture {
                result: super::super::GrabQueryBuffer::in_sim_allocation(),
                ready: false,
            }))
        };
        let first = future();
        let second = future();
        assert!(!manager.has_deferred(&first).unwrap());
        // Focused queue fixture exercises identity search, not live query admission.
        manager
            .state
            .lock()
            .unwrap()
            .queries
            .push(DeferredQuery::identity_fixture(Arc::clone(&first)));
        assert!(manager.has_deferred(&first).unwrap());
        assert!(!manager.has_deferred(&second).unwrap());
        assert!(world.try_initialize_grab_query_manager(true).is_err());
    }
}

#[path = "physical_player_grab_remove_deferred.rs"]
pub(crate) mod remove_deferred;
