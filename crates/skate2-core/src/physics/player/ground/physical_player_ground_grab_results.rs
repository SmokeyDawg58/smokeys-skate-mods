//! PhysicsGround.ConsiderGrabSplineQueryResults82d9a9c0 on the existing query.
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::remaining_states::read_floats,
    physical_player_grab_spline::{
        PlayerGrabSpline,
        geometry::{GrabBox, inverse_transform_point, transform_point},
    },
    physical_player_grind_manager::{
        math::{DEG_TO_RAD, GrindNum, J, V4, ZERO, cross, madd, sub},
        view::{GrindError, error, lanes, need},
    },
    physical_player_physics_state::PhysicsGroundFields,
    physical_player_trajectory_owners::selector_grind::vmax,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};

#[expect(clippy::too_many_arguments)]
pub(super) fn consider(
    ground: &mut PhysicsGroundFields,
    grab: &mut PlayerGrabSpline,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    effective: [V4; 4],
    bounds: &GrabBox,
    attributes: &AttributeRuntimeInstance,
    refs: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    num: &mut GrindNum<'_>,
) -> Result<bool, GrindError> {
    ground.grab_object = false;
    ground.grab_spline_type = 0;
    ground.grab_spline_object = 0;
    ground.skitch_spline_height = 0.0;
    grab.validated.cursor = 0;
    if grab.validated.result_count as usize > grab.validated.splines.len() {
        return Err(error("ground grab count exceeds validated storage"));
    }
    while grab.validated.cursor < grab.validated.result_count {
        let spline = grab.validated.splines[grab.validated.cursor as usize].clone();
        grab.validated.cursor += 1;
        let (mut p1, mut p2) = (spline.p1, spline.p2);
        let mut direction = sub(p2, p1);
        if num.dot(direction, bounds.transform[0])? < 0.0 {
            std::mem::swap(&mut p1, &mut p2);
            direction = direction.map(|v| -v);
        }
        // Native computes this normalized horizontal normal for debug display.
        let _normal =
            crate::reckoning_numeric::try_normalize_safe(cross(direction, J), ZERO, num.n)
                .map_err(error)?;
        if refs.try_draw_grabbable_object_test().map_err(error)? {
            overlay.require().map_err(error)?;
        }
        let horizontal = refs
            .try_skitch_max_horizontal_entry_angle()
            .map_err(error)?
            * DEG_TO_RAD;
        let angle = refs.try_skitch_max_entry_angle().map_err(error)? * DEG_TO_RAD;
        let com = lanes(
            need(
                processed
                    .lock()
                    .map_err(error)?
                    .prior_output()
                    .centre_of_mass(),
                "Processed COM",
            )?
            .value,
        );
        if !spline.can_grab(com, bounds, 0.0, angle, horizontal, num)? {
            continue;
        }
        {
            let p = processed.lock().map_err(error)?;
            let timers = p.prior_wheel_timer_fields();
            let delay = need(timers.skitch_delay_until_reskitchable, "reskitch delay")?;
            // Native ble is !GT, including unordered. Only positive delays
            // compare the old vehicle ID; the descriptor type is not filtered.
            if delay > 0.0
                && need(timers.skitching_vehicle_id, "skitch vehicle ID")?
                    == spline.descriptor_object as u32
            {
                continue;
            }
        }
        let a = inverse_transform_point(&bounds.transform, p1);
        let b = inverse_transform_point(&bounds.transform, p2);
        // Native two-refinement vrefp, not host division: intersection with
        // the box-local X=0 plane, without clamping to the endpoint interval.
        let weight = num.reciprocal(a[0] - b[0])? * a[0];
        let point = transform_point(&bounds.transform, madd(sub(b, a), weight, a));
        let current_effective = {
            let p = processed.lock().map_err(error)?;
            need(
                p.effective_transform_field(),
                "Processed effective transform",
            )?
            .columns()[3]
        };
        ground.skitch_spline_height = point[1] - current_effective.xyz[1];
        let entry_offset = read_floats::<1>(attributes, 0xbf43_3c58_d0f1_2390).map_err(error)?[0];
        let velocity = lanes(
            need(
                processed
                    .lock()
                    .map_err(error)?
                    .prior_output()
                    .centre_of_mass_velocity(),
                "Processed COM velocity",
            )?
            .value,
        );
        let minimum = refs.try_skitch_min_approach_speed().map_err(error)?;
        let distance = num.dot(sub(point, effective[3]), effective[2])?;
        let relative_speed = num.dot(sub(velocity, spline.velocity), effective[2])?;
        let time = num.reciprocal(vmax(relative_speed, minimum))? * (distance - entry_offset);
        ground.time_to_skitch = if time >= 0.0 { time } else { 0.0 };
        if refs.try_draw_grabbable_object_test().map_err(error)? {
            overlay.require().map_err(error)?;
        }
        grab.request_spline_data(spline.descriptor_type, spline.descriptor_object)?;
        let entry_time = read_floats::<1>(attributes, 0x424a_1c69_e007_6dbb).map_err(error)?[0];
        if ground.time_to_skitch < entry_time {
            ground.grab_object = true;
            ground.grab_object_id = spline.descriptor_object;
            ground.grab_spline_type = spline.descriptor_type;
            ground.grab_spline_object = spline.descriptor_object as u32;
            // Source requests twice on the accepted candidate, then disables
            // hand collisions for2 via the same caller's attached Skeleton.
            grab.request_spline_data(spline.descriptor_type, spline.descriptor_object)?;
            return Ok(true);
        }
    }
    Ok(false)
}
