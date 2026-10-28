//! Retained `PlayerGrabSpline`82dd1c10 and embedded `GrabSpline`82d98ce8.
#![expect(
    dead_code,
    reason = "constructed grab queries await actual launch and validation consumers"
)]
use crate::{
    living_world_grind_services::dmo_models::DmoGrabSpline,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    simulation_rigid_body_pool::SimulationInertia,
};
use std::sync::{Arc, Mutex};

#[path = "physical_player_grab_query_manager.rs"]
pub(crate) mod manager;

#[derive(Clone)]
pub(crate) struct GrabSpline {
    pub(crate) transform: [[f32; 4]; 4],
    pub(crate) p1: [f32; 4],
    pub(crate) p2: [f32; 4],
    pub(crate) normal: [f32; 4],
    pub(crate) length_unit: Option<[f32; 4]>,
    pub(crate) velocity: [f32; 4],
    pub(crate) body_position: [f32; 4],
    pub(crate) length: f32,
    pub(crate) half_length: f32,
    pub(crate) sort_parameter: f32,
    pub(crate) descriptor_type: i32,
    pub(crate) descriptor_object: i32,
    spline_data: Option<DmoGrabSpline>,
    pub(crate) status: u8,
    pub(crate) status_known_mask: u8,
    pub(crate) flags: u32,
    assembly: Option<
        crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    >,
    /// Part ordinal within the retained assembly; Init selects Part 0.
    part: Option<usize>,
    rigid_body: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyObservation>,
    pub(crate) inertia: SimulationInertia,
    pub(crate) inverse_tensor_w: f32,
}

#[path = "physical_player_grab_geometry.rs"]
pub(crate) mod geometry;
impl GrabSpline {
    /// Native pushing entry/update separately require mpSplineData and mRigidBody.
    /// A descriptor or a published-ready flag alone does not prove either.
    pub(crate) fn has_spline_and_body(&self) -> bool {
        self.spline_data.is_some() && self.rigid_body.is_some()
    }

    /// Init(type,id)82d15f08 is not Reset: flags, sort key, assembly/Part/body
    /// and inertia are deliberately untouched while geometry is invalidated.
    pub(crate) fn init_descriptor(&mut self, kind: i32, object: i32) {
        self.descriptor_type = kind;
        self.descriptor_object = object;
        self.spline_data = None;
        self.half_length = 0.0;
        self.length = 0.0;
        self.status = (self.status & 0x1f) | 0x80;
        self.status_known_mask |= 0xe0;
        self.transform = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0; 4],
        ];
        self.p1 = [0.0; 4];
        self.p2 = [0.0; 4];
        self.normal = [0.0; 4];
        self.length_unit = Some([0.0; 4]);
        self.velocity = [0.0; 4];
        self.body_position = [0.0; 4];
    }
    /// `UpdatePostPhysics82b57754` reads this only after actual status bit40.
    pub(crate) fn collision_rigid_body(
        &self,
    ) -> Option<crate::simulation_rigid_body_pool::SimulationRigidBodyObservation> {
        self.rigid_body
    }

    /// `GrabSpline::Reset`827ada10 preserves length-unit and low five status bits.
    pub(crate) fn reset(&mut self) {
        let length_unit = self.length_unit;
        let status = self.status & 0x1f;
        let known = self.status_known_mask | 0xe0;
        *self = Self::in_uninitialized_allocation();
        self.length_unit = length_unit;
        self.status = status;
        self.status_known_mask = known;
    }
    /// Reset827ada10 inside the actual Sim-cleared `PlayerGrabSpline` parent.
    /// mLengthUnit+70 is untouched by Reset; parent allocation proves zero.
    fn in_sim_allocation() -> Self {
        let mut value = Self::in_uninitialized_allocation();
        value.length_unit = Some([0.0; 4]);
        value.status_known_mask = u8::MAX;
        value
    }
    /// Same ctor/Reset in a `PhysicsSkater` allocation with no memory clearing.
    /// Reset leaves length-unit and the five low status bits unwritten.
    pub(crate) fn in_uninitialized_allocation() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0; 4],
            ],
            p1: [0.0; 4],
            p2: [0.0; 4],
            normal: [0.0; 4],
            length_unit: None,
            velocity: [0.0; 4],
            body_position: [0.0; 4],
            length: 0.0,
            half_length: 0.0,
            sort_parameter: 0.0,
            descriptor_type: 0,
            descriptor_object: 0,
            spline_data: None,
            status: 0,
            status_known_mask: 0xe0,
            flags: 0,
            assembly: None,
            part: None,
            rigid_body: None,
            inertia: SimulationInertia::native_default(),
            inverse_tensor_w: 1.0,
        }
    }
}
pub(crate) struct GrabSplineFuture {
    pub(crate) result: GrabSpline,
    pub(crate) ready: bool,
}
impl GrabSplineFuture {
    fn in_sim_allocation() -> Self {
        Self {
            result: GrabSpline::in_sim_allocation(),
            ready: false,
        }
    }
}
pub(crate) struct GrabQueryBuffer<const N: usize> {
    pub(crate) splines: [GrabSpline; N],
    pub(crate) result_count: u32,
    pub(crate) cursor: u32,
}
impl<const N: usize> GrabQueryBuffer<N> {
    fn in_sim_allocation() -> Self {
        Self {
            splines: std::array::from_fn(|_| GrabSpline::in_sim_allocation()),
            result_count: 0,
            cursor: 0,
        }
    }
}
pub(crate) struct GrabQueryFuture {
    pub(crate) result: GrabQueryBuffer<32>,
    pub(crate) ready: bool,
}
/// Native InteractableObjectQueryBuffer holds one ID plus count/cursor.
/// Deferred manager writes and player publication share this actual future.
#[derive(Default)]
pub(crate) struct InteractableFuture {
    pub(crate) object_id: u32,
    pub(crate) result_count: u32,
    pub(crate) cursor: u32,
    pub(crate) ready: bool,
}
pub(crate) struct PlayerGrabSpline {
    pub(crate) data: Arc<Mutex<GrabSplineFuture>>,
    pub(crate) alternate: Arc<Mutex<GrabSplineFuture>>,
    pub(crate) query: Arc<Mutex<GrabQueryFuture>>,
    pub(crate) pending_validation: GrabQueryBuffer<5>,
    pub(crate) validated: GrabQueryBuffer<5>,
    pub(crate) interactable: Arc<Mutex<InteractableFuture>>,
    pub(crate) position: [f32; 4],
    pub(crate) pending_position: [f32; 4],
    pub(crate) manager: Option<Arc<manager::GrabQueryManager>>,
    line_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch>,
    pub(crate) world: SimulationManagerContactAuthority,
    pub(crate) flags: u8,
    pub(crate) returned: bool,
}
#[derive(Debug)]
pub(crate) enum GrabConstructorError {
    MissingManager,
    ForeignWorld,
    ManagerPoisoned,
    DeferredQueryExists,
}
impl std::fmt::Display for GrabConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grab-spline constructor: {self:?}")
    }
}
impl PlayerGrabSpline {
    /// Activate/Freeze/ApplyForce/Notify wrappers82dc71a8..7220 forward the
    /// descriptor and payload into the same retained World manager.
    pub(crate) fn defer_message(
        &self,
        descriptor: crate::physical_player_assembly::remaining_states::GrabDescriptor,
        message: manager::GrabMessageKind,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::GrindError;
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| GrindError("grab message missing manager".into()))?;
        if !manager.belongs_to(self.world) {
            return Err(GrindError("grab message foreign World".into()));
        }
        manager.defer_message(manager::GrabMessage {
            descriptor,
            kind: message,
        })
    }

    /// RequestGrabSplineData82dcbac8 initializes the retained primary result
    /// before enqueuing that future in the shared World manager.
    pub(crate) fn request_spline_data(
        &mut self,
        kind: i32,
        object: i32,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::GrindError;
        self.data
            .lock()
            .map_err(|_| GrindError("grab data lock poisoned".into()))?
            .result
            .init_descriptor(kind, object);
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| GrindError("grab data missing manager".into()))?;
        if !manager.belongs_to(self.world) {
            return Err(GrindError("grab data foreign World".into()));
        }
        manager.request_spline_data(&self.data)
    }
    /// StartGrabSplineQuery(box) 82dc7280 always requests five results and
    /// saves the reach position only after the manager accepted the request.
    pub(crate) fn start_box_query(
        &mut self,
        query_type: u32,
        geometry: crate::living_world_grind_services::grab_provider::GrabBoxQuery,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::GrindError;
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| GrindError("grab query has no manager".into()))?;
        if !manager.belongs_to(self.world) {
            return Err(GrindError(
                "grab query manager belongs to another World".into(),
            ));
        }
        manager.add_deferred_box(
            manager::DeferredGrabBoxQuery {
                query_type,
                maximum: 5,
                geometry,
            },
            &self.query,
        )?;
        self.position = geometry.reach_start;
        Ok(())
    }

    /// Radius StartGrabSplineQuery82dc7220, sharing the existing manager/future.
    pub(crate) fn start_radius_query(
        &mut self,
        query_type: u32,
        position: [f32; 4],
        radius: f32,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::GrindError;
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| GrindError("grab query has no manager".into()))?;
        if !manager.belongs_to(self.world) {
            return Err(GrindError(
                "grab query manager belongs to another World".into(),
            ));
        }
        // StartGrabSplineQuery82dc7220: enqueue five results, THEN save position.
        manager.add_deferred_query(
            manager::DeferredGrabQuery::Radius {
                query_type,
                maximum: 5,
                position,
                radius,
            },
            &self.query,
        )?;
        self.position = position;
        Ok(())
    }

    /// IsQueryReady 82dc7380 reads the validated flag, not future readiness.
    pub(crate) fn query_is_ready(&self) -> bool {
        self.flags & 0x40 != 0
    }

    pub(crate) fn new_prefix(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        manager: Option<Arc<manager::GrabQueryManager>>,
    ) -> Self {
        Self {
            data: Arc::new(Mutex::new(GrabSplineFuture::in_sim_allocation())),
            alternate: Arc::new(Mutex::new(GrabSplineFuture::in_sim_allocation())),
            query: Arc::new(Mutex::new(GrabQueryFuture {
                result: GrabQueryBuffer::in_sim_allocation(),
                ready: false,
            })),
            pending_validation: GrabQueryBuffer::in_sim_allocation(),
            validated: GrabQueryBuffer::in_sim_allocation(),
            interactable: Arc::new(Mutex::new(InteractableFuture::default())),
            position: [0.0; 4],
            pending_position: [0.0; 4],
            manager,
            line_batch: None,
            world: owner.manager(),
            flags: 0,
            returned: false,
        }
    }
    /// Source constructor must query the actual manager after all prior stores.
    pub(crate) fn try_finish(&mut self) -> Result<(), GrabConstructorError> {
        let manager = self
            .manager
            .as_ref()
            .ok_or(GrabConstructorError::MissingManager)?;
        if !manager.belongs_to(self.world) {
            return Err(GrabConstructorError::ForeignWorld);
        }
        if manager.has_deferred(&self.query)? {
            return Err(GrabConstructorError::DeferredQueryExists);
        }
        self.returned = true;
        Ok(())
    }
}

#[path = "physical_player_grab_publication.rs"]
pub(crate) mod publication;

#[path = "physical_player_grab_sync.rs"]
pub(crate) mod sync;

#[path = "physical_player_grab_invalidate.rs"]
pub(crate) mod invalidate;

#[cfg(test)]
mod descriptor_tests {
    use super::GrabSpline;
    #[test]
    fn descriptor_init_is_not_reset_and_preserves_unwritten_members() {
        let mut spline = GrabSpline::in_sim_allocation();
        spline.status = 0xff;
        spline.flags = 0x1234;
        spline.sort_parameter = 9.0;
        spline.inverse_tensor_w = 7.0;
        spline.p1 = [1.0; 4];
        spline.length_unit = Some([2.0; 4]);
        spline.init_descriptor(2, 67);
        assert_eq!((spline.descriptor_type, spline.descriptor_object), (2, 67));
        assert_eq!(spline.status, 0x9f);
        assert_eq!(spline.flags, 0x1234);
        assert_eq!(spline.sort_parameter, 9.0);
        assert_eq!(spline.inverse_tensor_w, 7.0);
        assert_eq!(spline.p1, [0.0; 4]);
        assert_eq!(spline.length_unit, Some([0.0; 4]));
    }
}
