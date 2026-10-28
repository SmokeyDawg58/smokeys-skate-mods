//! Definition-backed source routes on the manager's existing body pool.
//! Routes preserve native Add-selected inertia words, full Part transforms,
//! and copyback/removal authority. Image/resource bases remain explicitly
//! staged allocator inputs; this module never manufactures an address.

use super::{
    Arc, AssemblyPartMaterial, DynamicAssemblyPartAttachmentMetadata,
    DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload, Error,
    SerialisedAssemblyArenaRoute, SerialisedAssemblyArenaRouteToken,
    SerialisedAssemblyPartAttachmentBindError, SerialisedAssemblyRoutedInertia,
    SerialisedAssemblyRuntimeInertia, SerialisedAssemblySimulationBackendInitError,
    SerialisedAssemblySimulationBindError, SerialisedAssemblyTemplate,
    SimulationArenaXenonAddressBinding, SimulationAssemblyBackend,
    SimulationAssemblyBackendAddFailure, SimulationAssemblyInertia, SimulationNormalArenaPlan,
    SimulationPackedBodyWordImageDomain, SimulationRigidBodyAddInertiaInput,
    SimulationRigidBodyAddInertiaOwner, SimulationRigidBodyAddInertiaOwnerError,
    SimulationRigidBodyHandle, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
    SimulationXenonAddressImage, fmt,
};
use crate::{
    serialised_assembly_create::{SimulationSourceBodyTransformError, try_source_body_transform},
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
};

/// Native Part getter/copyback composition at 0x826c3480..0x826c35a8.
/// Both paths retain packed W through the same accepted full-lane graph.
pub(crate) fn source_part_transform(body: [f32; 16], inverse: Option<&[f32; 16]>) -> [f32; 16] {
    inverse.map_or(body, |inverse| {
        crate::skateboard_spawn_math::skateboard_spawn_compose(&body, inverse)
    })
}

#[cfg(test)]
#[path = "simulation_assembly_source_backend_tests.rs"]
pub(crate) mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SimulationAssemblySourceError {
    Unconfigured,
    ForeignRoute,
    MissingRoute,
    ArenaNotRegistered,
    ForeignArena,
    UnroutedBody,
    BodyRouteMismatch,
    OccupiedDirectory { slot: u32 },
    Body(SimulationRigidBodyPoolError),
    Inertia(SimulationRigidBodyAddInertiaOwnerError),
    Transform(SimulationSourceBodyTransformError),
}

impl fmt::Display for SimulationAssemblySourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body(error) => write!(f, "source body: {error}"),
            Self::Inertia(error) => write!(f, "source inertia: {error}"),
            Self::Transform(error) => write!(f, "source transform: {error}"),
            other => write!(f, "source body authority: {other:?}"),
        }
    }
}
impl Error for SimulationAssemblySourceError {}

/// Public backend diagnostic retains a private, typed source-route cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationAssemblySourceAddError(SimulationAssemblySourceError);

impl SimulationAssemblySourceAddError {
    pub(crate) fn cause(&self) -> &SimulationAssemblySourceError {
        &self.0
    }
}
impl fmt::Display for SimulationAssemblySourceAddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.cause().fmt(f)
    }
}
impl Error for SimulationAssemblySourceAddError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SimulationAssemblySourceInitError {
    AlreadyConfigured,
    DirectoryAllocation { capacity: usize },
    Source(SerialisedAssemblySimulationBackendInitError),
}
impl fmt::Display for SimulationAssemblySourceInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConfigured => f.write_str("shared source routes are already configured"),
            Self::DirectoryAllocation { capacity } => {
                write!(f, "could not reserve {capacity} source body routes")
            }
            Self::Source(error) => error.fmt(f),
        }
    }
}
impl Error for SimulationAssemblySourceInitError {}

#[derive(Clone, Copy, Debug)]
struct SourceBodyRoute {
    observation: SimulationRigidBodyObservation,
    route: SerialisedAssemblyArenaRouteToken,
}

pub(super) struct SimulationAssemblySourceRoutes {
    image: SimulationPackedBodyWordImageDomain,
    routes: Box<[SerialisedAssemblyArenaRoute]>,
    bodies: Box<[Option<SourceBodyRoute>]>,
}

impl SimulationAssemblySourceRoutes {
    /// The published `Simulation` image, borrowed beside an exclusive body
    /// pool phase by serialized collision jobs.
    pub(super) const fn image(&self) -> &SimulationPackedBodyWordImageDomain {
        &self.image
    }
}

impl fmt::Debug for SimulationAssemblySourceRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationAssemblySourceRoutes")
            .field("image_epoch", &self.image.epoch())
            .field("arena_count", &self.routes.len())
            .field("body_capacity", &self.bodies.len())
            .finish_non_exhaustive()
    }
}

/// Immutable same-owner authorities for the existing Create/refresh helpers.
pub(crate) struct SimulationAssemblySourceContext<'a> {
    pub(crate) body_pool: &'a SimulationRigidBodyPool,
    pub(crate) inertia_owner: &'a SimulationRigidBodyAddInertiaOwner,
    pub(crate) image_domain: &'a SimulationPackedBodyWordImageDomain,
}

impl SerialisedAssemblyRoutedInertia {
    pub(crate) fn source_owner(&self) -> &Arc<SimulationNormalArenaPlan> {
        self.inertia.source().owner()
    }

    pub(crate) fn runtime_inertia(&self) -> &SerialisedAssemblyRuntimeInertia {
        &self.inertia
    }
}

/// Shared by the isolated staging backend and actual-manager source service.
pub(super) fn try_build_routes(
    pool: &SimulationRigidBodyPool,
    bindings: Vec<(
        Arc<SimulationNormalArenaPlan>,
        Arc<SimulationArenaXenonAddressBinding>,
    )>,
) -> Result<Box<[SerialisedAssemblyArenaRoute]>, SerialisedAssemblySimulationBackendInitError> {
    let mut routes = Vec::new();
    routes.try_reserve_exact(bindings.len()).map_err(|_| {
        SerialisedAssemblySimulationBackendInitError::RouteStorageAllocation {
            requested: bindings.len(),
        }
    })?;
    for (ordinal, (owner, binding)) in bindings.into_iter().enumerate() {
        let inertia =
            SimulationRigidBodyAddInertiaOwner::try_new(pool, binding).map_err(|source| {
                SerialisedAssemblySimulationBackendInitError::Route { ordinal, source }
            })?;
        if !inertia.is_same_arena_owner(&owner) {
            return Err(
                SerialisedAssemblySimulationBackendInitError::BindingOwnerMismatch { ordinal },
            );
        }
        if let Some((first, _)) =
            routes
                .iter()
                .enumerate()
                .find(|(_, route): &(usize, &SerialisedAssemblyArenaRoute)| {
                    route.inertia.is_same_arena_owner(&owner)
                })
        {
            return Err(
                SerialisedAssemblySimulationBackendInitError::DuplicateArenaOwner {
                    first,
                    second: ordinal,
                },
            );
        }
        let binding_epoch = inertia.binding_epoch();
        if let Some((first, _)) =
            routes
                .iter()
                .enumerate()
                .find(|(_, route): &(usize, &SerialisedAssemblyArenaRoute)| {
                    route.token.binding_epoch == binding_epoch
                })
        {
            return Err(
                SerialisedAssemblySimulationBackendInitError::DuplicateBindingEpoch {
                    first,
                    second: ordinal,
                    epoch: binding_epoch,
                },
            );
        }
        routes.push(SerialisedAssemblyArenaRoute {
            token: SerialisedAssemblyArenaRouteToken {
                pool: pool.authority(),
                binding_epoch,
            },
            owner,
            inertia,
        });
    }
    Ok(routes.into_boxed_slice())
}

/// Same effect-free Definition-order binder for both backend storage policies.
pub(super) fn try_bind_attachment<I>(
    template: &SerialisedAssemblyTemplate,
    mut bind: impl FnMut(
        &SerialisedAssemblyRuntimeInertia,
    ) -> Result<I, SerialisedAssemblySimulationBindError>,
) -> Result<
    Arc<DynamicAssemblyPartAttachmentTemplate<I, AssemblyPartMaterial>>,
    SerialisedAssemblyPartAttachmentBindError,
> {
    let assembly = template.assembly();
    let definition = template.definition();
    if assembly.joint_count != definition.joint_count {
        return Err(
            SerialisedAssemblyPartAttachmentBindError::JointCountMismatch {
                assembly: assembly.joint_count,
                definition: definition.joint_count,
            },
        );
    }
    if assembly.drive_count != definition.drive_count {
        return Err(
            SerialisedAssemblyPartAttachmentBindError::DriveCountMismatch {
                assembly: assembly.drive_count,
                definition: definition.drive_count,
            },
        );
    }
    if assembly.joint_count != 0 {
        return Err(
            SerialisedAssemblyPartAttachmentBindError::UnsupportedJoints {
                count: assembly.joint_count,
            },
        );
    }
    if assembly.drive_count != 0 {
        return Err(
            SerialisedAssemblyPartAttachmentBindError::UnsupportedDrives {
                count: assembly.drive_count,
            },
        );
    }
    let part_count = template.parts().len();
    let mut metadata = Vec::new();
    metadata.try_reserve_exact(part_count).map_err(|_| {
        SerialisedAssemblyPartAttachmentBindError::StorageAllocation {
            requested: part_count,
        }
    })?;
    let mut payloads = Vec::new();
    payloads.try_reserve_exact(part_count).map_err(|_| {
        SerialisedAssemblyPartAttachmentBindError::StorageAllocation {
            requested: part_count,
        }
    })?;
    for (ordinal, part) in template.parts().iter().enumerate() {
        let inertia = template
            .runtime_inertia(ordinal)
            .expect("Definition Part remains present");
        let inertia = bind(&inertia).map_err(|source| {
            SerialisedAssemblyPartAttachmentBindError::Route { ordinal, source }
        })?;
        metadata.push(DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: part.inverse_body_transform,
            has_inverse_body_transform: part.has_inverse_body_transform,
            is_static_body: part.is_static_body,
        });
        payloads.push(DynamicAssemblyPartPayload {
            inertia,
            material: part.material,
        });
    }
    DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads)
        .map(Arc::new)
        .map_err(SerialisedAssemblyPartAttachmentBindError::AttachmentTemplate)
}

impl SimulationAssemblySourceRoutes {
    fn route_index(&self, token: SerialisedAssemblyArenaRouteToken) -> Option<usize> {
        self.routes.iter().position(|route| route.token == token)
    }

    fn bind(
        &self,
        inertia: &SerialisedAssemblyRuntimeInertia,
    ) -> Result<SerialisedAssemblyRoutedInertia, SerialisedAssemblySimulationBindError> {
        let route = self
            .routes
            .iter()
            .find(|route| route.inertia.is_same_arena_owner(inertia.source().owner()))
            .ok_or(SerialisedAssemblySimulationBindError::ArenaNotRegistered)?;
        Ok(SerialisedAssemblyRoutedInertia {
            route: route.token,
            inertia: inertia.clone(),
        })
    }

    fn try_add(
        &mut self,
        pool: &mut SimulationRigidBodyPool,
        transform: [f32; 16],
        inertia: &SerialisedAssemblyRoutedInertia,
        state: u32,
    ) -> Result<Option<SimulationRigidBodyHandle>, SimulationAssemblySourceError> {
        if inertia.route.pool != pool.authority() {
            return Err(SimulationAssemblySourceError::ForeignRoute);
        }
        let route_index = self
            .route_index(inertia.route)
            .ok_or(SimulationAssemblySourceError::MissingRoute)?;
        // Drop-inert preflight checks the directory before the reused outcome
        // owner commits. Repeated reservation has no native/pool effects.
        let slot = {
            let Some(reservation) = pool
                .try_prepare_add_rigid_body(state)
                .map_err(SimulationAssemblySourceError::Body)?
            else {
                return Ok(None);
            };
            reservation.id().index()
        };
        let index = usize::try_from(slot).expect("pool slot fits host usize");
        if self.bodies[index].is_some() {
            return Err(SimulationAssemblySourceError::OccupiedDirectory { slot });
        }
        let handle = self.routes[route_index]
            .inertia
            .try_add(
                pool,
                transform,
                SimulationRigidBodyAddInertiaInput::Definition(&inertia.inertia),
                state,
            )
            .map_err(SimulationAssemblySourceError::Inertia)?;
        if let Some(body) = handle {
            debug_assert_eq!(body.id().index(), slot);
            self.bodies[index] = Some(SourceBodyRoute {
                observation: body.observation(),
                route: inertia.route,
            });
        }
        Ok(handle)
    }

    fn body_route(
        &self,
        pool: &SimulationRigidBodyPool,
        body: SimulationRigidBodyHandle,
    ) -> Result<Option<usize>, SimulationAssemblySourceError> {
        pool.try_validate_observation(body.observation())
            .map_err(SimulationAssemblySourceError::Body)?;
        let index = usize::try_from(body.id().index()).expect("pool slot fits host usize");
        let Some(record) = self.bodies[index] else {
            return Ok(None);
        };
        if record.observation != body.observation() {
            return Err(SimulationAssemblySourceError::BodyRouteMismatch);
        }
        self.route_index(record.route)
            .map(Some)
            .ok_or(SimulationAssemblySourceError::MissingRoute)
    }

    fn try_remove(
        &mut self,
        pool: &mut SimulationRigidBodyPool,
        body: SimulationRigidBodyHandle,
    ) -> Result<bool, SimulationAssemblySourceError> {
        let Some(route) = self.body_route(pool, body)? else {
            return Ok(false);
        };
        self.routes[route]
            .inertia
            .try_remove(pool, body)
            .map_err(SimulationAssemblySourceError::Inertia)?;
        self.bodies[usize::try_from(body.id().index()).expect("pool slot fits host usize")] = None;
        Ok(true)
    }
}

impl<M, JF: Copy, JL: Copy> SimulationAssemblyBackend<M, JF, JL> {
    /// Configure source publication once on the EXISTING manager pool. Every
    /// allocation/owner check finishes before publishing the service. Existing
    /// ordinary bodies remain owned by that same pool and are not re-added.
    pub(crate) fn try_bind_source_arenas(
        &mut self,
        image: SimulationXenonAddressImage,
        bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> Result<(), SimulationAssemblySourceInitError> {
        if self.source_routes.is_some() {
            return Err(SimulationAssemblySourceInitError::AlreadyConfigured);
        }
        let image = SimulationPackedBodyWordImageDomain::try_new(&self.rigid_bodies, image)
            .map_err(SerialisedAssemblySimulationBackendInitError::ImageDomain)
            .map_err(SimulationAssemblySourceInitError::Source)?;
        let routes = try_build_routes(&self.rigid_bodies, bindings)
            .map_err(SimulationAssemblySourceInitError::Source)?;
        let capacity = self.rigid_bodies.slot_capacity();
        let mut bodies = Vec::new();
        bodies
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationAssemblySourceInitError::DirectoryAllocation { capacity })?;
        bodies.resize_with(capacity, || None);
        self.source_routes = Some(SimulationAssemblySourceRoutes {
            image,
            routes,
            bodies: bodies.into_boxed_slice(),
        });
        Ok(())
    }

    pub(crate) fn try_source_context(
        &self,
        owner: &Arc<SimulationNormalArenaPlan>,
    ) -> Result<SimulationAssemblySourceContext<'_>, SimulationAssemblySourceError> {
        let source = self
            .source_routes
            .as_ref()
            .ok_or(SimulationAssemblySourceError::Unconfigured)?;
        let route = source
            .routes
            .iter()
            .find(|route| route.inertia.is_same_arena_owner(owner))
            .ok_or(SimulationAssemblySourceError::ArenaNotRegistered)?;
        Ok(SimulationAssemblySourceContext {
            body_pool: &self.rigid_bodies,
            inertia_owner: &route.inertia,
            image_domain: &source.image,
        })
    }

    pub(crate) fn try_source_body_pose(
        &self,
        body: SimulationRigidBodyHandle,
        owner: &Arc<SimulationNormalArenaPlan>,
    ) -> Result<[f32; 16], SimulationAssemblySourceError> {
        let source = self
            .source_routes
            .as_ref()
            .ok_or(SimulationAssemblySourceError::Unconfigured)?;
        let index = source
            .body_route(&self.rigid_bodies, body)?
            .ok_or(SimulationAssemblySourceError::UnroutedBody)?;
        let route = &source.routes[index];
        if !route.inertia.is_same_arena_owner(owner) {
            return Err(SimulationAssemblySourceError::ForeignArena);
        }
        try_source_body_transform(
            owner,
            &self.rigid_bodies,
            &route.inertia,
            &source.image,
            body.observation(),
        )
        .map_err(SimulationAssemblySourceError::Transform)
    }

    /// Read-only observation for owner-integrated Create input capture. This
    /// validates the route/outcome without projecting transform words early.
    pub(crate) fn try_source_body_observation(
        &self,
        body: SimulationRigidBodyHandle,
        owner: &Arc<SimulationNormalArenaPlan>,
    ) -> Result<SimulationRigidBodyObservation, SimulationAssemblySourceError> {
        let source = self
            .source_routes
            .as_ref()
            .ok_or(SimulationAssemblySourceError::Unconfigured)?;
        let index = source
            .body_route(&self.rigid_bodies, body)?
            .ok_or(SimulationAssemblySourceError::UnroutedBody)?;
        let route = &source.routes[index];
        if !route.inertia.is_same_arena_owner(owner) {
            return Err(SimulationAssemblySourceError::ForeignArena);
        }
        route
            .inertia
            .try_outcome(&self.rigid_bodies, body.observation())
            .map_err(SimulationAssemblySourceError::Inertia)?;
        Ok(body.observation())
    }

    pub(super) fn add_source_body(
        &mut self,
        transform: [f32; 16],
        inertia: &SerialisedAssemblyRoutedInertia,
        state: u32,
    ) -> Option<SimulationRigidBodyHandle> {
        let result = self
            .source_routes
            .as_mut()
            .ok_or(SimulationAssemblySourceError::Unconfigured)
            .and_then(|source| source.try_add(&mut self.rigid_bodies, transform, inertia, state));
        match result {
            Ok(body) => body,
            Err(error) => {
                self.last_add_failure = Some(SimulationAssemblyBackendAddFailure::SourceRigidBody(
                    SimulationAssemblySourceAddError(error),
                ));
                None
            }
        }
    }

    pub(super) fn source_copyback_pose(
        &self,
        body: SimulationRigidBodyHandle,
    ) -> Result<Option<[f32; 16]>, SimulationAssemblySourceError> {
        let Some(source) = &self.source_routes else {
            return Ok(None);
        };
        let Some(index) = source.body_route(&self.rigid_bodies, body)? else {
            return Ok(None);
        };
        let route = &source.routes[index];
        // The route's binding owns the plan. Obtain it from the exact Add
        // Definition when nonstatic; Static has no Definition outcome, so use
        // the registered owner retained independently in the route.
        let owner = &route.owner;
        try_source_body_transform(
            owner,
            &self.rigid_bodies,
            &route.inertia,
            &source.image,
            body.observation(),
        )
        .map(Some)
        .map_err(SimulationAssemblySourceError::Transform)
    }

    pub(super) fn remove_source_body(
        &mut self,
        body: SimulationRigidBodyHandle,
    ) -> Result<bool, SimulationAssemblySourceError> {
        let Some(source) = &mut self.source_routes else {
            return Ok(false);
        };
        source.try_remove(&mut self.rigid_bodies, body)
    }
}

impl<JF: Copy, JL: Copy> SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL> {
    pub(crate) fn try_bind_serialised_part_attachment_template(
        &self,
        template: &SerialisedAssemblyTemplate,
    ) -> Result<
        Arc<DynamicAssemblyPartAttachmentTemplate<SimulationAssemblyInertia, AssemblyPartMaterial>>,
        SerialisedAssemblyPartAttachmentBindError,
    > {
        try_bind_attachment(template, |inertia| {
            self.source_routes
                .as_ref()
                .ok_or(SerialisedAssemblySimulationBindError::ArenaNotRegistered)?
                .bind(inertia)
                .map(SimulationAssemblyInertia::source_routed)
        })
    }
}
