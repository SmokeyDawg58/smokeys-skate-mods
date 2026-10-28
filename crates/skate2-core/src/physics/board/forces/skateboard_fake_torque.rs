//! Ground control's fixed-base power graph at 82d9a1d8..a42c.
//! Unregistered draft: numeric estimates remain an explicit host policy.
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(crate) trait FakeTorqueRead {
    type Error;
    fn enabled(&mut self) -> Result<bool, Self::Error>;
    fn exponent(&mut self) -> Result<i32, Self::Error>;
    fn multiplier(&mut self) -> Result<f32, Self::Error>;
}
pub(crate) trait FakeTorqueNumeric {
    type Error;
    fn log_two_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
    fn exp_two_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
}

/// Explicit desktop estimate policy; this is not a Xenon VLOGEFP/VEXPTEFP
/// accuracy claim. The surrounding native polynomial/refinements still run.
pub(crate) struct HostFakeTorqueNumeric;
impl FakeTorqueNumeric for HostFakeTorqueNumeric {
    type Error = std::convert::Infallible;
    fn log_two_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(value.log2())
    }
    fn exp_two_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(value.exp2())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value)
    }
}
#[derive(Debug)]
pub(crate) enum FakeTorqueError<R, N> {
    Read(R),
    Numeric(N),
}
impl<R: std::fmt::Display, N: std::fmt::Display> std::fmt::Display for FakeTorqueError<R, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => error.fmt(f),
            Self::Numeric(error) => error.fmt(f),
        }
    }
}

/// Only the actual constant-positive-base specialization is recovered here.
/// The source still evaluates all estimates for exponent zero before selecting
/// one; disabled skips both tuning reads and every numerical operation.
#[expect(
    clippy::cast_precision_loss,
    reason = "source extsw/fcfid/frsp converts the live signed integer to f32"
)]
pub(crate) fn try_fake_torque<R: FakeTorqueRead, N: FakeTorqueNumeric>(
    read: &mut R,
    numeric: &mut N,
) -> Result<Option<SkeletonRootVector>, FakeTorqueError<R::Error, N::Error>> {
    use FakeTorqueError::{Numeric, Read};
    if !read.enabled().map_err(Read)? {
        return Ok(None);
    }
    let logarithm_seed = numeric.log_two_estimate(10.0).map_err(Numeric)?; //a258
    let exponent = read.exponent().map_err(Read)? as f32; //a26c..a2b0
    let binary_exponent = logarithm_seed.floor(); //a2a8
    // Positive 10 has mantissa 1.25; source masks never route an unknown lane.
    let delta = f32::from_bits(0x3fa0_0000) - 1.0;
    let polynomial = logarithm_polynomial(delta);
    let scaled_delta = delta * exponent;
    let scaled_binary_exponent = binary_exponent * exponent;
    let logarithm = scaled_delta.mul_add(polynomial, scaled_binary_exponent);
    let integral = logarithm.floor();
    let fraction = logarithm - integral;
    let scale = numeric.exp_two_estimate(integral).map_err(Numeric)?; //a318
    let denominator = exponential_denominator(fraction);
    let multiplier = read.multiplier().map_err(Read)?;
    let reciprocal = refined_reciprocal(denominator, numeric).map_err(Numeric)?;
    let power = scale * reciprocal;
    // Positive nonzero base removes the negative/zero-base masks, but the
    // exponent-zero mask remains and follows all source numerical operations.
    let power = if exponent == 0.0 { 1.0 } else { power };
    let magnitude = power * multiplier;
    Ok(Some(SkeletonRootVector::known([0.0, magnitude, 0.0, 0.0])))
}

/// Shared XMVectorLog/XMVectorExp polynomials. Constants at820f0fa0/fb0
/// and821ee8b0/c0; also inlined by LWPedestrianMan.UpdateMoodables82e83e40.
fn polynomial(delta: f32, first: [u32;4], second: [u32;4]) -> f32 {
    let a=first.map(f32::from_bits);
    let b=second.map(f32::from_bits);
    let square=delta*delta;
    let p=delta.mul_add(a[1],a[0]);
    let q=delta.mul_add(b[1],b[0]);
    let cube=delta*square;
    let p=square.mul_add(a[2],p);
    let q=square.mul_add(b[2],q);
    let fourth=square*square;
    let p=cube.mul_add(a[3],p);
    let q=cube.mul_add(b[3],q);
    fourth.mul_add(q,p)
}
fn logarithm_polynomial(delta:f32)->f32 {
    polynomial(delta,[0x3fb8aa0e,0xbf389e52,0x3ef5162d,0xbeb1d204],
        [0x3e77adbd,0xbe0cd4fb,0x3d5541c6,0xbc188b0b])
}
fn exponential_denominator(delta:f32)->f32 {
    polynomial(delta,[0x3f800000,0xbf317218,0x3e75fded,0xbd6357ca],
        [0x3c1d8c54,0xbaae1854,0x391aa7d7,0xb7364261])
}
pub(crate) fn refined_reciprocal<N:FakeTorqueNumeric>(value:f32,numeric:&mut N)->Result<f32,N::Error> {
    let seed=numeric.reciprocal_estimate(value)?;
    let (first,second)=refine_reciprocal_seed(value,seed);
    Ok(if first.is_nan(){seed}else{second})
}
fn refine_reciprocal_seed(value:f32,seed:f32)->(f32,f32) {
    let residual=-(value.mul_add(seed,-1.));
    let first=seed.mul_add(residual,seed);
    let residual=-(value.mul_add(first,-1.));
    let second=first.mul_add(residual,first);
    (first,second)
}
/// The Vector division used for the sound calibration has no NaN seed select.
pub(crate) fn unconditional_refined_reciprocal<N:FakeTorqueNumeric>(value:f32,numeric:&mut N)->Result<f32,N::Error> {
    let seed=numeric.reciprocal_estimate(value)?;
    Ok(refine_reciprocal_seed(value,seed).1)
}
/// Fixed positive base10 specialization shared with UpdateMoodables. Estimate
/// instructions retain the explicit desktop policy above, not Xenon bit parity.
pub(crate) fn power_ten<N:FakeTorqueNumeric>(exponent:f32,numeric:&mut N)->Result<f32,N::Error> {
    let binary=numeric.log_two_estimate(10.)?.floor();
    let delta=f32::from_bits(0x3fa00000)-1.;
    let scaled_delta=delta*exponent;
    let scaled_binary=binary*exponent;
    let logarithm=scaled_delta.mul_add(logarithm_polynomial(delta),scaled_binary);
    let integral=logarithm.floor();
    let fraction=logarithm-integral;
    let scale=numeric.exp_two_estimate(integral)?;
    let value=scale*refined_reciprocal(exponential_denominator(fraction),numeric)?;
    Ok(if exponent==0.{1.}else{value})
}
/// Native vsel extracts the mantissa and VLOGEFP/floor supplies the exponent;
/// no clamp of zero/negative/NaN inputs is introduced.
pub(crate) fn natural_log<N:FakeTorqueNumeric>(value:f32,numeric:&mut N)->Result<f32,N::Error> {
    let delta=f32::from_bits((value.to_bits()&0x007fffff)|0x3f800000)-1.;
    let binary=numeric.log_two_estimate(value)?.floor();
    Ok(delta.mul_add(logarithm_polynomial(delta),binary)*f32::from_bits(0x3f317218))
}

#[cfg(test)]
mod tests {
    use super::{FakeTorqueNumeric, FakeTorqueRead, try_fake_torque};
    use std::{cell::RefCell, convert::Infallible, rc::Rc};
    struct Read {
        calls: Rc<RefCell<Vec<&'static str>>>,
        enabled: bool,
        exponent: i32,
    }
    impl FakeTorqueRead for Read {
        type Error = Infallible;
        fn enabled(&mut self) -> Result<bool, Self::Error> {
            self.calls.borrow_mut().push("enabled");
            Ok(self.enabled)
        }
        fn exponent(&mut self) -> Result<i32, Self::Error> {
            self.calls.borrow_mut().push("exponent");
            Ok(self.exponent)
        }
        fn multiplier(&mut self) -> Result<f32, Self::Error> {
            self.calls.borrow_mut().push("multiplier");
            Ok(f32::from_bits(0xc0c1_999a))
        }
    }
    struct Numeric(Rc<RefCell<Vec<&'static str>>>);
    impl FakeTorqueNumeric for Numeric {
        type Error = Infallible;
        fn log_two_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
            self.0.borrow_mut().push("log");
            assert_eq!(v.to_bits(), 10.0f32.to_bits());
            Ok(v.log2())
        }
        fn exp_two_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
            self.0.borrow_mut().push("exp");
            Ok(v.exp2())
        }
        fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
            self.0.borrow_mut().push("reciprocal");
            Ok(1.0 / v)
        }
    }
    #[test]
    fn disabled_is_a_live_branch_and_zero_exponent_still_runs_native_graph() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut read = Read {
            calls: Rc::clone(&calls),
            enabled: false,
            exponent: 0,
        };
        let mut numeric = Numeric(Rc::clone(&calls));
        assert!(try_fake_torque(&mut read, &mut numeric).unwrap().is_none());
        assert_eq!(*calls.borrow(), ["enabled"]);
        calls.borrow_mut().clear();
        read.enabled = true;
        let result = try_fake_torque(&mut read, &mut numeric).unwrap().unwrap();
        assert_eq!(result.xyz.map(f32::to_bits), [0, 0xc0c1_999a, 0]);
        assert_eq!(result.w.map(f32::to_bits), Some(0));
        assert_eq!(
            *calls.borrow(),
            [
                "enabled",
                "log",
                "exponent",
                "exp",
                "multiplier",
                "reciprocal"
            ]
        );
    }
    #[test]
    fn source_default_has_nonzero_negative_torque() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut read = Read {
            calls: Rc::clone(&calls),
            enabled: true,
            exponent: -1,
        };
        let result = try_fake_torque(&mut read, &mut Numeric(calls))
            .unwrap()
            .unwrap();
        assert!((result.xyz[1] - -0.605).abs() < 0.000_001);
    }
}
