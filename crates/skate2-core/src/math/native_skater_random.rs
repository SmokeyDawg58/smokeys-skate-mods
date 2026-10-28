//! Shared `Sk8::Random` storage and exact integer `RandValue82a6dbc8`.
//! Float range conversion is separate; command consumption discards that result.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared RNG advance awaits the production command schedule; Reset/SetSeed already serve retained constructors"
    )
)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[path = "native_rw_math_random.rs"]
pub(crate) mod rw_math;
const INITIAL_SEEDS: [u32; 6] = [
    0xf22d_0e56,
    0x8831_26e9,
    0xc624_dd2f,
    0x0702_c49c,
    0x9e35_3f7d,
    0x6fdf_3b64,
];

/// Same source-global logical access flag, supplied to all participating owners.
/// An application constructs this once, independently of any generator reset.
pub(crate) struct NativeRandomGate {
    locked: AtomicBool,
}
impl NativeRandomGate {
    /// The executable has one msLocked flag, shared by Simulation and graph RNGs.
    pub(crate) fn production_shared() -> Arc<Self> {
        static GATE: std::sync::OnceLock<Arc<NativeRandomGate>> = std::sync::OnceLock::new();
        Arc::clone(GATE.get_or_init(Self::from_static_initializer))
    }
    /// Actual static byte8308c740 starts0; this is not per-RNG constructor state.
    pub(crate) fn from_static_initializer() -> Arc<Self> {
        Arc::new(Self {
            locked: AtomicBool::new(false),
        })
    }
    pub(crate) fn set_locked(&self, locked: bool) {
        self.locked.store(locked, Ordering::Relaxed);
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NativeRandomLocked;
impl std::fmt::Display for NativeRandomLocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("native Random access is locked")
    }
}
#[derive(Clone)]
pub(crate) struct NativeSkaterRandom {
    pub(crate) seed: u32,
    pub(crate) index: u32,
    pub(crate) values: [u32; 6],
}
impl NativeSkaterRandom {
    /// LW.GetRandom82e4e838: unsigned conversion, rounded multiply, then
    /// fmadds(rangeProduct,2^-32,min). Distinct from skater range consumers.
    pub(crate) fn try_living_world_range(
        &mut self,
        gate: &NativeRandomGate,
        minimum: f32,
        maximum: f32,
    ) -> Result<f32, NativeRandomLocked> {
        let word = self.try_next(gate)?;
        let difference = maximum - minimum;
        let product = word as f32 * difference;
        Ok(product.mul_add(f32::from_bits(0x2f80_0000), minimum))
    }
    pub(crate) const fn reset() -> Self {
        Self {
            seed: 0,
            index: 0,
            values: INITIAL_SEEDS,
        }
    }
    pub(crate) fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
        self.index = 0;
        self.values = INITIAL_SEEDS.map(|initial| seed.wrapping_add(initial));
    }
    pub(crate) fn try_next(&mut self, gate: &NativeRandomGate) -> Result<u32, NativeRandomLocked> {
        if gate.locked.load(Ordering::Relaxed) {
            return Err(NativeRandomLocked);
        }
        Ok(self.next_value())
    }

    /// `Random::RandValue` 0x82a6dbc8 past its `msLocked` assert.
    fn next_value(&mut self) -> u32 {
        let last = self.values[5];
        let fourth = self.values[4];
        let sum = fourth.wrapping_add(last);
        let carry = u32::from(sum < last || sum < fourth);
        let third = self.values[3];
        self.values[4] = sum;
        let second = self.values[2];
        let first = self.values[1];
        let sum = third.wrapping_add(sum).wrapping_add(carry);
        let head = self.values[0];
        self.values[3] = sum;
        // Native compares the final wrapped sum, not a generic add-with-carry.
        let next = second
            .wrapping_add(sum)
            .wrapping_add(u32::from(sum < third));
        self.values[2] = next;
        let next_first = first
            .wrapping_add(next)
            .wrapping_add(u32::from(next < second));
        self.values[1] = next_first;
        self.values[5] = last.wrapping_add(1);
        let mut result = head
            .wrapping_add(next_first)
            .wrapping_add(u32::from(next_first < first));
        self.values[0] = result;
        if last == u32::MAX {
            for slot in (1..=4).rev() {
                self.values[slot] = self.values[slot].wrapping_add(1);
                if self.values[slot] != 0 {
                    break;
                }
                if slot == 1 {
                    result = result.wrapping_add(1);
                    self.values[0] = result;
                }
            }
        }
        self.index = self.index.wrapping_add(1);
        result
    }
}

/// Selected actual Simulation RNG field with the supplied shared global gate.
/// This does not construct a Simulation or acknowledge its wider Reset sequence.
#[derive(Clone)]
pub(crate) struct SimulationCommandRandom {
    pub(crate) random: NativeSkaterRandom,
    gate: Arc<NativeRandomGate>,
}
impl SimulationCommandRandom {
    /// Simulation ctor828a8724 resets its field before workspace construction.
    pub(crate) fn construct_field(gate: Arc<NativeRandomGate>) -> Self {
        Self {
            random: NativeSkaterRandom::reset(),
            gate,
        }
    }
    /// Simulation Reset8289eee8, before the existing input-owner reset.
    pub(crate) fn set_seed_before_input_reset(&mut self, seed: u32) {
        self.random.set_seed(seed);
    }
    pub(crate) fn unlock_for_commands(&self) {
        self.gate.set_locked(false);
    }
    /// `Random.GetRandomNumber` 8288adb8 sets msLocked again after its draw.
    pub(crate) fn lock_after_commands(&self) {
        self.gate.set_locked(true);
    }
    /// `Simulation::GetRandomValue` 0x8289e870, whose lock test is only an assert: the draw is
    /// made either way.
    pub(crate) fn random_value(&mut self) -> u32 {
        self.random.next_value()
    }
    pub(crate) fn try_advance_for_discarded_range(&mut self) -> Result<(), NativeRandomLocked> {
        self.random.try_next(&self.gate).map(|_| ())
    }
    ///Simulation.GetRandom8289e808: same unsigned conversion and fmuls/fmadds
    ///as LW.GetRandom, but advances THIS Simulation field, not Manager RNG.
    pub(crate) fn try_range(&mut self, minimum:f32, maximum:f32)->Result<f32,NativeRandomLocked>{
        self.random.try_living_world_range(&self.gate,minimum,maximum)
    }
    /// `Simulation::GetRandom` 0x8289e808 as `IRandom::GetRandom` within the simulation update,
    /// which `SimController::Update1` 0x828a74f0 unlocks: the challenge crowd generators' jitter.
    #[expect(clippy::cast_precision_loss, reason = "the native unsigned conversion")]
    pub(crate) fn range(&mut self, minimum: f32, maximum: f32) -> f32 {
        let product = self.random.next_value() as f32 * (maximum - minimum);
        product.mul_add(f32::from_bits(0x2f80_0000), minimum)
    }
}
#[cfg(test)]
#[path = "native_skater_random_tests.rs"]
mod tests;
