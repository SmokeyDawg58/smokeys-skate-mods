//! `Skateboard::SetStandard` 0x82b355a0 and its actual Body mutations.
//! Services must operate on the same retained Body/material/volume owners;
//! this leaf does not itself establish a live World publication.
use crate::skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead};
use std::fmt;

#[cfg(test)]
#[path = "skateboard_set_standard_tests.rs"]
mod tests;

const ANGULAR_DAMPING: u64 = 0x8d30_65ce_d858_a7df;
const ANGULAR_DAMPING_SCALE: f32 = f32::from_bits(0x426f_ffff);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardStandardMaterial {
    Deck,
    Wheel,
    Truck,
}

/// Captures preserve pointer-read stages without prefetching later body fields.
/// Materials identifies the three embedded records of this exact Body; stores
/// must retain their identities. Volume effects are bit-one RMW operations on
/// the actual shared runtime flags, with publication invalidation as needed.
pub(crate) trait SkateboardSetStandardServices {
    type Error;
    type Body;
    type Assembly;
    type Parts;
    type PartBody;
    type Inertia;
    type Materials;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn clear_free_wheel(&mut self, body: &Self::Body) -> Result<(), Self::Error>;
    fn capture_assembly(&mut self, body: &Self::Body) -> Result<Self::Assembly, Self::Error>;
    fn capture_parts(&mut self, assembly: &Self::Assembly) -> Result<Self::Parts, Self::Error>;
    fn capture_part_body(
        &mut self,
        parts: &Self::Parts,
        ordinal: usize,
    ) -> Result<Self::PartBody, Self::Error>;
    fn capture_inertia(&mut self, body: &Self::PartBody) -> Result<Self::Inertia, Self::Error>;
    fn store_angular_drag(
        &mut self,
        inertia: &Self::Inertia,
        value: f32,
    ) -> Result<(), Self::Error>;
    fn capture_standard_materials(
        &mut self,
        body: &Self::Body,
    ) -> Result<Self::Materials, Self::Error>;
    fn store_material(
        &mut self,
        parts: &Self::Parts,
        ordinal: usize,
        materials: &Self::Materials,
        selected: SkateboardStandardMaterial,
    ) -> Result<(), Self::Error>;
    /// Top0 is the deck wrapper, Top1 the shared truck volume, Top2 the shared
    /// wheel volume. These are not Part ordinals0,1,2.
    fn enable_top_volume(&mut self, body: &Self::Body, index: usize) -> Result<(), Self::Error>;
    fn deck_volume_count(&mut self, body: &Self::Body) -> Result<i32, Self::Error>;
    fn enable_deck_volume(&mut self, body: &Self::Body, index: usize) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardSetStandardError<A, S> {
    Attribute(A),
    Service(S),
}
impl<A: fmt::Display, S: fmt::Display> fmt::Display for SkateboardSetStandardError<A, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "standard board attribute: {e}"),
            Self::Service(e) => write!(f, "standard board owner: {e}"),
        }
    }
}

/// `SkateboardBody::SetStandardAngularDamping` 0x82b2b818: the global deck
/// `AngularDamping` scaled into the deck Part's inertia.
pub(crate) fn try_set_standard_angular_damping<
    A: SkateboardAttributeRead,
    S: SkateboardSetStandardServices,
>(
    attributes: &mut A,
    services: &mut S,
    body: &S::Body,
) -> Result<(), SkateboardSetStandardError<A::Error, S::Error>> {
    use SkateboardSetStandardError as E;
    let value = attributes
        .lookup(SkateboardAttributeClass::Deck, ANGULAR_DAMPING)
        .map_err(E::Attribute)?;
    // b88c Assembly before b894 scalar, b898 Parts before b8a0 multiply.
    let assembly = services.capture_assembly(body).map_err(E::Service)?;
    let value = attributes.float(&value).map_err(E::Attribute)?;
    let parts = services.capture_parts(&assembly).map_err(E::Service)?;
    store_deck_angular_drag(services, &parts, value * ANGULAR_DAMPING_SCALE).map_err(E::Service)
}

/// `SkateboardBody::SetAngularDamping` 0x82b24a78: Assembly, Parts, then the
/// scaled value into the deck Part's inertia (the grind states' Enter).
pub(crate) fn try_set_angular_damping<S: SkateboardSetStandardServices>(
    services: &mut S,
    body: &S::Body,
    damping: f32,
) -> Result<(), S::Error> {
    let assembly = services.capture_assembly(body)?;
    let parts = services.capture_parts(&assembly)?;
    store_deck_angular_drag(services, &parts, damping * ANGULAR_DAMPING_SCALE)
}

fn store_deck_angular_drag<S: SkateboardSetStandardServices>(
    services: &mut S,
    parts: &S::Parts,
    scaled: f32,
) -> Result<(), S::Error> {
    let deck = services.capture_part_body(parts, 6)?;
    let inertia = services.capture_inertia(&deck)?;
    services.store_angular_drag(&inertia, scaled)
}

/// Uses the existing GLOBAL `physicsdeck.default` provider, not the own deck
/// collection. Every error preserves preceding native stores; there is no
/// preflight of all materials/volumes and no constructor/replay state here.
pub(crate) fn try_set_standard<A: SkateboardAttributeRead, S: SkateboardSetStandardServices>(
    attributes: &mut A,
    services: &mut S,
) -> Result<(), SkateboardSetStandardError<A::Error, S::Error>> {
    use SkateboardSetStandardError as E;
    let body = services.capture_body().map_err(E::Service)?;
    services.clear_free_wheel(&body).map_err(E::Service)?;
    try_set_standard_angular_damping(attributes, services, &body)?;

    // d94 Assembly; d98..a0 derive all three embedded material addresses;
    // da4 Parts; da8 first pointer store. Subsequent stores reload Assembly.
    let assembly = services.capture_assembly(&body).map_err(E::Service)?;
    let materials = services
        .capture_standard_materials(&body)
        .map_err(E::Service)?;
    let parts = services.capture_parts(&assembly).map_err(E::Service)?;
    services
        .store_material(&parts, 6, &materials, SkateboardStandardMaterial::Deck)
        .map_err(E::Service)?;
    for ordinal in 0..6 {
        let assembly = services.capture_assembly(&body).map_err(E::Service)?;
        let parts = services.capture_parts(&assembly).map_err(E::Service)?;
        let selected = if ordinal < 4 {
            SkateboardStandardMaterial::Wheel
        } else {
            SkateboardStandardMaterial::Truck
        };
        services
            .store_material(&parts, ordinal, &materials, selected)
            .map_err(E::Service)?;
    }

    // Outer SetStandard reloads mSkateboardBody before EnableCollisionVolumes.
    let body = services.capture_body().map_err(E::Service)?;
    for index in 0..3 {
        services
            .enable_top_volume(&body, index)
            .map_err(E::Service)?;
    }
    let mut count = services.deck_volume_count(&body).map_err(E::Service)?;
    let mut index = 0_i32;
    while index < count {
        // 241ec/f0 truncate the address index to 16 bits; loop comparison uses
        // the full signed counter. Original deck counts are much smaller.
        let volume_index = usize::try_from(index & 0xffff).expect("masked nonnegative index");
        services
            .enable_deck_volume(&body, volume_index)
            .map_err(E::Service)?;
        index += 1;
        count = services.deck_volume_count(&body).map_err(E::Service)?;
    }
    Ok(())
}
