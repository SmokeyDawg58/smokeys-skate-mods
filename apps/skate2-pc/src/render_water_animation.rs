//! cPCAWaterAnimationData::Init 827e9498 / Update 827d02f0, standard table path.
#[path = "render_water_constants.rs"]
mod constants;

pub(crate) struct WaterAnimation {
    frame_time: f32,
    current_frame: u16,
    globals: [[f32; 4]; 7],
}
impl Default for WaterAnimation {
    fn default() -> Self {
        let mut globals = [[0.; 4]; 7];
        globals[0] = [1.; 4];
        Self {
            frame_time: 0.,
            current_frame: 0,
            globals,
        }
    }
}
impl WaterAnimation {
    pub(crate) fn update(&mut self, delta: f32) {
        self.frame_time += delta;
        // Native uses a strict comparison and subtracts ONCE per presentation
        // update, including zero-delta updates. Do not replace with a while loop.
        if self.frame_time > 1.0_f32 / 30.0 {
            self.frame_time -= 1.0_f32 / 30.0;
            self.current_frame = self.current_frame.wrapping_add(1);
        }
        let index = usize::from(self.current_frame % 30);
        let scale = 1.0_f32 / 255.0;
        for channel in 0..3 {
            self.globals[0][channel] = f32::from_bits(constants::MEAN[index][channel]) * scale;
        }
        for (dst, bits) in self.globals[1..]
            .iter_mut()
            .flatten()
            .zip(constants::WEIGHTS[index])
        {
            *dst = f32::from_bits(bits) * scale;
        }
    }
    pub(crate) fn globals(&self) -> [[f32; 4]; 7] {
        self.globals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_strict_boundary_and_single_step_catchup() {
        let mut water = WaterAnimation::default();
        water.update(1. / 30.);
        assert_eq!(water.current_frame, 0);
        water.update(1. / 60.);
        assert_eq!(water.current_frame, 1);
        water.update(0.2);
        assert_eq!(water.current_frame, 2);
        water.update(0.);
        assert_eq!(water.current_frame, 3);
    }
    #[test]
    fn native_tables_wrap_and_preserve_channel_order() {
        let mut water = WaterAnimation::default();
        water.update(0.);
        let first = water.globals();
        assert_eq!(first[0][3], 1.);
        assert!((first[0][1] - 0.987569).abs() < 0.00001);
        water.current_frame = 30;
        water.update(0.);
        assert_eq!(first, water.globals());
        for channel in 0..3 {
            for weight in 0..8 {
                assert_eq!(
                    first[1 + channel * 2 + weight / 4][weight % 4],
                    f32::from_bits(constants::WEIGHTS[0][channel * 8 + weight]) * (1.0_f32 / 255.0)
                );
            }
        }
    }
}
