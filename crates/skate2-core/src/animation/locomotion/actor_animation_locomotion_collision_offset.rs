//! SkaterAnim::UpdateCollisionOffset82c8b3f0. Caller passes retained owner fields.
use crate::{
    reckoning_numeric::{ReckoningClampError, ReckoningNumeric, try_clamp, try_normalize_safe},
    skeleton_root_transforms::SkeletonRootVector as V,
};

pub(crate) trait CollisionOffsetTuning {
    type Error;
    fn displacement_blend(&mut self) -> Result<f32, Self::Error>;
    fn minimum_displacement(&mut self) -> Result<f32, Self::Error>;
    fn avoidance_duration(&mut self) -> Result<f32, Self::Error>;
    fn maximum_displacement(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum CollisionOffsetError<N, T> {
    Numeric(N),
    Clamp(ReckoningClampError<N>),
    Tuning(T),
}

/// COM velocity is an unused native argument; no host read of it is required.
#[allow(clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments)]
pub(crate) fn update_collision_offset<N: ReckoningNumeric, T: CollisionOffsetTuning>(
    displacement: &mut V,
    trajectory_displacement: &mut V,
    last_normal: &mut V,
    timer: &mut f32,
    incoming: V,
    proxy_collision: bool,
    numeric: &mut N,
    tuning: &mut T,
) -> Result<bool, CollisionOffsetError<N::Error, T::Error>> {
    use CollisionOffsetError as E;
    let zero = V::known([0.0; 4]);
    // b420 BGT skips substitution; unordered follows the substitution branch.
    let incoming = if !(*timer > 0.0) && !proxy_collision {
        zero
    } else {
        incoming
    };
    // b484/b498 sample only once, despite duplicate reads in decompiler output.
    let blend = tuning.displacement_blend().map_err(E::Tuning)?;
    let old = displacement.map(|v| v * (1.0 - blend));
    *displacement = incoming.zip(old, |v, old| v.mul_add(blend, old));
    displacement.xyz[1] = 0.0;
    let square = numeric
        .dot_vector(*displacement, *displacement)
        .map_err(E::Numeric)?;
    let minimum = tuning.minimum_displacement().map_err(E::Tuning)?;
    if square > minimum * minimum {
        *last_normal = try_normalize_safe(*displacement, zero, numeric).map_err(E::Numeric)?;
        *timer = tuning.avoidance_duration().map_err(E::Tuning)?;
    } else {
        *trajectory_displacement = zero;
    }
    // b588 BO4 BI25 returns when GT is clear, including unordered.
    if !(*timer > 0.0) {
        return Ok(false);
    }
    *timer -= f32::from_bits(0x3c88_8889);
    let mut horizontal = *displacement;
    horizontal.xyz[1] = 0.0;
    let maximum = tuning.maximum_displacement().map_err(E::Tuning)?;
    *trajectory_displacement = try_clamp(horizontal, maximum, numeric).map_err(E::Clamp)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    struct Tuning {
        calls: Vec<&'static str>,
        fail_duration: bool,
    }
    impl CollisionOffsetTuning for Tuning {
        type Error = ();
        fn displacement_blend(&mut self) -> Result<f32, ()> {
            self.calls.push("blend");
            Ok(0.5)
        }
        fn minimum_displacement(&mut self) -> Result<f32, ()> {
            self.calls.push("minimum");
            Ok(0.1)
        }
        fn avoidance_duration(&mut self) -> Result<f32, ()> {
            self.calls.push("duration");
            if self.fail_duration { Err(()) } else { Ok(1.0) }
        }
        fn maximum_displacement(&mut self) -> Result<f32, ()> {
            self.calls.push("maximum");
            Ok(0.4)
        }
    }
    #[test]
    fn duration_failure_retains_displacement_and_normal_prefix_without_timer_or_clamp() {
        let mut d = V {
            xyz: [2.0, 4.0, 0.0],
            w: None,
        };
        let mut trajectory = V::known([7.0; 4]);
        let mut normal = V::known([8.0; 4]);
        let mut timer = 0.0;
        let mut tuning = Tuning {
            calls: Vec::new(),
            fail_duration: true,
        };
        assert!(matches!(
            update_collision_offset(
                &mut d,
                &mut trajectory,
                &mut normal,
                &mut timer,
                V::known([20.0; 4]),
                false,
                &mut HostReckoningNumeric,
                &mut tuning
            ),
            Err(CollisionOffsetError::Tuning(()))
        ));
        assert_eq!(tuning.calls, ["blend", "minimum", "duration"]);
        assert_eq!(d.xyz, [1.0, 0.0, 0.0]);
        assert_eq!(d.w, None);
        assert_eq!(normal.xyz, [1.0, 0.0, 0.0]);
        assert_eq!(normal.w, None);
        assert_eq!(timer, 0.0);
        assert_eq!(trajectory, V::known([7.0; 4]));
    }
    #[test]
    fn nan_timer_uses_zero_input_and_returns_before_clamp() {
        let mut d = V::known([0.0; 4]);
        let mut trajectory = V::known([7.0; 4]);
        let mut normal = V::known([8.0; 4]);
        let mut timer = f32::NAN;
        let mut tuning = Tuning {
            calls: Vec::new(),
            fail_duration: false,
        };
        assert!(
            !update_collision_offset(
                &mut d,
                &mut trajectory,
                &mut normal,
                &mut timer,
                V::known([20.0; 4]),
                false,
                &mut HostReckoningNumeric,
                &mut tuning
            )
            .unwrap()
        );
        assert_eq!(tuning.calls, ["blend", "minimum"]);
        assert_eq!(d, V::known([0.0; 4]));
        assert_eq!(trajectory, V::known([0.0; 4]));
        assert_eq!(normal, V::known([8.0; 4]));
        assert!(timer.is_nan());
    }
}
