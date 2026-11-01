//! PC debug-camera controls, kept separate from the game's normal input maps.

const DPAD_UP: u16 = 0x01;
const DPAD_DOWN: u16 = 0x02;
const DPAD_LEFT: u16 = 0x04;
const DPAD_RIGHT: u16 = 0x08;
const FOV_SPEED: f32 = 45.0;
const MIN_FOV: f32 = 25.0;
const MAX_FOV: f32 = 110.0;
const VERTICAL_SPEED: f32 = 6.0;

pub(super) struct Controls {
    fov: f32,
    vertical_boost: f32,
}

impl Default for Controls {
    fn default() -> Self { Self { fov: 55.0, vertical_boost: 1.0 } }
}

pub(super) struct Frame {
    pub fast: f32,
    pub fast_multiplier: f32,
    pub vertical_speed: f32,
    pub up: f32,
    pub down: f32,
}

impl Controls {
    pub fn fov(&self) -> f32 { self.fov }
    pub fn reset_boost(&mut self) { self.vertical_boost = 1.0; }

    /// RT is a proportional accelerator, reaching 20x when fully pressed.
    /// Its vertical transition follows ManualCam's own 60 Hz smoothing.
    pub fn advance(&mut self, buttons: u16, right_trigger: u8, dt: f32) -> Frame {
        let step = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let direction = f32::from(buttons & DPAD_LEFT != 0) - f32::from(buttons & DPAD_RIGHT != 0);
        self.fov = (self.fov + direction * FOV_SPEED * step).clamp(MIN_FOV, MAX_FOV);
        let pressure = f32::from(right_trigger) / 255.0;
        let pressure = if pressure > 0.05 { pressure } else { 0.0 };
        let target = 1.0 + 19.0 * pressure;
        self.vertical_boost += (target - self.vertical_boost) * (6.0 * step).clamp(0.0, 1.0);
        Frame {
            fast: f32::from(pressure > 0.0),
            fast_multiplier: target,
            vertical_speed: VERTICAL_SPEED * self.vertical_boost,
            up: f32::from(buttons & DPAD_UP != 0),
            down: f32::from(buttons & DPAD_DOWN != 0),
        }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn fov_moves_in_both_directions_and_stays_bounded() {
        let mut c=Controls::default();
        assert_eq!(c.fov(),55.0);
        c.advance(DPAD_LEFT,0,1.0); assert_eq!(c.fov(),100.0);
        c.advance(DPAD_LEFT,0,1.0); assert_eq!(c.fov(),110.0);
        c.advance(DPAD_RIGHT,0,2.0); assert_eq!(c.fov(),25.0);
    }
    #[test] fn trigger_accelerates_horizontally_and_vertically_with_smoothing() {
        let mut c=Controls::default();
        let normal=c.advance(DPAD_UP,0,1.0/60.0);
        assert_eq!((normal.up,normal.down,normal.fast,normal.vertical_speed),(1.0,0.0,0.0,6.0));
        let first=c.advance(DPAD_UP,255,1.0/60.0);
        assert_eq!(first.fast_multiplier,20.0);
        assert!(first.vertical_speed>6.0 && first.vertical_speed<120.0);
        for _ in 0..120 {c.advance(DPAD_UP,255,1.0/60.0);}
        let full=c.advance(DPAD_DOWN,255,1.0/60.0);
        assert_eq!((full.up,full.down),(0.0,1.0));
        assert!((full.vertical_speed-120.0).abs()<0.01);
        let half=c.advance(0,128,1.0/60.0);
        assert!(half.fast_multiplier>10.4 && half.fast_multiplier<10.7);
    }
}
