//! ScoringMeter82dea250/390/3e8 and MomentumMeter82dea488/668.
use super::{
    collector_base::{MomentumMeter, ScoringMeter},
    scoring_attributes::*,
};

#[derive(Clone, Copy)]
pub(super) enum MeterKind {
    Line,
    Momentum,
}
impl ScoringMeter {
    pub(super) fn rate(&self, kind: MeterKind) -> Result<f32, ScoreError> {
        scalar(
            &self.general,
            match kind {
                MeterKind::Line => LINE_RATE,
                MeterKind::Momentum => MOMENTUM_RATE,
            },
        )
    }
    pub(super) fn maximum(&self, kind: MeterKind) -> Result<f32, ScoreError> {
        scalar(
            &self.general,
            match kind {
                MeterKind::Line => LINE_MAX,
                MeterKind::Momentum => MOMENTUM_MAX,
            },
        )
    }
    pub(super) fn seconds(&self, kind: MeterKind) -> Result<f32, ScoreError> {
        let rate = self.rate(kind)?;
        if rate <= 0.0 {
            return Err(ScoreError::Invalid("meter points per second"));
        }
        Ok(self.points / rate)
    }
    pub(super) fn stoke(&mut self, kind: MeterKind, points: f32) -> Result<(), ScoreError> {
        let sum = self.points + points;
        let maximum = self.maximum(kind)?;
        self.points = if maximum - sum >= 0.0 { sum } else { maximum };
        Ok(())
    }
    pub(super) fn decrement(
        &mut self,
        kind: MeterKind,
        fraction: f32,
        prevent_zero: bool,
        dt: f32,
    ) -> Result<bool, ScoreError> {
        self.just_reached_zero = false;
        if self.points > 0.0 {
            // Preserve native ordered assertions rather than inventing a clamp.
            if fraction > 1.0 || fraction < 0.0 {
                return Err(ScoreError::Invalid("meter decrement fraction"));
            }
            let previous = self.points;
            self.points = -(self.rate(kind)? * dt).mul_add(fraction, -previous);
            if prevent_zero
                && self.points < f32::from_bits(0x3f8020c5)
                && previous > f32::from_bits(0x3f800008)
            {
                self.points = previous;
                return Ok(true);
            }
            if self.points <= 0.0 {
                self.points = 0.0;
                self.just_reached_zero = true;
            }
        }
        Ok(false)
    }
}
impl MomentumMeter {
    pub(super) fn end_sequence(&mut self, raw_score: f32) -> Result<(), ScoreError> {
        self.base.stoke(MeterKind::Momentum, raw_score)?;
        let mut next = 1.0;
        for i in (0..3).rev() {
            if self.base.points >= scalar(&self.base.general, MOMENTUM_THRESHOLDS[i])? {
                next = scalar(&self.base.general, MOMENTUM_MULTIPLIERS[i])?;
                break;
            }
        }
        let old = self
            .multiplier
            .ok_or(ScoreError::Missing("MomentumMeter.Reset"))?;
        if next > old || raw_score > scalar(&self.base.general, MOMENTUM_ALLOW_DROP)? {
            self.multiplier = Some(next);
        }
        Ok(())
    }
    /// GetMultiplier's debug override is supplied by the existing cvar owner.
    pub(super) fn get_multiplier(&self, override_value: Option<i32>) -> Result<f32, ScoreError> {
        match override_value {
            Some(i @ 1..=3) => scalar(&self.base.general, MOMENTUM_MULTIPLIERS[i as usize - 1]),
            Some(_) => Ok(1.0),
            None => self
                .multiplier
                .ok_or(ScoreError::Missing("MomentumMeter.Reset")),
        }
    }
}
