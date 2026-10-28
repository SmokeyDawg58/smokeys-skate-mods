//! `GrindCollision` intersectors against the nearby primitives:
//! `CalcTruckCollisions` 0x82cfbd38, `CalcTipCollisions` 0x82cfa600 and
//! `CalcBoardslideCollision` 0x82cfb990. Each primitive segment is tested
//! against two triangles per contact region; the nearest hit to the region
//! centre wins with a strict "best > distance squared" update from 1e6.
use crate::physics_state_ids::{PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_TIPSLIDE};
use super::{
    math::{GrindNum, V4, add, madd, scale, sub},
    types::{CollTestPair, CollTestResult},
    view::{
        BOARDSLIDE_DEPTH, BOARDSLIDE_LIFT, DECK_CENTER_TO_TRUCK, GrindAttributes, GrindError,
        GrindView, TRUCK_TO_WHEEL,
    },
};
use crate::grind_data_runtime::GrindDataPrimitive;

/// `__real_49742400`, the initial best distance squared.
const FAR: f32 = f32::from_bits(0x4974_2400);
/// `VECFLOAT_HALF`.
const HALF: f32 = 0.5;

fn region(
    point: Option<V4>,
    centre: V4,
    index: usize,
    best: &mut f32,
    result: &mut CollTestResult,
    num: &mut GrindNum<'_>,
) -> Result<(), GrindError> {
    let Some(point) = point else {
        return Ok(());
    };
    let offset = sub(point, centre);
    let squared = num.dot(offset, offset)?;
    if *best > squared {
        result.point = point;
        *best = squared;
        result.index = i32::try_from(index).map_err(super::view::error)?;
        result.hit = true;
    }
    Ok(())
}

/// Both triangles are always tested; the first hit's point is kept.
fn quad(
    prim: &GrindDataPrimitive,
    first: [V4; 3],
    second: [V4; 3],
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    let a = num.triangle(prim.start, prim.end, first)?;
    let b = num.triangle(prim.start, prim.end, second)?;
    Ok(a.or(b))
}

fn fresh_pair() -> CollTestPair {
    let mut pair = CollTestPair::empty();
    pair.performed = true;
    pair
}

/// `CalcTruckCollisions` 0x82cfbd38: axle-width quads below each truck.
pub(crate) fn truck_collisions(
    view: &GrindView,
    attributes: &GrindAttributes,
    prims: &[GrindDataPrimitive],
    num: &mut GrindNum<'_>,
) -> Result<CollTestPair, GrindError> {
    let mut pair = fresh_pair();
    let to_truck = attributes.scalar(DECK_CENTER_TO_TRUCK)?;
    let to_wheel = attributes.scalar(TRUCK_TO_WHEEL)?;
    let (x, y, z, t) = (view.deck_x(), view.deck_y(), view.deck_z(), view.deck_t());
    let depth = scale(y, f32::from_bits(0xbe4c_cccd));
    let base = madd(y, f32::from_bits(0xbca3_d70a), t);
    let along = scale(z, to_truck);
    let across = scale(x, to_wheel);
    let front = add(base, along);
    let rear = sub(base, along);
    let front_right = add(front, across);
    let front_left = sub(front, across);
    let rear_right = add(rear, across);
    let rear_left = sub(rear, across);
    let front_right_low = add(front_right, depth);
    let front_sum = add(front_right, front_left);
    let front_left_low = add(front_left, depth);
    let rear_right_low = add(rear_right, depth);
    let rear_sum = add(rear_right, rear_left);
    let rear_left_low = add(rear_left, depth);
    let front_centre = scale(front_sum, HALF);
    let rear_centre = scale(rear_sum, HALF);
    let (mut best_front, mut best_rear) = (FAR, FAR);
    for (index, prim) in prims.iter().enumerate() {
        let hit = quad(
            prim,
            [front_right, front_left, front_left_low],
            [front_right, front_right_low, front_left_low],
            num,
        )?;
        region(
            hit,
            front_centre,
            index,
            &mut best_front,
            &mut pair.front,
            num,
        )?;
        let hit = quad(
            prim,
            [rear_right, rear_left, rear_left_low],
            [rear_right, rear_right_low, rear_left_low],
            num,
        )?;
        region(hit, rear_centre, index, &mut best_rear, &mut pair.rear, num)?;
    }
    Ok(pair)
}

/// `CalcTipCollisions` 0x82cfa600: nose and tail quads beyond the trucks.
/// In a tip grind (402/404) only the end the grind is on is tested.
pub(crate) fn tip_collisions(
    view: &GrindView,
    attributes: &GrindAttributes,
    prims: &[GrindDataPrimitive],
    num: &mut GrindNum<'_>,
) -> Result<CollTestPair, GrindError> {
    let mut pair = fresh_pair();
    let to_truck = attributes.scalar(DECK_CENTER_TO_TRUCK)?;
    let to_tip = to_truck + f32::from_bits(0x3e5c_28f6);
    let (y, z, t) = (view.deck_y(), view.deck_z(), view.deck_t());
    let tip_along = scale(z, to_tip);
    let base = madd(y, f32::from_bits(0x3ca3_d70a), t);
    let along = scale(z, to_truck);
    let lift = scale(y, f32::from_bits(0x3cf5_c28f));
    let nose = add(base, tip_along);
    let front = add(base, along);
    let rear = sub(base, along);
    let tail = sub(base, tip_along);
    let depth = scale(y, f32::from_bits(0xbe4c_cccd));
    let nose_top = add(nose, lift);
    let tail_top = add(tail, lift);
    let front_low = add(front, depth);
    let rear_low = add(rear, depth);
    let front_sum = add(nose_top, front);
    let rear_sum = add(tail_top, rear);
    let nose_low = add(nose_top, depth);
    let tail_low = add(tail_top, depth);
    let front_centre = scale(front_sum, HALF);
    let rear_centre = scale(rear_sum, HALF);
    let (mut test_front, mut test_rear) = (true, true);
    if matches!(view.state, PHYSICS_STATE_GRIND_TIPSLIDE | PHYSICS_STATE_GRIND_BACKSLASH) {
        if view.bit_8c4(0x200)? {
            test_rear = false;
        } else {
            test_front = false;
        }
    }
    let (mut best_front, mut best_rear) = (FAR, FAR);
    for (index, prim) in prims.iter().enumerate() {
        if test_front {
            let hit = quad(
                prim,
                [front, nose_top, nose_low],
                [front_low, front, nose_low],
                num,
            )?;
            region(
                hit,
                front_centre,
                index,
                &mut best_front,
                &mut pair.front,
                num,
            )?;
        }
        if test_rear {
            let hit = quad(
                prim,
                [rear, tail_top, tail_low],
                [rear_low, rear, tail_low],
                num,
            )?;
            region(hit, rear_centre, index, &mut best_rear, &mut pair.rear, num)?;
        }
    }
    Ok(pair)
}

/// `CalcBoardslideCollision` 0x82cfb990: one quad along the deck's middle,
/// the nearest hit measured to the deck position.
pub(crate) fn boardslide_collision(
    view: &GrindView,
    attributes: &GrindAttributes,
    prims: &[GrindDataPrimitive],
    num: &mut GrindNum<'_>,
) -> Result<CollTestResult, GrindError> {
    let mut result = CollTestResult::empty();
    result.performed = true;
    let to_truck = attributes.scalar(DECK_CENTER_TO_TRUCK)?;
    let depth = attributes.scalar(BOARDSLIDE_DEPTH)?;
    let lift = attributes.scalar(BOARDSLIDE_LIFT)?;
    let drop = super::math::neg1(lift + depth);
    let (y, z, t) = (view.deck_y(), view.deck_z(), view.deck_t());
    let base = madd(y, lift, t);
    let along = scale(z, to_truck);
    let down = scale(y, drop);
    let front_top = add(base, along);
    let rear_top = sub(base, along);
    let front_bottom = add(front_top, down);
    let rear_bottom = add(rear_top, down);
    let mut best = FAR;
    for (index, prim) in prims.iter().enumerate() {
        let hit = quad(
            prim,
            [front_top, rear_top, front_bottom],
            [rear_bottom, rear_top, front_bottom],
            num,
        )?;
        region(hit, t, index, &mut best, &mut result, num)?;
    }
    Ok(result)
}
