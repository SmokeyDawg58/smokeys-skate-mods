//! `LivingWorld` Manager82703ff0: DMO then vehicles, including a full result.
#![expect(
    clippy::manual_midpoint,
    reason = "native adds the bounds before halving, preserving source rounding and overflow"
)]
use super::{LivingWorldGrindServices, dmo_objects::DmoObjectMan};
use crate::{
    grind_data_octree::OctreeBounds,
    grind_data_runtime::{DynamicGrindSplineProvider, GrindDataPrimitive, GrindDataQueryResult},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::host_dot3,
    world::transform_collision_aabb,
};

/// `tSplineSegment` stride walked from `m_Head` to `m_Tail` (0x82d312a4).
const SPLINE_SEGMENT_STRIDE: u32 = 0x90;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityKind {
    Dmo,
    Vehicle,
}
#[derive(Debug)]
pub(crate) enum LivingWorldGrindError {
    Cvar(InputCvarError),
    MissingEntityMan(EntityKind),
    MissingVehicle(u32),
    /// An embedded LivingWorld spline whose `m_Head..m_Tail` walk leaves its segments.
    MalformedSpline {
        kind: EntityKind,
        entity_id: u32,
    },
}
impl std::fmt::Display for LivingWorldGrindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "LivingWorld grind cvar: {error:?}"),
            Self::MissingEntityMan(kind) => write!(f, "LivingWorld {kind:?} EntityMan missing"),
            Self::MissingVehicle(id) => {
                write!(f, "LivingWorld vehicle {id:08x} absent from its map")
            }
            Self::MalformedSpline { kind, entity_id } => {
                write!(
                    f,
                    "LivingWorld {kind:?} {entity_id:08x} grind spline leaves its segments"
                )
            }
        }
    }
}
/// Borrow the SAME workspace `LivingWorld` occurrence; never construct per query.
pub(crate) struct LivingWorldGrindProvider<'a> {
    pub(crate) living_world: &'a LivingWorldGrindServices,
    pub(crate) cvars: &'a InputCvarReferences,
}
impl DynamicGrindSplineProvider for LivingWorldGrindProvider<'_> {
    type Error = LivingWorldGrindError;
    fn query(
        &mut self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
    ) -> Result<(), Self::Error> {
        use LivingWorldGrindError as E;
        //0fd4..e0. A disabled feed does not dereference the DMO EntityMan.
        if self.cvars.try_feed_dmo_grind_splines().map_err(E::Cvar)? {
            let owner = self
                .living_world
                .dmo_objects
                .as_ref()
                .ok_or(E::MissingEntityMan(EntityKind::Dmo))?; //0ff0
            let center =
                std::array::from_fn(|lane| (bounds.maximum[lane] + bounds.minimum[lane]) * 0.5); //1030, all4
            let size = self
                .cvars
                .try_max_dmo_grind_object_size()
                .map_err(E::Cvar)?; //1058
            let radius_squared = size * size; //105c
            query_dmo_grind_splines(owner, &bounds, center, radius_squared, result)?;
        }
        let owner = self
            .living_world
            .grind_vehicle_entities
            .as_ref()
            .ok_or(E::MissingEntityMan(EntityKind::Vehicle))?; //37b8
        let center =
            std::array::from_fn(|lane| (bounds.maximum[lane] + bounds.minimum[lane]) * 0.5); //380c
        let ids = owner.ids(); //3838/3854: capture native map walk before size
        let size = self
            .cvars
            .try_max_vehicle_grind_object_size()
            .map_err(E::Cvar)?; //3858
        let radius_squared = size * size; //385c
        for id in ids {
            let object = owner.get(id).ok_or(E::MissingVehicle(id))?;
            let transform = std::array::from_fn(|i| object.core.target_transform[i / 4][i % 4]);
            if !query_entity_grind_splines(
                EntityKind::Vehicle,
                id,
                &transform,
                object.grind_splines.values(),
                &bounds,
                center,
                radius_squared,
                result,
            )? {
                break;
            }
        }
        Ok(())
    }
}
/// `fma(z, p.z, fma(y, p.y, fma(x, p.x, w)))`, the `vmaddfp128` chain of
/// 0x82d3120c..1218 (all four lanes).
fn transform_point(transform: &[f32; 16], point: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = transform[lane].mul_add(point[0], transform[12 + lane]);
        let y = transform[4 + lane].mul_add(point[1], x);
        transform[8 + lane].mul_add(point[2], y)
    })
}

/// `LWDynamicObjectMan::QueryGrindSplinesWithBoundingBox` 0x82d30fb0: every
/// entity in map order whose target translation lies inside
/// `kMaxDynamicObjectSize`; each embedded spline's segments whose transformed
/// AABB overlaps the query box append their transformed endpoints
/// (`row3`, `SplineEval_One`) until the result is full. The flags word's bit
/// 31 comes from an uninitialized stack word (0x82d31080) natively; the port
/// stores zero (not vertical).
fn query_dmo_grind_splines(
    owner: &DmoObjectMan,
    bounds: &OctreeBounds,
    center: [f32; 4],
    radius_squared: f32,
    result: &mut GrindDataQueryResult,
) -> Result<(), LivingWorldGrindError> {
    for id in owner.ids() {
        let Some(object) = owner.object(id) else {
            continue;
        };
        if !query_entity_grind_splines(
            EntityKind::Dmo,
            id,
            &object.target_transform,
            object.grind_splines.values(),
            bounds,
            center,
            radius_squared,
            result,
        )? {
            break;
        }
    }
    Ok(())
}

/// Shared segment kernel. Vehicle82d237a0 raw VMX matches DMO82d30fb0:
/// target pose, ordered radius>= rejection, inclusive AABB boundary, head..tail
/// stride0x90 and endpoint FMA chains. false means native result capacity40 hit.
pub(crate) fn query_entity_grind_splines<'a>(
    kind: EntityKind,
    id: u32,
    transform: &[f32; 16],
    splines: impl Iterator<Item = &'a super::dmo_models::DmoGrindSpline>,
    bounds: &OctreeBounds,
    center: [f32; 4],
    radius_squared: f32,
    result: &mut GrindDataQueryResult,
) -> Result<bool, LivingWorldGrindError> {
    let delta: [f32; 4] = std::array::from_fn(|lane| transform[12 + lane] - center[lane]);
    if host_dot3(delta, delta) >= radius_squared {
        return Ok(true);
    }
    for spline in splines {
        let data = spline.source.data();
        let Some(header) = data.spline_at_object_offset(spline.header_offset) else {
            continue;
        };
        let (Some(head), Some(tail)) = (header.head_offset, header.tail_offset) else {
            continue;
        };
        let mut offset = head;
        while offset <= tail {
            let segment = data.segment_at_object_offset(offset).ok_or(
                LivingWorldGrindError::MalformedSpline {
                    kind,
                    entity_id: id,
                },
            )?;
            let (minimum, maximum) = transform_collision_aabb(
                [
                    segment.bounding_box_min[0],
                    segment.bounding_box_min[1],
                    segment.bounding_box_min[2],
                ],
                [
                    segment.bounding_box_max[0],
                    segment.bounding_box_max[1],
                    segment.bounding_box_max[2],
                ],
                *transform,
            );
            let separated = (0..3).any(|axis| {
                bounds.minimum[axis] > maximum[axis] || minimum[axis] > bounds.maximum[axis]
            });
            if !separated {
                let basis = &segment.basis_matrix;
                let start_local = [basis[12], basis[13], basis[14], basis[15]];
                // SplineEval_One 0x82e49908: (r0 + r1) + (r2 + r3), W = X.
                let mut end_local: [f32; 4] = std::array::from_fn(|lane| {
                    (basis[lane] + basis[4 + lane]) + (basis[8 + lane] + basis[12 + lane])
                });
                end_local[3] = end_local[0];
                let relative = offset.checked_sub(data.segments_offset).ok_or(
                    LivingWorldGrindError::MalformedSpline {
                        kind,
                        entity_id: id,
                    },
                )?;
                let appended = result.try_append(GrindDataPrimitive {
                    start: transform_point(transform, start_local),
                    end: transform_point(transform, end_local),
                    source: spline.source.clone(),
                    segment_index: relative / SPLINE_SEGMENT_STRIDE,
                    spline_header_offset: Some(spline.header_offset),
                    vertical: false,
                });
                if !appended {
                    return Ok(false);
                }
            }
            offset += SPLINE_SEGMENT_STRIDE;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicGrindSplineProvider, EntityKind, GrindDataQueryResult, LivingWorldGrindError,
        LivingWorldGrindProvider, LivingWorldGrindServices, OctreeBounds,
    };
    use crate::input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences};
    fn bounds() -> OctreeBounds {
        OctreeBounds {
            minimum: [-1.0, -2.0, -3.0, 13.0],
            maximum: [7.0, 8.0, 9.0, 17.0],
        }
    }
    #[test]
    fn missing_owner_errors_follow_feed_read_and_dmo_then_vehicle_order() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut living = LivingWorldGrindServices::new();
        let mut result = GrindDataQueryResult::new();
        assert!(matches!(
            LivingWorldGrindProvider {
                living_world: &living,
                cvars: &refs
            }
            .query(bounds(), &mut result),
            Err(LivingWorldGrindError::Cvar(InputCvarError::Unlinked(
                "kFeedDMOGrindSplines"
            )))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(matches!(
            LivingWorldGrindProvider {
                living_world: &living,
                cvars: &refs
            }
            .query(bounds(), &mut result),
            Err(LivingWorldGrindError::MissingEntityMan(EntityKind::Dmo))
        ));
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkFeedDMOGrindSplines=0\r\n"))
            .unwrap();
        assert!(matches!(
            LivingWorldGrindProvider {
                living_world: &living,
                cvars: &refs
            }
            .query(bounds(), &mut result),
            Err(LivingWorldGrindError::MissingEntityMan(EntityKind::Vehicle))
        ));
        let vault = std::sync::Arc::new(crate::attribute_runtime::tests::graph(0, vec![]));
        living.try_initialize_selected_entities(&vault).unwrap();
        LivingWorldGrindProvider {
            living_world: &living,
            cvars: &refs,
        }
        .query(bounds(), &mut result)
        .unwrap();
        assert_eq!(result.count(), 0);
    }
}
