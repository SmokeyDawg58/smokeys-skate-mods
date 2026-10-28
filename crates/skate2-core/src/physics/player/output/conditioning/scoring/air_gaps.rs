//! Original AirGroundData82df44d8 / AirGapData82df45f0 and gap scoring.
use super::{
    super::simple::GroundRemainder,
    air_collector::{AirCollector, AirGapData, AirGroundData},
    scoring_attributes::{ScoreError, curve, scalar},
};
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningVector, try_length};

impl AirGroundData {
    pub(super) fn from_ground(ground: &GroundRemainder, takeoff: Option<&Self>) -> Self {
        let mut result = Self {
            hips_position: ground.below_hips_ground_data_start_pos,
            height_above_ground: 0.0,
            relative_height_above_ground: 0.0,
            is_above_ground: false,
            is_above_grass: false,
            is_above_stairs: false,
        };
        if ground.below_hips_ground_data_is_valid {
            result.is_above_ground = true;
            let start_y = result.hips_position.xyz[1];
            let hit_y = ground.below_hips_ground_data_position.xyz[1];
            result.height_above_ground = start_y - hit_y;
            result.relative_height_above_ground = if let Some(takeoff) = takeoff {
                // VMX compare82df4570 chooses the lower of the two probe starts.
                let reference_y = if takeoff.hips_position.xyz[1] > start_y {
                    start_y
                } else {
                    takeoff.hips_position.xyz[1]
                };
                reference_y - hit_y
            } else {
                result.height_above_ground
            };
            result.is_above_grass = ground.below_hips_ground_data_surface_id & 0x7f == 10;
            result.is_above_stairs = ground.below_hips_ground_data_surface_id & 0xf80 == 0x400;
        }
        result
    }
}

impl AirGapData {
    pub(super) fn update(
        &mut self,
        ground: &AirGroundData,
        gapping: bool,
        can_score: bool,
    ) -> Result<(), ScoreError> {
        if gapping && !self.is_currently_gapping {
            self.current_gap_distance = 0.0;
            self.is_currently_gapping = true;
            self.current_gap_start_position = ground.hips_position;
        }
        if self.is_currently_gapping {
            let mut delta = ground
                .hips_position
                .zip(self.current_gap_start_position, |a, b| a - b);
            delta.xyz[1] = 0.0;
            let distance = try_length(delta.operand(), &mut HostReckoningNumeric)
                .map_err(ScoreError::Numeric)?;
            if gapping {
                self.current_gap_distance = distance;
            } else {
                if can_score {
                    self.accumulated_gap_distance = distance + self.accumulated_gap_distance;
                    self.gap_count = self.gap_count.wrapping_add(1);
                    if distance - self.maximum_gap_distance >= 0.0 {
                        self.maximum_gap_distance = distance;
                    }
                }
                self.current_gap_distance = 0.0;
                self.is_currently_gapping = false;
            }
        }
        Ok(())
    }
}

impl AirCollector {
    pub(super) fn calculate_gap_score(&self) -> Result<f32, ScoreError> {
        let mut total = 0.0;
        for (gap, offset) in self.gaps.iter().zip([1440, 1280, 1360, 1200]) {
            let points = curve(
                &self.base.states,
                offset,
                gap.accumulated_gap_distance + gap.current_gap_distance,
            )?;
            total = points + total;
        }
        Ok(total)
    }
    pub(super) fn update_gap_detection(
        &mut self,
        ground: &GroundRemainder,
    ) -> Result<f32, ScoreError> {
        let current = AirGroundData::from_ground(ground, Some(&self.takeoff_ground));
        let grass = current.is_above_grass && !self.takeoff_ground.is_above_grass;
        let small = current.relative_height_above_ground
            > scalar(&self.base.states, 0x0ba6_f149_f146_2eb0)?;
        let large = current.relative_height_above_ground
            > scalar(&self.base.states, 0x4474_6535_286e_1613)?;
        for (gap, active) in
            self.gaps
                .iter_mut()
                .zip([grass, current.is_above_stairs, small, large])
        {
            // The leaf callee preserves r6=1 across all four native calls.
            gap.update(&current, active, true)?;
        }
        self.calculate_gap_score()
    }
    pub(super) fn activity_sequence_worthy(&self, was_in_sequence: bool) -> bool {
        was_in_sequence || self.air_frames > 5 || self.base.animation_trick.bits & 0x0100_0000 != 0
    }
    pub(super) fn line_decrement_fraction(&self) -> Result<f32, ScoreError> {
        scalar(&self.base.states, 0xc33e_4aa5_07a2_cb4c)
    }
    pub(super) fn momentum_decrement_fraction(&self) -> Result<f32, ScoreError> {
        scalar(&self.base.states, 0xbc91_e9c8_4e09_9f2c)
    }
}
