//! AptIntegration.Update826014c0's shared 3D HUD landing jolt.
//! This updates once per frontend update, independently of APT movie frames.
#[derive(Default, Debug)]
pub struct HudJolt {
    process: bool,
    down: bool,
    target: f32,
    offset: f32,
}
impl HudJolt {
    pub fn trigger(&mut self, distance: f32) {
        self.process = true;
        self.down = true;
        self.target = distance;
    }
    pub fn offset(&self) -> f32 { self.offset }
    pub fn update(&mut self, speed: f32, distance: f32, recovery: f32) {
        if !self.process { return; }
        if self.down {
            self.offset += speed;
            if self.offset >= distance {
                self.offset = distance;
                self.down = false;
                self.target = 0.;
            }
        } else {
            // Original fsubs followed by fmadds8260157c (one final rounding).
            self.offset = (self.target - self.offset).mul_add(recovery, self.offset);
            if self.offset <= 0. { self.offset = 0.; }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn landing_reaches_original_limit_then_recovers_and_retrigger_keeps_offset() {
        let mut jolt=HudJolt::default();
        jolt.trigger(50.);
        for _ in 0..6 {jolt.update(9.98,50.,0.3258);}
        assert_eq!(jolt.offset(),50.);
        jolt.update(9.98,50.,0.3258);
        assert_eq!(jolt.offset(),(-50.0_f32).mul_add(0.3258,50.));
        let before=jolt.offset();jolt.trigger(50.);jolt.update(9.98,50.,0.3258);
        assert_eq!(jolt.offset(),(before+9.98).min(50.));
    }
}
