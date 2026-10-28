//! Buffered `Fetch82a2f740` time fields and `EnQueueCmd82a30458` decoding.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferedFetchTiming {
    pub(crate) previous: u32,
    pub(crate) delta_or_current: u16,
    pub(crate) looped: bool,
    pub(crate) blend_frames: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FetchTimingError {
    NonFiniteTime,
    IntegerConversion,
    InvalidFetchMode(u32),
    InvalidFrame,
}
impl std::fmt::Display for FetchTimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFetchMode(mode) => write!(f, "unknown native fetch mode {mode}"),
            other => write!(f, "native fetch timing: {other:?}"),
        }
    }
}

const PREVIOUS_SCALE: f32 = f32::from_bits(0x44b6_0b33);
const PREVIOUS_STEP: f32 = f32::from_bits(0x3a34_002d);
const DELTA_SCALE: f32 = f32::from_bits(0x447f_e000);
// Verified consumer constant from EnQueueCmd82a3062c.
const DELTA_STEP: f32 = f32::from_bits(0x3a80_1002);

impl BufferedFetchTiming {
    #[expect(
        clippy::cast_precision_loss,
        reason = "source converts an 18-bit time integer exactly to float"
    )]
    pub(crate) fn encode(
        previous: f32,
        current: f32,
        loops: u32,
        mode: u32,
        default_mode: u32,
    ) -> Result<Self, FetchTimingError> {
        if !previous.is_finite() || !current.is_finite() {
            return Err(FetchTimingError::NonFiniteTime);
        }
        let mode = if mode == 2 { default_mode } else { mode };
        if mode > 2 {
            return Err(FetchTimingError::InvalidFetchMode(mode));
        }
        // Source inserts low18 bits, rather than saturating long animations.
        let previous = low_integer_word((previous * PREVIOUS_SCALE).floor())? & 0x0003_ffff;
        let decoded = previous as f32 * PREVIOUS_STEP;
        let delta = if current < decoded - PREVIOUS_STEP {
            current
        } else {
            let difference = current - decoded;
            if difference.abs() > PREVIOUS_STEP {
                difference
            } else {
                0.0
            }
        };
        let delta_or_current = low_integer_word((delta * DELTA_SCALE).floor())? & 0x7ff;
        Ok(Self {
            previous,
            delta_or_current: u16::try_from(delta_or_current).expect("11-bit mask"),
            looped: loops != 0,
            blend_frames: mode == 1,
        })
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "source converts an 18-bit time integer exactly to float"
    )]
    pub(crate) fn decode(self) -> (f32, f32) {
        let previous = self.previous as f32 * PREVIOUS_STEP;
        let delta = f32::from(self.delta_or_current) * DELTA_STEP;
        (previous, if self.looped { delta } else { delta + previous })
    }
}

// fctidz -> low32 extraction; out-of-domain conversion remains explicit.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "source signed integer conversion followed by low word extraction"
)]
pub(crate) fn low_integer_word(value: f32) -> Result<u32, FetchTimingError> {
    if !value.is_finite()
        || !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&f64::from(value))
    {
        return Err(FetchTimingError::IntegerConversion);
    }
    Ok((value as i64) as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FetchFramePair {
    pub(crate) frames: [u16; 2],
    pub(crate) blend: u16,
}
/// `BatchFetch82d73950`: actual decoded seconds, clamped frame position, packed blend.
pub(crate) fn frame_pair(
    time: f32,
    fps: f32,
    frames: f32,
    blend_frames: bool,
    truncation_offset: f32,
) -> Result<FetchFramePair, FetchTimingError> {
    let last = frames - 1.0;
    let position = fps * time;
    let position = if position - last >= 0.0 {
        last
    } else {
        position
    };
    let offset = if blend_frames { 0.0 } else { truncation_offset };
    let integer = position + offset;
    if !integer.is_finite()
        || !(0.0..65_536.0).contains(&integer)
        || !last.is_finite()
        || !(0.0..65_536.0).contains(&last)
    {
        return Err(FetchTimingError::InvalidFrame);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked source frame conversion domain"
    )]
    let first = integer as u16;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked source last-frame conversion domain"
    )]
    let last = last as u16;
    let second = if blend_frames {
        first.saturating_add(1).min(last)
    } else {
        first
    };
    let fraction = if blend_frames {
        position - f32::from(first)
    } else {
        1.0
    };
    let blend = low_integer_word(fraction * 65_535.0)? & 0xffff;
    Ok(FetchFramePair {
        frames: [first, second],
        blend: u16::try_from(blend).expect("16-bit mask"),
    })
}
