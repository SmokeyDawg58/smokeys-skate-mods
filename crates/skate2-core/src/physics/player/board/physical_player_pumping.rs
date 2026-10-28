//! Actual `HiLOD`+2c0 / board+108 shared Pumping allocation and Reset.
//! `DebugGraphSignal` bytes are unrepresented; constructor/reset never write them.
use crate::skeleton_root_transforms::SkeletonRootVector;
#[path = "physical_player_pumping_attributes.rs"]
pub(crate) mod attributes;
#[path = "physical_player_pumping_update.rs"]
pub(crate) mod update;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PumpingPhase {
    Unattempted,
    AllocationNull,
    Constructed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PumpingError {
    AlreadyAttempted,
    NotConstructed,
}

pub(crate) struct PhysicalPlayerPumpingOwner {
    phase: PumpingPhase,
    value: Option<Pumping>,
}

struct Pumping {
    record_valid: bool,
    smooth_raw_height_delta: f32,
    pump_time: f32,
    last_height: f32,
    pump_amount: f32,
    acceleration: f32,
    angular_speed: f32,
    absorption: f32,
    ground_normal_absorption: f32,
    ground_normal_minimum_crouch: f32,
    deck_angle_absorption: f32,
    last_height_delta: f32,
    last_position: SkeletonRootVector,
    last_normal: SkeletonRootVector,
}
impl Pumping {
    /// Constructor is the folded Reset itself: call82e040c8 ->82dd8a60.
    fn new() -> Self {
        Self {
            record_valid: false,
            smooth_raw_height_delta: 0.0,
            pump_time: 0.0,
            last_height: 0.0,
            pump_amount: 0.0,
            acceleration: 0.0,
            angular_speed: 0.0,
            absorption: 0.0,
            ground_normal_absorption: 0.0,
            ground_normal_minimum_crouch: 0.0,
            deck_angle_absorption: 0.0,
            last_height_delta: 0.0,
            last_position: SkeletonRootVector::known([0.0; 4]),
            last_normal: SkeletonRootVector::known([0.0; 4]),
        }
    }
    /// 82dd8a74 valid=false; aa4..acc scalars; ad8/adc full zero vectors.
    fn reset(&mut self) {
        self.record_valid = false;
        self.smooth_raw_height_delta = 0.0;
        self.pump_time = 0.0;
        self.last_height = 0.0;
        self.pump_amount = 0.0;
        self.acceleration = 0.0;
        self.angular_speed = 0.0;
        self.absorption = 0.0;
        self.ground_normal_absorption = 0.0;
        self.ground_normal_minimum_crouch = 0.0;
        self.deck_angle_absorption = 0.0;
        self.last_height_delta = 0.0;
        self.last_position = SkeletonRootVector::known([0.0; 4]);
        self.last_normal = SkeletonRootVector::known([0.0; 4]);
    }
}
impl PhysicalPlayerPumpingOwner {
    pub(crate) fn new() -> Self {
        Self {
            phase: PumpingPhase::Unattempted,
            value: None,
        }
    }
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "construction phase inspected by retained-owner tests; full scheduler pending"
        )
    )]
    pub(crate) fn phase(&self) -> PumpingPhase {
        self.phase
    }
    /// Source allocation040bc, nullable branch040c4, publication040d8.
    pub(crate) fn try_construct(&mut self, allocated: bool) -> Result<(), PumpingError> {
        if self.phase != PumpingPhase::Unattempted {
            return Err(PumpingError::AlreadyAttempted);
        }
        if allocated {
            self.value = Some(Pumping::new());
            self.phase = PumpingPhase::Constructed;
        } else {
            self.phase = PumpingPhase::AllocationNull;
        }
        Ok(())
    }
    pub(crate) fn try_reset(&mut self) -> Result<(), PumpingError> {
        self.value
            .as_mut()
            .ok_or(PumpingError::NotConstructed)?
            .reset();
        Ok(())
    }
    /// Board Fill82b2f808 reads this actual Pumping+2c, after its output latch sample.
    pub(crate) fn try_pump_amount(&self) -> Result<f32, PumpingError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(PumpingError::NotConstructed)?
            .pump_amount)
    }
    /// Opt-in `SKATE2_TRACE_VERT` summary; reads retained fields only.
    pub(crate) fn trace_summary(&self) -> Option<String> {
        let v = self.value.as_ref()?;
        Some(format!(
            "valid={} height={} smooth_dh={} pump_time={} pump={} accel={} angular={} absorption={} ground_absorption={} min_crouch={} deck_absorption={} last_normal_y={}",
            v.record_valid,
            v.last_height,
            v.smooth_raw_height_delta,
            v.pump_time,
            v.pump_amount,
            v.acceleration,
            v.angular_speed,
            v.absorption,
            v.ground_normal_absorption,
            v.ground_normal_minimum_crouch,
            v.deck_angle_absorption,
            v.last_normal.xyz[1],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_keeps_constructed_owner_and_clears_full_vectors() {
        let mut owner = PhysicalPlayerPumpingOwner::new();
        assert_eq!(owner.try_reset(), Err(PumpingError::NotConstructed));
        owner.try_construct(true).unwrap();
        let value = owner.value.as_mut().unwrap();
        value.last_position = SkeletonRootVector::known([1.0; 4]);
        value.last_normal = SkeletonRootVector::known([-0.0; 4]);
        value.record_valid = true;
        value.pump_time = 9.0;
        owner.try_reset().unwrap();
        assert_eq!(owner.phase(), PumpingPhase::Constructed);
        let value = owner.value.as_ref().unwrap();
        assert!(!value.record_valid);
        assert_eq!(value.pump_time.to_bits(), 0);
        assert_eq!(
            value.last_position.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            value.last_normal.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            owner.try_construct(true),
            Err(PumpingError::AlreadyAttempted)
        );
    }
    #[test]
    fn null_publication_never_becomes_constructed_reset() {
        let mut owner = PhysicalPlayerPumpingOwner::new();
        owner.try_construct(false).unwrap();
        assert_eq!(owner.phase(), PumpingPhase::AllocationNull);
        assert_eq!(owner.try_reset(), Err(PumpingError::NotConstructed));
    }
}

#[path = "physical_player_pumping_force_read.rs"]
pub(crate) mod force_read;

#[path = "physical_player_pumping_fill.rs"]
pub(crate) mod fill;
