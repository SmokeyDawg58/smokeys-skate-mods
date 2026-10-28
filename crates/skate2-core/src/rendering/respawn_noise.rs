//! Original final-blit snow: `PostFX::CreateNoiseTexture_ARGB32` 828317f0,
//! `SetNoiseParameters` 82828ac0 and `basictex_fisheye_noise_PS` 8212cff0.
//! Texture generation shares the existing process-global RW math generator.
use crate::native_skater_random::rw_math;

pub const NOISE_TEXTURE_SIZE: u32 = 64;

#[derive(Debug)]
pub struct NoiseRandomError;
impl std::fmt::Display for NoiseRandomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("original post-FX random generator lock is poisoned")
    }
}
impl std::error::Error for NoiseRandomError {}

/// Shader constants c1..c4, in original component order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseFrame {
    pub scroll: [f32; 4],
    pub fade: [f32; 4],
    pub dot1: [f32; 4],
    pub dot2: [f32; 4],
}

/// Process-lifetime phase globals `noisedot1`83057094 / `noisedot2`83057090.
/// Keep this owner across resize and scene changes, like native PostFX.
#[derive(Default)]
pub struct RespawnNoise {
    phase1: f32,
    phase2: f32,
}

impl RespawnNoise {
    /// `Blit_ToDeviceRT`82831ee0..f50 selects this only when the current
    /// VD frame contains an OOB message or the authored white noise is >0.
    /// Call once per rendered frame, not per simulation tick. `None` means
    /// absence of the VD record; `Some(0)` still selects/advances noise.
    pub fn frame(
        &mut self,
        boundary_amount: Option<f32>,
        white_noise: f32,
        paused: bool,
    ) -> Result<Option<NoiseFrame>, NoiseRandomError> {
        if boundary_amount.is_none() && !(white_noise > 0.0) {
            return Ok(None);
        }
        self.frame_with_random(boundary_amount, white_noise, paused, || {
            rw_math::global_next().map_err(|_| NoiseRandomError)
        })
        .map(Some)
    }

    fn frame_with_random<E>(
        &mut self,
        boundary_amount: Option<f32>,
        white_noise: f32,
        paused: bool,
        mut next: impl FnMut() -> Result<u32, E>,
    ) -> Result<NoiseFrame, E> {
        self.phase1 += 0.045000002;
        self.phase2 += 0.06;
        if self.phase1 >= 4.0 {
            self.phase1 -= 4.0;
        }
        if self.phase2 >= 4.0 {
            self.phase2 -= 4.0;
        }
        // 82828cb0..d40: low16 draws written in reverse call order.
        let mut scroll = [0.0; 4];
        for component in scroll.iter_mut().rev() {
            *component = (next()? & 0xffff) as f32 * (1.0 / 65536.0);
        }
        let boundary = if paused {
            0.0
        } else {
            boundary_amount.unwrap_or(0.0)
        };
        let sum = white_noise + boundary;
        // fsel(sum-1,1,sum), not a lower/upper saturation.
        let amount = if sum - 1.0 >= 0.0 { 1.0 } else { sum };
        Ok(NoiseFrame {
            scroll,
            fade: [1.0 - amount, amount, 0.0, 0.0],
            dot1: channel_weights(self.phase1),
            dot2: channel_weights(self.phase2),
        })
    }
}

/// Piecewise channel crossfade at 82828b60..c98.
fn channel_weights(phase: f32) -> [f32; 4] {
    if phase < 1.0 {
        [1.0 - phase, phase, 0.0, 0.0]
    } else if phase < 2.0 {
        [0.0, 2.0 - phase, phase - 1.0, 0.0]
    } else if phase < 3.0 {
        [0.0, 0.0, 3.0 - phase, phase - 2.0]
    } else {
        [phase - 3.0, 0.0, 0.0, 4.0 - phase]
    }
}

/// 64x64, one mip, RGBA8. Native ARGB words become RGBA bytes for the host
/// texture. Every channel reads bits8..15 of a separate RW random word and
/// compares strictly >128 (raw `extrwi` / `bgt` 828318f8..1950).
pub fn create_noise_texture_rgba() -> Result<Vec<u8>, NoiseRandomError> {
    create_texture_with_random(|| rw_math::global_next().map_err(|_| NoiseRandomError))
}

fn create_texture_with_random<E>(mut next: impl FnMut() -> Result<u32, E>) -> Result<Vec<u8>, E> {
    let mut rgba = Vec::with_capacity((NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE * 4) as usize);
    for _ in 0..NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE {
        for _ in 0..4 {
            let byte = (next()? >> 8) & 0xff;
            rgba.push(if byte > 128 { 255 } else { 0 });
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[test]
    fn texture_uses_middle_byte_strict_threshold_and_four_draws_per_pixel() {
        let draws = [0x0081_8000, 0x0000_8100, 0xffff_0000, 0x0000_ff00];
        let mut count = 0;
        let texture = create_texture_with_random(|| {
            let value = draws[count % 4];
            count += 1;
            Ok::<_, Infallible>(value)
        })
        .unwrap();
        assert_eq!(count, 64 * 64 * 4);
        assert_eq!(&texture[..4], &[0, 255, 0, 255]);
        assert!(texture.chunks_exact(4).all(|p| p == [0, 255, 0, 255]));
    }

    #[test]
    fn absent_frame_does_not_advance_and_pause_only_suppresses_boundary() {
        let mut noise = RespawnNoise::default();
        assert_eq!(noise.frame(None, 0.0, false).unwrap(), None);
        assert_eq!((noise.phase1, noise.phase2), (0.0, 0.0));
        let mut word = 0;
        let frame = noise
            .frame_with_random(Some(0.8), 0.25, true, || {
                word += 1;
                Ok::<_, Infallible>(word)
            })
            .unwrap();
        assert_eq!(
            frame.scroll,
            [4.0 / 65536.0, 3.0 / 65536.0, 2.0 / 65536.0, 1.0 / 65536.0]
        );
        assert_eq!(frame.fade, [0.75, 0.25, 0.0, 0.0]);
        assert_eq!(frame.dot1, [1.0 - 0.045000002, 0.045000002, 0.0, 0.0]);
    }

    #[test]
    fn cyclic_weights_cross_each_channel_and_wrap_without_resetting_fraction() {
        assert_eq!(channel_weights(0.5), [0.5, 0.5, 0.0, 0.0]);
        assert_eq!(channel_weights(1.5), [0.0, 0.5, 0.5, 0.0]);
        assert_eq!(channel_weights(2.5), [0.0, 0.0, 0.5, 0.5]);
        assert_eq!(channel_weights(3.5), [0.5, 0.0, 0.0, 0.5]);
        let mut noise = RespawnNoise {
            phase1: 3.98,
            phase2: 3.98,
        };
        let frame = noise
            .frame_with_random(Some(1.2), 0.0, false, || Ok::<_, Infallible>(0))
            .unwrap();
        assert_eq!(noise.phase1, (3.98_f32 + 0.045000002) - 4.0);
        assert_eq!(noise.phase2, (3.98_f32 + 0.06) - 4.0);
        assert_eq!(frame.fade, [0.0, 1.0, 0.0, 0.0]);
    }
}
