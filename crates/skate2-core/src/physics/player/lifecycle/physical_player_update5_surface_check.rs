//! Same retained Processed and current `PhysOut` at Update5 call82e061ec.
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::update5_surface_check::{
        Update5SurfaceCheckError, Update5SurfaceCheckSource,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SurfaceCheckRuntimeError {
    Poisoned,
    Missing(&'static str),
}
impl std::fmt::Display for SurfaceCheckRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 surface input owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 surface check requires {field}"),
        }
    }
}
struct Source<'a>(&'a Arc<PhysicalPlayerAssemblyOwner>);
impl Update5SurfaceCheckSource for Source<'_> {
    type Error = SurfaceCheckRuntimeError;
    type Processed = Arc<Mutex<SkeletonProcessedContact>>;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        Ok(Arc::clone(
            self.0
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        ))
    }
    fn physics_state(&mut self, owner: &Self::Processed) -> Result<i32, Self::Error> {
        // mPhysicsState8e4, not distinct mCurrentPhysicsState8e8.
        owner
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .physics
            .ok_or(Self::Error::Missing("Processed physics state8e4"))
    }
    fn hips_valid(&mut self, owner: &Self::Processed) -> Result<bool, Self::Error> {
        Ok(owner
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .line_surfaces()
            .hips
            .ok_or(Self::Error::Missing("Processed hips surface414"))?
            .valid)
    }
    fn hips_surface_id(&mut self, owner: &Self::Processed) -> Result<u32, Self::Error> {
        let id = owner
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .line_surfaces()
            .hips
            .ok_or(Self::Error::Missing("Processed hips surface410"))?
            .surface_id;
        Ok(u32::from_ne_bytes(id.to_ne_bytes()))
    }
}
pub(crate) fn try_do_update5_surface_check(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<(), Update5SurfaceCheckError<SurfaceCheckRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5SurfaceCheckError::Source(SurfaceCheckRuntimeError::Poisoned))?;
    output.try_do_update5_surface_check(&mut Source(owner))
}
