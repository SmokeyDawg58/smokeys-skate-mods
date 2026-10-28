//! Common retained tEmbeddedData grab/grind reference projection. DMO banks
//! and character recipes use the SAME parser and validated inline headers.
use super::dmo_models::{DmoGrabSpline, DmoGrindSpline};
use crate::grind_data_runtime::{GrindDataSourceError, GrindSplineSource};
use skate2_formats::{
    pegasus::{
        EmbeddedData, GRAB_DATA_TYPE_ID, GrabData, ObjectId, PegasusError, SPLINE_DATA_TYPE_ID,
    },
    rw4::{Arena, ArenaError, ArenaSections},
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) type ModelSplines = (Vec<DmoGrindSpline>, Vec<DmoGrabSpline>);
#[derive(Debug)]
pub(crate) enum EmbeddedSplineError {
    Sections(ArenaError),
    Unresolved(ObjectId),
    Parse(PegasusError),
    Source(GrindDataSourceError),
    Allocation,
}
impl std::fmt::Display for EmbeddedSplineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "embedded splines: {self:?}")
    }
}
impl std::error::Error for EmbeddedSplineError {}

/// Retains the recipe arena's original instance -> embedded-reference mapping
/// during the existing geometry load. No additional file IO or asset registry.
pub(crate) fn decode_character_arena(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    collision: &Arc<crate::world::CollisionArena>,
    asset_id: u64,
) -> Result<BTreeMap<u64, ModelSplines>, Box<dyn std::error::Error>> {
    use skate2_formats::pegasus::{INSTANCE_DATA_TYPE_ID, InstanceData};
    let mut result = BTreeMap::new();
    let sections = arena.sections(resources)?;
    for entry in &arena.dictionary {
        if entry.type_id != INSTANCE_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or("missing recipe instance bytes")?;
        for instance in InstanceData::parse(bytes)?.instances {
            let Some(embedded) = instance.embedded_data else {
                continue;
            };
            let (index, offset) = resolve_reference(&sections, embedded)
                .ok_or(EmbeddedSplineError::Unresolved(embedded))?;
            let entry = arena
                .dictionary
                .get(index as usize)
                .ok_or(EmbeddedSplineError::Unresolved(embedded))?;
            let bytes = arena
                .object_bytes(resources, entry)
                .and_then(|bytes| bytes.get(offset as usize..))
                .ok_or(EmbeddedSplineError::Unresolved(embedded))?;
            let embedded = EmbeddedData::parse(bytes)?;
            let splines = decode(arena, resources, &embedded, &mut |index, bytes| {
                GrindSplineSource::try_from_character_spline_data(
                    Arc::clone(collision),
                    index,
                    asset_id,
                    bytes,
                )
            })?;
            if result.insert(instance.guid, splines).is_some() {
                return Err("duplicate recipe embedded instance GUID".into());
            }
        }
    }
    Ok(result)
}

pub(crate) fn resolve_reference(sections: &ArenaSections, object: ObjectId) -> Option<(u32, u32)> {
    match object.section_index() {
        0 => Some((object.entry_index(), 0)),
        2 => {
            let reference = sections.subreferences.get(object.entry_index() as usize)?;
            (reference.object.section_index() == 0)
                .then_some((reference.object.entry_index(), reference.object_offset))
        }
        _ => None,
    }
}

pub(crate) fn decode(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    embedded: &EmbeddedData,
    make_source: &mut impl FnMut(u32, &[u8]) -> Result<GrindSplineSource, GrindDataSourceError>,
) -> Result<ModelSplines, EmbeddedSplineError> {
    use EmbeddedSplineError as E;
    let sections = arena.sections(resources).map_err(E::Sections)?;
    let mut grabs = Vec::new();
    let mut grab_data: BTreeMap<u32, Arc<GrabData>> = BTreeMap::new();
    grabs
        .try_reserve_exact(embedded.grab_splines().len())
        .map_err(|_| E::Allocation)?;
    for reference in embedded.grab_splines() {
        let (entry_index, header_offset) = resolve_reference(&sections, reference.object)
            .ok_or(E::Unresolved(reference.object))?;
        let data = if let Some(data) = grab_data.get(&entry_index) {
            Arc::clone(data)
        } else {
            let entry = arena
                .dictionary
                .get(entry_index as usize)
                .ok_or(E::Unresolved(reference.object))?;
            if entry.type_id != GRAB_DATA_TYPE_ID {
                return Err(E::Unresolved(reference.object));
            }
            let bytes = arena
                .object_bytes(resources, entry)
                .ok_or(E::Unresolved(reference.object))?;
            let data = Arc::new(GrabData::parse(bytes).map_err(E::Parse)?);
            grab_data.insert(entry_index, Arc::clone(&data));
            data
        };
        if data.grab_at_object_offset(header_offset).is_none() {
            return Err(E::Unresolved(reference.object));
        }
        grabs.push(DmoGrabSpline {
            entry_index,
            header_offset,
            data,
        });
    }
    let mut grinds = Vec::new();
    grinds
        .try_reserve_exact(embedded.grind_splines().len())
        .map_err(|_| E::Allocation)?;
    for reference in embedded.grind_splines() {
        let (entry_index, header_offset) = resolve_reference(&sections, reference.object)
            .ok_or(E::Unresolved(reference.object))?;
        let entry = arena
            .dictionary
            .get(entry_index as usize)
            .ok_or(E::Unresolved(reference.object))?;
        if entry.type_id != SPLINE_DATA_TYPE_ID {
            return Err(E::Unresolved(reference.object));
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or(E::Unresolved(reference.object))?;
        let source = make_source(entry_index, bytes).map_err(E::Source)?;
        if source
            .data()
            .spline_at_object_offset(header_offset)
            .is_none()
        {
            return Err(E::Unresolved(reference.object));
        }
        grinds.push(DmoGrindSpline {
            source,
            header_offset,
        });
    }
    Ok((grinds, grabs))
}
