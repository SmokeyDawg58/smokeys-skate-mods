//! `rw::math::Random` 0x82bc34d0 / `SeedRandom` 0x82bc34e0: a six-word add-with-carry generator.
//! `Sk8AppMan::InitApp` seeds it from the time base (`mftb`), so its sequence differs every run;
//! the audio port seeds its own instance the same way.

/// `rw::math::sSeedArray`.
#[derive(Debug, Clone)]
pub struct RwRandom {
    seed: [u32; 6],
}

impl RwRandom {
    /// `SeedRandom(s)`.
    #[must_use]
    pub fn new(s: u32) -> Self {
        Self {
            seed: [
                s.wrapping_add(0xF22D_0E56),
                s.wrapping_add(0x8831_26E9),
                s.wrapping_add(0xC624_DD2F),
                s.wrapping_add(0x0702_C49C),
                s.wrapping_add(0x9E35_3F7D),
                s.wrapping_add(0x6FDF_3B64),
            ],
        }
    }

    /// `RandomImpl` 0x82bc33a8.
    pub fn next_u32(&mut self) -> u32 {
        let s = &mut self.seed;
        let (sum, overflow) = s[4].overflowing_add(s[5]);
        s[4] = sum;
        let mut carry = u32::from(overflow);
        // Each step adds the previous word and the carry, carrying out when the result does
        // not exceed the added word (with a carry in) or is below it.
        let step = |word: &mut u32, previous: u32, carry: &mut u32| {
            let value = previous.wrapping_add(*carry).wrapping_add(*word);
            *carry = u32::from(value < previous || (value == previous && *carry != 0));
            *word = value;
        };
        let previous = s[4];
        step(&mut s[3], previous, &mut carry);
        let previous = s[3];
        step(&mut s[2], previous, &mut carry);
        let previous = s[2];
        step(&mut s[1], previous, &mut carry);
        let result = s[0].wrapping_add(s[1]).wrapping_add(carry);
        s[0] = result;
        s[5] = s[5].wrapping_add(1);
        if s[5] != 0 {
            return result;
        }
        for index in (1..5).rev() {
            s[index] = s[index].wrapping_add(1);
            if s[index] != 0 {
                return result;
            }
        }
        s[0] = result.wrapping_add(1);
        s[0]
    }

    /// `Random() % count + start` as the native `divwu`/`mullw`/`subf` sequence; a zero count
    /// (the native `twllei` trap) yields `start`.
    pub fn range(&mut self, start: i32, count: u32) -> i32 {
        let value = self.next_u32();
        if count == 0 {
            return start;
        }
        (value % count).wrapping_add(start as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_zero_matches_the_static_array() {
        let random = RwRandom::new(0);
        assert_eq!(random.seed, [0xF22D_0E56, 0x8831_26E9, 0xC624_DD2F, 0x0702_C49C, 0x9E35_3F7D, 0x6FDF_3B64]);
    }
}
