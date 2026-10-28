//! rw::math::RandomImpl82bc33a8 and SeedRandom82bc34e0. This is the source
//! GLOBAL generator, distinct from each Sk8::Random instance. Their carry
//! comparisons differ; replacing one with the other changes the sequence.
use super::INITIAL_SEEDS;
use std::sync::{Mutex, OnceLock};

pub(crate) struct RwMathRandom {
    values: [u32; 6],
}
impl RwMathRandom {
    pub(crate) const fn from_seed(seed: u32) -> Self {
        Self {
            values: [
                INITIAL_SEEDS[0].wrapping_add(seed),
                INITIAL_SEEDS[1].wrapping_add(seed),
                INITIAL_SEEDS[2].wrapping_add(seed),
                INITIAL_SEEDS[3].wrapping_add(seed),
                INITIAL_SEEDS[4].wrapping_add(seed),
                INITIAL_SEEDS[5].wrapping_add(seed),
            ],
        }
    }
    pub(crate) fn next(&mut self) -> u32 {
        let last = self.values[5];
        let mut sum = self.values[4].wrapping_add(last);
        let mut carry = sum < last;
        self.values[4] = sum;
        for index in (1..=3).rev() {
            let previous = sum;
            sum = sum
                .wrapping_add(u32::from(carry))
                .wrapping_add(self.values[index]);
            carry = sum < previous || (sum == previous && carry);
            self.values[index] = sum;
        }
        self.values[0] = self.values[0]
            .wrapping_add(sum)
            .wrapping_add(u32::from(carry));
        self.values[5] = last.wrapping_add(1);
        if last == u32::MAX {
            for index in (0..=4).rev() {
                self.values[index] = self.values[index].wrapping_add(1);
                if self.values[index] != 0 {
                    break;
                }
            }
        }
        self.values[0]
    }
}

#[derive(Debug)]
pub(crate) struct RwRandomLockError;
/// Math::DiscreteRandom<VecFloat>82d1e7d8, for splatted scalar probabilities.
/// The VMX >= comparison rejects NaN, unlike the scalar census overload's ble.
pub(crate) fn vector_discrete_choice(probabilities: &[f32], word: u32) -> Option<usize> {
    let draw = (word % 100 + 1) as f32;
    let mut cumulative = 0.;
    for (index, probability) in probabilities.iter().enumerate() {
        cumulative += probability;
        if cumulative * 100. >= draw {
            return Some(index);
        }
    }
    None
}
/// Math::BernoulliRandom826eaa98; consumes a draw even for probability0/1.
pub(crate) fn bernoulli_choice(probability: f32, word: u32) -> bool {
    probability * 100. >= (word % 100 + 1) as f32
}
/// One host-process occurrence, matching the native global sSeedArray.
/// InitApp82734a64 uses the low32 hardware time-base word as seed. The host
/// substitutes its clock for that nondeterministic initial word; it does not
/// claim the original application's full pre-game RNG-consumption sequence.
pub(crate) fn global_next() -> Result<u32, RwRandomLockError> {
    static RANDOM: OnceLock<Mutex<RwMathRandom>> = OnceLock::new();
    let owner = RANDOM.get_or_init(|| {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or_else(|e| e.duration().as_nanos() as u32, |d| d.as_nanos() as u32);
        Mutex::new(RwMathRandom::from_seed(seed))
    });
    Ok(owner.lock().map_err(|_| RwRandomLockError)?.next())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carried_equality_and_full_tail_wrap_follow_global_variant() {
        let mut random = RwMathRandom {
            values: [0, 0, 0, u32::MAX, u32::MAX, 1],
        };
        // Word3's zero equality must retain the incoming carry through2/1/0.
        assert_eq!(random.next(), 1);
        assert_eq!(random.values, [1, 1, 1, 0, 0, 2]);
        let mut random = RwMathRandom { values: [0; 6] };
        random.values[5] = u32::MAX;
        assert_eq!(random.next(), 0);
        assert_eq!(random.values, [0, 0, 0, 0, 0, 0]);
    }
}
