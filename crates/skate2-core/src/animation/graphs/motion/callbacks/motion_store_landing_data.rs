//! StoreLandingData.Update82ca1d00, preserving early history and late stores.
use crate::skeleton_root_transforms::SkeletonRootVector;

/// Create82ca1f78: size0x10, alignment0x10, float+8 and byte+c cleared.
#[derive(Debug, Default)]
pub(crate) struct LandingInstance {
    pub(crate) previous_vertical_velocity: f32,
    pub(crate) was_air: bool,
}

pub(crate) trait LandingSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn is_air(&mut self) -> Result<bool, Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn set_landing_velocity(&mut self, value: f32) -> Result<(), Self::Error>;
    fn centre_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn up(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, Self::Error>;
}

pub(crate) fn update<S: LandingSource>(
    source: &mut S,
    instance: Option<&mut LandingInstance>,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let air = source.is_air()?;
    let instance = instance.ok_or_else(|| source.missing_instance())?;
    if !instance.was_air && air {
        source.set_landing_velocity(0.0)?;
    }
    instance.was_air = air; //82ca1ed4, before either SystemReckoning load.
    let velocity = source.centre_velocity()?;
    let up = source.up()?; //separate GetPhysOut call at82ca1f0c.
    let vertical = source.dot(velocity, up)?;
    // Raw40980028/4098001c branch on !LT, including unordered.
    if vertical < 0.0 && vertical < instance.previous_vertical_velocity {
        source.set_landing_velocity(vertical.abs())?;
    }
    instance.previous_vertical_velocity = vertical; //82ca1f58, even NaN.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        air: bool,
        vertical: f32,
        fail_up: bool,
        writes: Vec<u32>,
    }
    impl LandingSource for Source {
        type Error = ();
        fn resolve_interfaces(&mut self) -> Result<(), ()> {
            Ok(())
        }
        fn is_air(&mut self) -> Result<bool, ()> {
            Ok(self.air)
        }
        fn missing_instance(&mut self) {}
        fn set_landing_velocity(&mut self, v: f32) -> Result<(), ()> {
            self.writes.push(v.to_bits());
            Ok(())
        }
        fn centre_velocity(&mut self) -> Result<SkeletonRootVector, ()> {
            Ok(SkeletonRootVector {
                xyz: [0.0, self.vertical, 0.0],
                w: None,
            })
        }
        fn up(&mut self) -> Result<SkeletonRootVector, ()> {
            if self.fail_up {
                Err(())
            } else {
                Ok(SkeletonRootVector {
                    xyz: [0.0, 1.0, 0.0],
                    w: None,
                })
            }
        }
        fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, ()> {
            Ok(crate::reckoning_numeric::host_dot3(
                [a.xyz[0], a.xyz[1], a.xyz[2], f32::NAN],
                [b.xyz[0], b.xyz[1], b.xyz[2], f32::NAN],
            ))
        }
    }
    #[test]
    fn successive_frames_use_previous_sample_and_do_not_require_air_to_publish() {
        let mut s = Source {
            air: true,
            vertical: -3.0,
            fail_up: false,
            writes: vec![],
        };
        let mut i = LandingInstance::default();
        update(&mut s, Some(&mut i)).unwrap();
        s.vertical = -2.0;
        update(&mut s, Some(&mut i)).unwrap();
        s.air = false;
        s.vertical = -2.5;
        update(&mut s, Some(&mut i)).unwrap();
        assert_eq!(s.writes, [0, 3.0_f32.to_bits(), 2.5_f32.to_bits()]);
        s.vertical = f32::NAN;
        update(&mut s, Some(&mut i)).unwrap();
        assert!(i.previous_vertical_velocity.is_nan());
        s.vertical = -9.0;
        update(&mut s, Some(&mut i)).unwrap();
        assert_eq!(s.writes.len(), 3); //finite < previous NaN is false.
    }
    #[test]
    fn missing_late_up_preserves_air_reset_and_history_but_not_previous_sample() {
        let mut s = Source {
            air: true,
            vertical: -3.0,
            fail_up: true,
            writes: vec![],
        };
        let mut i = LandingInstance {
            previous_vertical_velocity: -1.0,
            was_air: false,
        };
        assert!(update(&mut s, Some(&mut i)).is_err());
        assert!(i.was_air);
        assert_eq!(i.previous_vertical_velocity.to_bits(), (-1.0_f32).to_bits());
        assert_eq!(s.writes, [0]);
    }
}
