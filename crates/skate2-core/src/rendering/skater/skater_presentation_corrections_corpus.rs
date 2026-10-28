//! Original retained replay correction and reached twist-operand validation.
use super::*;
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric};
pub(crate) struct Numeric {
    pub(crate) lengths: std::rc::Rc<std::cell::RefCell<Vec<(usize, usize, u32)>>>,
}
impl ReckoningNumeric for Numeric {
    type Error = ReckoningHostNumericError;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        HostReckoningNumeric.dot3(a, b)
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric.reciprocal_estimate(v)
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric.reciprocal_sqrt_estimate(v)
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(v)
    }
}
impl PresentationMatrixNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        PresentationMatrixNumeric::dot4(&mut HostReckoningNumeric, a, b)
    }
    fn observe_twist_length(&mut self, s: usize, d: usize, v: f32) {
        self.lengths.borrow_mut().push((s, d, v.to_bits()));
    }
}
impl crate::game_input_command::GameInputCommandNumeric for Numeric {
    fn truncate_i64(&mut self, value: f32) -> Result<i64, Self::Error> {
        crate::game_input_command::GameInputCommandNumeric::truncate_i64(
            &mut HostReckoningNumeric,
            value,
        )
    }
}
impl super::super::read::decode::PresentationDecodeNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        super::super::read::decode::PresentationDecodeNumeric::dot4(&mut HostReckoningNumeric, a, b)
    }
}
pub(crate) fn check_corrected(presentation: &super::super::startup::SelectedPresentationStartup) {
    let transforms = presentation.animation.hips_transforms.lock().unwrap();
    assert!(transforms[0].unwrap().iter().all(|v| v.is_finite()));
    let replay = presentation.animation.replay[0].read().unwrap().1;
    assert!(
        replay.iter().all(|sqt| sqt
            .rotation
            .iter()
            .chain(&sqt.translation)
            .chain(&sqt.scale)
            .all(|v| v.is_finite())),
        "native MoveRoot/Expressions produced nonfinite channels; inspect actual twist operands, do not synthesize a fallback"
    );
}
