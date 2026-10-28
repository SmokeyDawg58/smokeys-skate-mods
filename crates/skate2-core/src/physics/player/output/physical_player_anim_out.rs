//! Physical `AnimOut` ctor82e03be8 and Reset82b3a1a0, distinct from `PhysOut`.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "physical animation reset awaits actual state update consumers"
    )
)]
use crate::play_animation::{ANDALE_NULL_STRING_NAME_30, FastString30};

pub(crate) struct PhysicalAnimOut {
    pub(crate) grind_name: FastString30,
    pub(crate) flags_bits: u32,
    pub(crate) flags_known_mask: u32,
}
impl PhysicalAnimOut {
    /// `HiLOD`82e040ec uses `PhysicsSkater`40, whose row82ff0d48 has OR flags0.
    /// Reset clears five high flags; it does not establish the other27 bits.
    pub(crate) fn new() -> Self {
        Self {
            grind_name: ANDALE_NULL_STRING_NAME_30,
            flags_bits: 0,
            flags_known_mask: 0xf800_0000,
        }
    }
    pub(crate) fn reset(&mut self) {
        self.grind_name = ANDALE_NULL_STRING_NAME_30;
        self.flags_bits &= 0x07ff_ffff;
        self.flags_known_mask |= 0xf800_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_preserves_unrelated_known_bits_and_does_not_invent_unknowns() {
        let mut output = PhysicalAnimOut::new();
        assert_eq!(output.flags_known_mask, 0xf800_0000);
        output.flags_bits = 0xffff_ffff;
        output.flags_known_mask = u32::MAX;
        output.reset();
        assert_eq!(output.flags_bits, 0x07ff_ffff);
        assert_eq!(output.flags_known_mask, u32::MAX);
        assert_eq!(output.grind_name, ANDALE_NULL_STRING_NAME_30);
    }
}
