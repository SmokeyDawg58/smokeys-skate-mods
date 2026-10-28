//! LWDynamicObjectMan box grab query 82d30a40 and spline refresh 82d2db78.
use super::dmo_objects::DmoObjectMan;
use crate::{
    dynamic_assembly_session::source_parts::SourcePartGrabSnapshot,
    input_cvar_runtime::InputCvarReferences,
    physical_player_grab_spline::{
        GrabQueryBuffer, GrabSpline,
        geometry::{GrabBox, transform_point},
    },
    physical_player_grind_manager::{
        math::{GrindNum, V4, sub},
        view::{GrindError, error},
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct GrabBoxQuery {
    pub(crate) reach_start: V4,
    pub(crate) bounds: GrabBox,
    pub(crate) end_exclusion: f32,
    pub(crate) angle_limit: f32,
    pub(crate) horizontal_limit: f32,
}

impl super::LivingWorldGrindServices {
    /// GrabSplineQuery827b1ed8 -> authored LWVehicleMan82d22e80. Uses the
    /// SAME retained vehicle map, registered splines and attached Part owner.
    pub(crate) fn try_vehicle_grab_radius(
        &self,
        position: V4,
        radius: f32,
        output: &mut GrabQueryBuffer<32>,
        cvars: &InputCvarReferences,
        read_part: &mut impl FnMut(
            &SimulationDmoAssemblyHandle,
        ) -> Result<Option<SourcePartGrabSnapshot>, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        if cvars
            .try_use_authored_vehicle_grab_splines()
            .map_err(error)?
        {
            // LWVehicleMan.GrabSplineQuery82d22e80: feed gate, entity map,
            // kMaxVechileObjectSize + radius, then entity traversal.
            if !cvars.try_feed_vehicle_grab_splines().map_err(error)? {
                return Ok(());
            }
            let entities = self
                .grind_vehicle_entities
                .as_ref()
                .ok_or_else(|| GrindError("vehicle grab missing EntityMan".into()))?;
            let ids = entities.ids();
            let range = cvars.try_max_vehicle_grind_object_size().map_err(error)? + radius;
            let range_squared = range * range;
            if !entities.initialized() {
                return Err(GrindError(
                    "vehicle grab EntityMan is not initialized".into(),
                ));
            }
            for entity_id in ids {
                let object = entities.get(entity_id).ok_or_else(|| {
                    GrindError(format!("vehicle grab missing map member {entity_id:08x}"))
                })?;
                let transform = object.core.target_transform;
                let delta = sub(transform[3], position);
                // Ordered VMX >=82d23034: unordered does not reject.
                if num.dot(delta, delta)? >= range_squared {
                    continue;
                }
                for (&id, source) in &object.grab_splines {
                    let data = source.grab();
                    if data.spline_points.is_empty() || data.approach_vectors.is_empty() {
                        continue;
                    }
                    // Vehicle query tests ONLY the first-to-last chord; DMO
                    // box query below walks every nondegenerate segment.
                    if !sphere_intersects(
                        transform_point(&transform, data.spline_points[0]),
                        transform_point(&transform, *data.spline_points.last().expect("nonempty")),
                        position,
                        radius,
                        num,
                    )? {
                        continue;
                    }
                    let Some(body) = object
                        .collision
                        .as_ref()
                        .and_then(|collision| collision.attached_assembly_handle())
                    else {
                        continue;
                    };
                    // GetVelocity82d36158 reads core+90, not Part velocity.
                    let velocity = object.core.velocity;
                    if output.result_count >= 32 {
                        continue;
                    }
                    let index = output.result_count as usize;
                    output.result_count += 1;
                    output.splines[index].init_living_world(
                        1,
                        id,
                        source.clone(),
                        transform,
                        velocity,
                        body.clone(),
                        read_part(body)?,
                        num,
                    )?;
                }
            }
        } else {
            // Legacy provider uses the same vehicle KD-tree as other queries.
            let mut count = 0;
            self.try_get_vehicles_within_range(
                [position[0], position[2]],
                radius,
                24,
                &mut count,
                &mut [0; 24],
                &mut [0.0; 24],
            )
            .map_err(error)?;
            if count != 0 {
                return Err(GrindError(
                    "populated legacy vehicle grab provider is not connected".into(),
                ));
            }
        }
        Ok(())
    }

    /// LWVehicleMan.GetGrabSpline82d1ed40: owner map -> active entity ->
    /// local spline -> collision Body.mIsland -> SAME Init. No feed cvar gate
    /// and failed lookup leaves the caller's previous result untouched.
    pub(crate) fn get_vehicle_grab_spline(
        &self,
        id: u32,
        output: &mut GrabSpline,
        read_part: &mut impl FnMut(
            &SimulationDmoAssemblyHandle,
        ) -> Result<Option<SourcePartGrabSnapshot>, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, GrindError> {
        let entity_id = self.vehicle_grabbables.owner(id);
        if entity_id == 0 {
            return Ok(false);
        }
        let entities = self
            .grind_vehicle_entities
            .as_ref()
            .ok_or_else(|| GrindError("vehicle grab refresh missing EntityMan".into()))?;
        let Some(object) = entities.get(entity_id) else {
            return Ok(false);
        };
        let Some(source) = object.grab_splines.get(&id) else {
            return Ok(false);
        };
        if source.grab().spline_points.is_empty() || source.grab().approach_vectors.is_empty() {
            return Ok(false);
        }
        let Some(body) = object
            .collision
            .as_ref()
            .and_then(|collision| collision.attached_assembly_handle())
        else {
            return Ok(false);
        };
        output.init_living_world(
            1,
            id,
            source.clone(),
            object.core.target_transform,
            object.core.velocity,
            body.clone(),
            read_part(body)?,
            num,
        )?;
        Ok(true)
    }
}

/// The return-value path of SphereVolume::LineSegIntersect 82ad9500 ->
/// rwcSphereLineSegIntersect 82acdef0. Unlike the formats crate's generic
/// analytic sphere helper, this uses the native cross-product discriminant,
/// fused fmsubs and strict initial-overlap test, without an epsilon cutoff.
/// The caller doesn't consume the hit point, fraction or surface normal.
fn sphere_intersects(
    start: V4,
    end: V4,
    centre: V4,
    radius: f32,
    num: &mut GrindNum<'_>,
) -> Result<bool, GrindError> {
    Ok(crate::collision_sphere_line::intersect(
        start, sub(end, start), centre, radius, |a, b| num.dot(a, b),
    )?.intersects())
}

impl DmoObjectMan {
    /// Feed existing output in native entity-map, then ascending spline-ID
    /// order. A full output doesn't terminate source traversal; failed
    /// CanGrabSpline removes only the just-added result.
    pub(crate) fn query_grab_splines(
        &self,
        query: &GrabBoxQuery,
        output: &mut GrabQueryBuffer<32>,
        cvars: &InputCvarReferences,
        read_part: &mut impl FnMut(
            &SimulationDmoAssemblyHandle,
        ) -> Result<Option<SourcePartGrabSnapshot>, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        if !cvars.try_feed_dmo_grab_splines().map_err(error)? {
            return Ok(());
        }
        let radius = num.length(query.bounds.half_dimensions)?;
        let range = cvars.try_max_dmo_grind_object_size().map_err(error)? + radius;
        let range_squared = range * range;
        let centre = query.bounds.transform[3];
        for id in self.ids() {
            let Some(object) = self.object(id) else {
                continue;
            };
            // IDynamicObject vtable+0x50 is IsStatic (not IsInUseByPlayer).
            if object.is_static() {
                continue;
            }
            let transform: [V4; 4] = std::array::from_fn(|row| {
                std::array::from_fn(|lane| object.target_transform[row * 4 + lane])
            });
            let delta = sub(transform[3], centre);
            if num.dot(delta, delta)? >= range_squared {
                continue;
            }
            for (&id, source) in &object.grab_splines {
                let data = source.grab();
                if data.spline_points.len() <= 1 || data.approach_vectors.is_empty() {
                    continue;
                }
                let mut intersects = false;
                for points in data.spline_points.windows(2) {
                    let delta = sub(points[1], points[0]);
                    if num.dot(delta, delta)? > f32::from_bits(0x3727_c5ac)
                        && sphere_intersects(
                            transform_point(&transform, points[0]),
                            transform_point(&transform, points[1]),
                            centre,
                            radius,
                            num,
                        )?
                    {
                        intersects = true;
                        break;
                    }
                }
                if !intersects {
                    continue;
                }
                let part = read_part(&object.body)?;
                let velocity = part
                    .as_ref()
                    .and_then(|part| part.velocity)
                    .ok_or_else(|| {
                        GrindError("DMO GetPartVelocity needs attached Part 0".into())
                    })?;
                if output.result_count >= 32 {
                    continue;
                }
                let index = output.result_count as usize;
                output.result_count += 1;
                output.splines[index].init_dmo(
                    id,
                    source.clone(),
                    transform,
                    velocity,
                    object.body.clone(),
                    part,
                    num,
                )?;
                if !output.splines[index].can_grab(
                    query.reach_start,
                    &query.bounds,
                    query.end_exclusion,
                    query.angle_limit,
                    query.horizontal_limit,
                    num,
                )? {
                    output.result_count -= 1;
                }
            }
        }
        Ok(())
    }

    /// Refresh follows the owner map then the live entity map. A pending
    /// cull therefore returns false even before DeInit removes its spline ID.
    /// The source leaves output unchanged on a failed lookup.
    pub(crate) fn get_grab_spline(
        &self,
        id: u32,
        output: &mut GrabSpline,
        read_part: &mut impl FnMut(
            &SimulationDmoAssemblyHandle,
        ) -> Result<Option<SourcePartGrabSnapshot>, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, GrindError> {
        let Some(object) = self.object(self.grab_spline_owner(id)) else {
            return Ok(false);
        };
        let Some(source) = object.grab_splines.get(&id) else {
            return Ok(false);
        };
        if source.grab().spline_points.is_empty() || source.grab().approach_vectors.is_empty() {
            return Ok(false);
        }
        let part = read_part(&object.body)?;
        let velocity = part
            .as_ref()
            .and_then(|part| part.velocity)
            .ok_or_else(|| GrindError("DMO GetPartVelocity needs attached Part 0".into()))?;
        let transform = std::array::from_fn(|row| {
            std::array::from_fn(|lane| object.target_transform[row * 4 + lane])
        });
        output.init_dmo(
            id,
            source.clone(),
            transform,
            velocity,
            object.body.clone(),
            part,
            num,
        )?;
        Ok(true)
    }
}
