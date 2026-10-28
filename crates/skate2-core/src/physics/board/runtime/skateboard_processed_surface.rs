//! `SurfacePhysics` constructor/Dispatch 0x82dd6060/0x82dd63e8 selection projection.
//! Consumes published collision type; does not invent contact/PhysOut scheduling.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "retained surface selection awaits grounded owner")
)]

use crate::skateboard_slide_collision_attributes::{
    SkateboardSelectedSurface, SkateboardSlideCollisionAttributeRead,
    SkateboardSlideCollisionVaultAttributes, SlideCollisionGlobal, SlideSurfaceField,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

/// A selected source material slot, not just an equal coefficient triple.
/// Tokens exist only after the one-shot constructor completes.
#[derive(Clone, Debug)]
pub(crate) struct SkateboardSelectedMaterial {
    occurrence: Arc<()>,
    slot: usize,
    material: Arc<AssemblyPartMaterial>,
}
impl SkateboardSelectedMaterial {
    pub(crate) fn material_arc(&self) -> &Arc<AssemblyPartMaterial> {
        &self.material
    }
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.occurrence, &other.occurrence)
            && self.slot == other.slot
            && Arc::ptr_eq(&self.material, &other.material)
    }
}

const COLLECTIONS: [u64; 6] = [
    0xd7ed_bd36_2d7d_2152,
    0x8e61_e92e_8922_5fbe,
    0xa28a_048a_da29_0408,
    0xcb95_5af8_908f_2d65,
    0x9609_09a8_602b_4070,
    0x4ad0_cd34_9768_3aec,
];

const MATERIAL_DEFAULT: AssemblyPartMaterial = AssemblyPartMaterial {
    static_friction: 0.5,
    dynamic_friction: 0.3,
    restitution: 0.4,
};
const WHEEL_RESTITUTION: u64 = 0x5d09_5e63_6526_0b84;

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardSurfaceDispatchError<E> {
    Publication(E),
    MaterialsNotReady,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardSurfaceConstructionError<E> {
    AlreadyStarted,
    Attribute(E),
}

/// Writes are separate source stages. Implementations must preserve previous
/// writes when a later publication fails. No attribute value preflight belongs here.
pub(crate) trait SkateboardSurfacePublication {
    type Error;
    fn publish_physics_type(&mut self, value: u32) -> Result<(), Self::Error>;
    fn publish_collection(&mut self, value: SkateboardSelectedSurface) -> Result<(), Self::Error>;
    fn publish_material(&mut self, value: SkateboardSelectedMaterial) -> Result<(), Self::Error>;
}

pub(crate) struct SkateboardProcessedSurface {
    // The source constructs default before the five selected instances even
    // though Dispatch never publishes default itself.
    surfaces: [SkateboardSelectedSurface; 6],
    occurrence: Arc<()>,
    materials: [Arc<AssemblyPartMaterial>; 5],
    materials_ready: bool,
    construction_started: bool,
}
impl SkateboardProcessedSurface {
    /// 0x82dd6094..6134: same source collection order, retained once. This is
    /// only the six instance bindings, not wheel-material coefficient construction.
    pub(crate) fn bind(attributes: &SkateboardSlideCollisionVaultAttributes) -> Self {
        Self {
            surfaces: COLLECTIONS.map(|key| attributes.bind_selected_surface(key)),
            occurrence: Arc::new(()),
            materials: std::array::from_fn(|_| Arc::new(MATERIAL_DEFAULT)),
            materials_ready: false,
            construction_started: false,
        }
    }

    /// Material constructor suffix 0x82dd6164..63dc. Partial writes survive
    /// errors. A constructor cannot be replayed on this retained owner.
    pub(crate) fn try_initialize_materials<
        A: SkateboardSlideCollisionAttributeRead<Surface = SkateboardSelectedSurface>,
    >(
        &mut self,
        attributes: &mut A,
    ) -> Result<(), SkateboardSurfaceConstructionError<A::Error>> {
        use SkateboardSurfaceConstructionError::{AlreadyStarted, Attribute};
        if self.construction_started {
            return Err(AlreadyStarted);
        }
        self.construction_started = true;
        for material in &mut self.materials[..4] {
            let global = attributes
                .capture_global(SlideCollisionGlobal::Wheels)
                .map_err(Attribute)?;
            let pointer = attributes
                .lookup(&global, WHEEL_RESTITUTION)
                .map_err(Attribute)?;
            let restitution = attributes.float(&pointer).map_err(Attribute)?;
            Arc::get_mut(material)
                .expect("no tokens before construction completion")
                .restitution = restitution;
        }
        let global = attributes
            .capture_global(SlideCollisionGlobal::Wheels)
            .map_err(Attribute)?;
        let pointer = attributes
            .lookup(&global, WHEEL_RESTITUTION)
            .map_err(Attribute)?;
        // 0x82dd6370 precedes fifth restitution lfs6374/store6378.
        let smooth_layout = attributes
            .capture_surface_layout(&self.surfaces[1])
            .map_err(Attribute)?;
        let restitution = attributes.float(&pointer).map_err(Attribute)?;
        Arc::get_mut(&mut self.materials[4])
            .expect("no tokens before construction completion")
            .restitution = restitution;
        let mut smooth_layout = Some(smooth_layout);
        for (index, material) in self.materials.iter_mut().enumerate() {
            let layout = if let Some(layout) = smooth_layout.take() {
                layout
            } else {
                attributes
                    .capture_surface_layout(&self.surfaces[index + 1])
                    .map_err(Attribute)?
            };
            let dynamic = attributes
                .layout_float(&layout, SlideSurfaceField::WheelDynamicFriction)
                .map_err(Attribute)?;
            let static_friction = attributes
                .layout_float(&layout, SlideSurfaceField::WheelStaticFriction)
                .map_err(Attribute)?;
            let material =
                Arc::get_mut(material).expect("no tokens before construction completion");
            material.static_friction = static_friction;
            material.dynamic_friction = dynamic;
        }
        self.materials_ready = true;
        Ok(())
    }
    pub(crate) fn material_prefix(&self) -> [AssemblyPartMaterial; 5] {
        std::array::from_fn(|index| *self.materials[index])
    }
    pub(crate) fn owns_material(&self, material: &SkateboardSelectedMaterial) -> bool {
        self.materials_ready
            && Arc::ptr_eq(&self.occurrence, &material.occurrence)
            && self
                .materials
                .get(material.slot)
                .is_some_and(|slot| Arc::ptr_eq(slot, &material.material))
    }

    /// 0x82dda0b0 then 0x82dd63e8. Caller supplies the actual published
    /// `PhysOut_Collision` type, not an arbitrary line result's packed surface ID.
    pub(crate) fn dispatch_from_published_type<P: SkateboardSurfacePublication>(
        &self,
        incoming: u32,
        output: &mut P,
    ) -> Result<(), SkateboardSurfaceDispatchError<P::Error>> {
        use SkateboardSurfaceDispatchError::Publication;
        output.publish_physics_type(incoming).map_err(Publication)?;
        let normalized = match incoming {
            1..=5 => incoming,
            6 => 3,
            _ => 1,
        };
        if incoming != normalized {
            output
                .publish_physics_type(normalized)
                .map_err(Publication)?;
        }
        output
            .publish_collection(self.surfaces[normalized as usize].clone())
            .map_err(Publication)?;
        if !self.materials_ready {
            return Err(SkateboardSurfaceDispatchError::MaterialsNotReady);
        }
        let slot = normalized as usize - 1;
        output
            .publish_material(SkateboardSelectedMaterial {
                occurrence: Arc::clone(&self.occurrence),
                slot,
                material: Arc::clone(&self.materials[slot]),
            })
            .map_err(Publication)
    }
}

/// Readiness is explicit: no guessed collection/material exists before dispatch.
/// This snapshot sink cannot be mutated by native callback aliases during a write.
#[derive(Default)]
pub(crate) struct SkateboardSurfaceProjection {
    pub(crate) physics_type: Option<u32>,
    pub(crate) selected: Option<SkateboardSelectedSurface>,
    pub(crate) material: Option<SkateboardSelectedMaterial>,
}
impl SkateboardSurfaceProjection {
    pub(crate) const fn new() -> Self {
        Self {
            physics_type: None,
            selected: None,
            material: None,
        }
    }
}
impl SkateboardSurfacePublication for SkateboardSurfaceProjection {
    type Error = std::convert::Infallible;
    fn publish_physics_type(&mut self, value: u32) -> Result<(), Self::Error> {
        self.physics_type = Some(value);
        Ok(())
    }
    fn publish_collection(&mut self, value: SkateboardSelectedSurface) -> Result<(), Self::Error> {
        self.selected = Some(value);
        Ok(())
    }
    fn publish_material(&mut self, value: SkateboardSelectedMaterial) -> Result<(), Self::Error> {
        self.material = Some(value);
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_processed_surface_tests.rs"]
pub(crate) mod tests;
