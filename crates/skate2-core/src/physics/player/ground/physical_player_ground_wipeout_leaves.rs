//! Source helper leaves used by `CheckForGroundWipeout`.
use super::{
    CollisionFlag, GroundWipeoutSource, Matrix, Scalar, Vector, attributes::Attribute,
    read_attribute,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(crate) fn squashed<S: GroundWipeoutSource>(
    s: &mut S,
    threshold: f32,
) -> Result<bool, S::Error> {
    let target = s.capture_draw_target()?;
    if s.read_draw_target(&target)? {
        s.draw_squashed()?;
    }
    Ok(s.scalar(Scalar::MaxDisplacement)? > threshold)
}
pub(crate) fn displacement<S: GroundWipeoutSource>(
    s: &mut S,
    threshold: f32,
) -> Result<bool, S::Error> {
    let target = s.capture_draw_target()?;
    if s.read_draw_target(&target)? {
        s.draw_displacement()?;
    }
    if s.wipeout_on_collision()? && s.collision_flag(CollisionFlag::CompliantJustCollided)? {
        return Ok(true);
    }
    let squared = threshold * threshold;
    let hook = s.vector(Vector::HookToHips)?;
    Ok(s.dot(hook, hook)? > squared)
}
fn select_max(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
pub(crate) fn contact_force<S: GroundWipeoutSource>(
    s: &mut S,
    body_threshold: f32,
    arm_threshold: f32,
) -> Result<bool, S::Error> {
    let target = s.capture_draw_target()?;
    let force0 = s.contact_force(0)?;
    let force1 = s.contact_force(1)?;
    let force5 = s.contact_force(5)?;
    let force4 = s.contact_force(4)?;
    let draw = s.read_draw_target(&target)?; //fdd4, before remaining four force loads
    let force7 = s.contact_force(7)?;
    let force6 = s.contact_force(6)?;
    let force2 = s.contact_force(2)?;
    let force3 = s.contact_force(3)?;
    let body = select_max(
        select_max(select_max(force0, force1), select_max(force5, force4)),
        select_max(force7, force6),
    );
    let arms = select_max(force2, force3);
    if draw {
        s.draw_contact_force(body, arms)?;
    }
    Ok(body > body_threshold || arms > arm_threshold) //fee8 BO4 BI25 skips GT-clear.
}
pub(crate) fn vehicle<S: GroundWipeoutSource>(s: &mut S) -> Result<(), S::Error> {
    let skitch = s.processed_flag(0x8cc, 0x10)?;
    let contact = s.scalar(Scalar::VehicleContact)?;
    let threshold = read_attribute(
        s,
        if skitch {
            Attribute::VehicleSkitch
        } else {
            Attribute::Vehicle
        },
    )?;
    //941c raw40990024 BO4 BI25 skips when GT clear, including unordered.
    if contact > threshold {
        s.request(7)?;
    }
    Ok(())
}
pub(crate) fn deck_flipped<S: GroundWipeoutSource>(
    s: &mut S,
    threshold: f32,
) -> Result<(), S::Error> {
    let board = s.matrix(Matrix::BoardTransform)?;
    let processed = s.matrix(Matrix::ProcessedSkateTransform)?;
    let x = s.dot(board.columns()[0], processed.columns()[0])?;
    let y = s.dot(board.columns()[1], processed.columns()[1])?;
    let z = s.dot(board.columns()[2], processed.columns()[2])?;
    let xy = s.minimum(x, y)?;
    let xyz = s.minimum(xy, z)?;
    let lower = s.maximum(-1.0, xyz)?;
    let bounded = s.minimum(1.0, lower)?;
    if s.acos(bounded)? > threshold {
        s.request(3)?;
    }
    Ok(())
}
pub(crate) fn deck_acceleration<S: GroundWipeoutSource>(
    s: &mut S,
    xz_threshold: f32,
    y_threshold: f32,
) -> Result<(), S::Error> {
    let num_parts = s.num_parts_in_contact()?;
    if num_parts == 0 {
        return Ok(());
    }
    let transform = s.matrix(Matrix::WorldToAnimation)?;
    // Native Body+290 is CollisionInfo.maxContactVel, NOT acceleration[6].
    let velocity = s.vector(Vector::MaxContactVelocity)?;
    let c = transform.columns();
    let xyz: [f32; 3] = std::array::from_fn(|i| {
        let first = c[0].xyz[i] * velocity.xyz[0];
        let second = c[1].xyz[i].mul_add(velocity.xyz[1], first);
        c[2].xyz[i].mul_add(velocity.xyz[2], second)
    });
    let absolute_y = f32::from_bits(xyz[1].to_bits() & 0x7fff_ffff);
    let planar = SkeletonRootVector {
        xyz: [xyz[0], 0.0, xyz[2]],
        w: None,
    };
    let length = s.length(planar)?;
    #[cfg(test)]
    if std::env::var_os("SKATE2_TRACE_DECK_ACCELERATION").is_some()
        && (length > xz_threshold || absolute_y > y_threshold)
    {
        eprintln!(
            "deck acceleration request operands: parts={num_parts} world_velocity_bits={:?} world_to_animation_bits={:?} local_velocity_bits={:?} planar_bits={:#010x} vertical_bits={:#010x} xz_threshold_bits={:#010x} y_threshold_bits={:#010x}",
            velocity.xyz.map(f32::to_bits),
            c.map(|column| column.xyz.map(f32::to_bits)),
            xyz.map(f32::to_bits),
            length.to_bits(),
            absolute_y.to_bits(),
            xz_threshold.to_bits(),
            y_threshold.to_bits(),
        );
    }
    //92e0 ordered BGT then92ec BO4 BI25: unordered vertical skips.
    if length > xz_threshold || absolute_y > y_threshold {
        s.request(2)?;
    }
    Ok(())
}
pub(crate) fn off_balance<S: GroundWipeoutSource>(s: &mut S) -> Result<(), S::Error> {
    let speed = read_attribute(s, Attribute::BalanceSpeed)?;
    let decay = read_attribute(s, Attribute::BalanceDecay)?;
    let threshold = read_attribute(s, Attribute::BalanceLimit)?;
    if speed.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        s.require_positive_balance_speed(speed)?;
    }
    let gate_a = if s.processed_flag(0x8cc, 0x0200_0000)? {
        0.0
    } else {
        1.0
    };
    let gate_b = if s.processed_flag(0x8c4, 0x20)? {
        0.0
    } else {
        1.0
    };
    let deck_speed = s.scalar(Scalar::DeckSpeed)?;
    let numerator = speed - deck_speed;
    let up_y = s.scalar(Scalar::UpY)?;
    let tilt = 1.0 - up_y;
    let fraction = numerator / speed;
    let scaled = (fraction * tilt) * gate_b;
    let contribution = scaled.mul_add(gate_a, -decay); //FMSUBS f1f8
    let total = if contribution > 0.0 {
        s.off_balance_total()? + contribution
    } else {
        0.0
    };
    s.set_off_balance_total(total)?;
    if total > threshold {
        s.request(11)?;
    }
    Ok(())
}
pub(crate) fn leaning<S: GroundWipeoutSource>(s: &mut S) -> Result<(), S::Error> {
    let up = s.scalar(Scalar::UpY)?;
    let threshold = read_attribute(s, Attribute::LeaningUp)?;
    if up < threshold && s.collision_flag(CollisionFlag::CompliantJustCollided)? {
        let normal = s.vector(Vector::MostVerticalContactNormal)?;
        if normal.xyz[1] > f32::from_bits(0x3f34_fdf4) {
            s.request(19)?;
        }
    }
    Ok(())
}
