//! Borrowed actual globals for `UpdateUpVector` 0x82b26158. Layout tokens
//! capture identity only; immutable range checks and decoding remain late.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimePhysics},
    reckoning_attributes::{ReckoningAttributeError, ReckoningCapturedLayout},
    skateboard_contact_surface::normal_attributes::{
        SkateboardGroundNormalAttributeError, SkateboardGroundNormalAttributeRead,
        SkateboardGroundNormalGlobal, SkateboardGroundNormalPointer,
        SkateboardGroundNormalVaultAttributes,
    },
    skateboard_update_up_vector::SkateboardUpdateUpAttributes,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum SkateboardUpdateUpAttributeError {
    Ground(SkateboardGroundNormalAttributeError),
    Physics(AttributeRuntimeError),
    Graph(ReckoningAttributeError),
    ForeignPhysics,
    ForeignLayout,
}
impl std::fmt::Display for SkateboardUpdateUpAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ground(e) => write!(f, "up-vector global: {e}"),
            Self::Physics(e) => write!(f, "up-vector physics: {e}"),
            Self::Graph(e) => write!(f, "up-vector graph: {e}"),
            Self::ForeignPhysics => f.write_str("foreign physics occurrence"),
            Self::ForeignLayout => f.write_str("foreign Reckoning layout occurrence"),
        }
    }
}
pub(crate) struct SkateboardUpdateUpPhysicsToken<'a>(&'a AttributeRuntimePhysics);
pub(crate) struct SkateboardUpdateUpReckoningLayout {
    identity: Arc<()>,
    instance: AttributeRuntimeInstance,
}
pub(crate) struct SkateboardUpdateUpVaultAttributes<'a> {
    ground: &'a mut SkateboardGroundNormalVaultAttributes,
    physics: &'a AttributeRuntimePhysics,
}
impl<'a> SkateboardUpdateUpVaultAttributes<'a> {
    pub(crate) fn new(
        ground: &'a mut SkateboardGroundNormalVaultAttributes,
        physics: &'a AttributeRuntimePhysics,
    ) -> Self {
        Self { ground, physics }
    }
    fn check_physics(
        &self,
        token: &SkateboardUpdateUpPhysicsToken<'_>,
    ) -> Result<(), SkateboardUpdateUpAttributeError> {
        if std::ptr::eq(self.physics, token.0) {
            Ok(())
        } else {
            Err(SkateboardUpdateUpAttributeError::ForeignPhysics)
        }
    }
}
impl SkateboardGroundNormalAttributeRead for SkateboardUpdateUpVaultAttributes<'_> {
    type Global = SkateboardGroundNormalGlobal;
    type Pointer = SkateboardGroundNormalPointer;
    type Error = SkateboardUpdateUpAttributeError;
    fn capture_global(&mut self) -> Result<Self::Global, Self::Error> {
        self.ground.capture_global().map_err(Self::Error::Ground)
    }
    fn lookup(&mut self, g: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.ground.lookup(g, key).map_err(Self::Error::Ground)
    }
    fn float(&mut self, p: &Self::Pointer) -> Result<f32, Self::Error> {
        self.ground.float(p).map_err(Self::Error::Ground)
    }
}
impl<'a> SkateboardUpdateUpAttributes for SkateboardUpdateUpVaultAttributes<'a> {
    type PhysicsGlobal = SkateboardUpdateUpPhysicsToken<'a>;
    type PhysicsLayout = SkateboardUpdateUpPhysicsToken<'a>;
    type ReckoningLayout = SkateboardUpdateUpReckoningLayout;
    fn capture_physics_global(&mut self) -> Result<Self::PhysicsGlobal, Self::Error> {
        Ok(SkateboardUpdateUpPhysicsToken(self.physics))
    }
    fn capture_physics_layout(
        &mut self,
        g: &Self::PhysicsGlobal,
    ) -> Result<Self::PhysicsLayout, Self::Error> {
        self.check_physics(g)?;
        Ok(SkateboardUpdateUpPhysicsToken(g.0))
    }
    fn world_gravity(
        &mut self,
        l: &Self::PhysicsLayout,
    ) -> Result<SkeletonRootVector, Self::Error> {
        self.check_physics(l)?;
        let layout = l.0.try_physics_layout().map_err(Self::Error::Physics)?;
        Ok(SkeletonRootVector::known(std::array::from_fn(|i| {
            f32::from_be_bytes(
                layout.raw_bytes[i * 4..i * 4 + 4]
                    .try_into()
                    .expect("physics layout first four words"),
            )
        })))
    }
    fn capture_reckoning_layout(
        &mut self,
        g: &Self::Global,
    ) -> Result<Self::ReckoningLayout, Self::Error> {
        let instance = self
            .ground
            .checked_instance(g)
            .map_err(Self::Error::Ground)?
            .clone();
        Ok(SkateboardUpdateUpReckoningLayout {
            identity: Arc::clone(self.ground.occurrence()),
            instance,
        })
    }
    fn evaluate_graph8(
        &mut self,
        l: &Self::ReckoningLayout,
        offset: usize,
        input: f32,
    ) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(&l.identity, self.ground.occurrence()) {
            return Err(Self::Error::ForeignLayout);
        }
        ReckoningCapturedLayout::try_capture(&l.instance)
            .and_then(|layout| layout.evaluate_graph8(offset, input))
            .map_err(Self::Error::Graph)
    }
}
#[cfg(test)]
#[path = "skateboard_update_up_attributes_tests.rs"]
mod tests;
