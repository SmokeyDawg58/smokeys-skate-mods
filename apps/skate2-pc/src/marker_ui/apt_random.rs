//! APT's `Math.random`: the one Mersenne Twister (829e0680/829e9288) every translated movie's
//! ActionScript draws from, including its old seed expansion and nonstandard 9d2c56ff
//! tempering mask.

pub(super) struct AptRandom {
    state: [u32; 624],
    next: usize,
}

impl AptRandom {
    pub fn new() -> Self {
        let mut state = [0u32; 624];
        state[0] = 4357;
        for i in 1..624 {
            state[i] = state[i - 1].wrapping_mul(69069);
        }
        Self { state, next: 624 }
    }

    pub fn next(&mut self) -> f32 {
        if self.next == 624 {
            for i in 0..624 {
                let following = self.state[(i + 1) % 624];
                let mix = (self.state[i] & 0x8000_0000) | (following & 0x7fff_ffff);
                self.state[i] = self.state[(i + 397) % 624] ^ (mix >> 1) ^ if following & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.next = 0;
        }
        let mut word = self.state[self.next];
        self.next += 1;
        word ^= word >> 11;
        word ^= (word << 7) & 0x9d2c_56ff;
        word ^= (word << 15) & 0xefc6_0000;
        word ^= word >> 18;
        word as f32 * 2.328_306_4e-10
    }

    /// `Math.floor(Math.random() * count)`, which the scripts reset to 0 outside `0..count`.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a whole number in range")]
    pub fn index(&mut self, count: u8) -> usize {
        let value = (self.next() * f32::from(count)).floor();
        if value < 0. || value > f32::from(count - 1) { 0 } else { value as usize }
    }
}
