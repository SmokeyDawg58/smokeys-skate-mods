//! Full native 64-bit `physics_wipeout/default` keys. No cached tuning values.
use crate::skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attribute {
    Squash,
    SquashSpecial,
    SkaterScalar,
    VehicleScalar,
    SkitchContactScalar,
    SkitchArmScalar,
    GroundDisplacement,
    GroundContact,
    GroundArms,
    GroundFlip,
    TrickY,
    TrickXZ,
    GroundXZ,
    GroundY,
    DeckAiScalar,
    DeckPlayerScalar,
    LightObjectScalar,
    DeckSkitchScalar,
    Vehicle,
    VehicleSkitch,
    BalanceSpeed,
    BalanceDecay,
    BalanceLimit,
    LeaningUp,
}
impl Attribute {
    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::Squash => 0xbe4f_6f73_4258_f86e,
            Self::SquashSpecial => 0x402f_b9da_fc91_e645,
            Self::SkaterScalar => 0x31ab_a15b_f262_ec6f,
            Self::VehicleScalar => 0x555b_e2e2_c5eb_88aa,
            Self::SkitchContactScalar => 0x2b7b_53a2_de1a_cd63,
            Self::SkitchArmScalar => 0x9aef_b310_7d75_e119,
            Self::GroundDisplacement => 0x5e3f_b044_48f3_1604,
            Self::GroundContact => 0xd50a_c973_07b0_117c,
            Self::GroundArms => 0x744b_b592_dd7e_0cfb,
            Self::GroundFlip => 0xf75f_a322_7c7f_007e,
            Self::TrickY => 0xef8e_891e_272c_e88b,
            Self::TrickXZ => 0x0740_bd13_1aac_09e6,
            Self::GroundXZ => 0x6731_36e3_0a37_f69d,
            Self::GroundY => 0xcb22_5317_37ee_a5e2,
            Self::DeckAiScalar => 0xb164_5ee9_67a3_68e3,
            Self::DeckPlayerScalar => 0xa9dc_ad1b_5c0f_272d,
            Self::LightObjectScalar => 0x9b5a_8ca6_5bef_ff73,
            Self::DeckSkitchScalar => 0x0618_6f09_c8f6_b6e9,
            Self::Vehicle => 0x7880_0d6e_6d22_13db,
            Self::VehicleSkitch => 0xf5b0_ae2a_1b21_fb2b,
            Self::BalanceSpeed => 0xd321_9668_7438_a6a0,
            Self::BalanceDecay => 0x7f0f_4677_552f_181f,
            Self::BalanceLimit => 0x0197_c0aa_6ede_4a4a,
            Self::LeaningUp => 0x0124_1fb4_ce7c_9f33,
        }
    }
}
/// Reuses retained Vault inheritance and missing-pointer DefaultDataArea(4).
/// Lookup remains separate from float load for `GroundArms` and `TrickY` pointers.
pub(crate) fn lookup<A: SkateboardAttributeRead>(
    attributes: &mut A,
    attribute: Attribute,
) -> Result<A::Pointer, A::Error> {
    attributes.lookup(SkateboardAttributeClass::Wipeout, attribute.key())
}
