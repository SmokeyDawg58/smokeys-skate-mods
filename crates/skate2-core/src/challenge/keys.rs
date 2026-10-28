//! Hashes and Vault class keys used by the challenge system.

use skate2_formats::{attrib::string_hash64, hash::sk8_hash_value_32, state_graph::state_graph_string_hash32};

/// `Attrib::StringHash64` (collection and attribute keys).
#[must_use]
pub fn hash64(name: &str) -> u64 {
    string_hash64(name)
}

/// `stringhash32` (`0x82985a40`): state names and Hermes message kinds.
#[must_use]
pub fn hash32(name: &str) -> u32 {
    state_graph_string_hash32(name.as_bytes())
}

/// `GetHashValue32` (`tHashName32`): challenge title/name maps.
#[must_use]
pub fn hash_value32(name: &str) -> u32 {
    sk8_hash_value_32(name.as_bytes())
}

/// `StringHash64` of each class name.
pub mod class {
    pub const CHALLENGES: u64 = 0x8876_e0b5_5674_0e36;
    pub const CHALLENGE_GLOBAL_DATA: u64 = 0xa4f3_5b15_9d74_32b5;
    pub const CHALLENGE_LOCAL_DATA: u64 = 0x46a5_6add_f284_c381;
    pub const CHALLENGE_STATES: u64 = 0x2dee_2b54_d25c_86c4;
    pub const CHALLENGE_MESSAGE_HANDLERS: u64 = 0x038f_a43a_1183_77a5;
    pub const CHALLENGE_OBJECTIVE: u64 = 0x59a3_4a6c_5074_f88a;
    /// The map icons (`MapIconManager`), named by their layout's first word.
    pub const MAP_ICON: u64 = 0xf788_504a_8d92_2ced;
    pub const PROGRESSION_STATE_MACHINES: u64 = 0x2500_715b_aa0f_4d9c;
    pub const PROGRESSION_STATES: u64 = 0xe4b8_ece3_3a07_cc09;
    /// The progression handler class (`cMsg*`-named handlers with Lua source).
    pub const PROGRESSION_HANDLERS: u64 = 0x1b08_ce40_5acd_d040;
    pub const PROGRESSION_CHALLENGES: u64 = 0x62a8_c252_3da4_1d76;
    pub const PROGRESSION_MESSAGES: u64 = 0xb9e2_54f1_92f0_721a;
    pub const PROGRESSION_MESSAGE_TYPES: u64 = 0xf8de_5f97_81eb_63f0;
    pub const PROGRESSION_GROUPS: u64 = 0x9e81_4e06_4366_e4a4;
    pub const PROGRESSION_GROUP_EXITS: u64 = 0x7e94_a8f0_3622_7172;
    pub const PROGRESSION_LISTS: u64 = 0x67bf_7446_1ef3_1220;
    pub const PROGRESSION_POINT_POOLS: u64 = 0x28dd_7b80_0e72_2053;
    pub const PROGRESSION_RANKINGS: u64 = 0x53ae_88ff_9b39_6c43;
    pub const PROGRESSION_RANKING_GROUPS: u64 = 0x75f8_4936_dc1a_647d;
    pub const PROGRESSION_PINCH_POINTS: u64 = 0x78c4_14dd_e97b_c582;
    pub const FE_MOVIE: u64 = 0x18b2_7535_7481_fb52;
    pub const FE_LOCATIONS: u64 = 0xddcd_6bc2_3255_e508;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_keys_are_string_hashes() {
        for (name, key) in [
            ("challenges", class::CHALLENGES),
            ("challenge_states", class::CHALLENGE_STATES),
            ("progression_statemachines", class::PROGRESSION_STATE_MACHINES),
            ("progression_challenges", class::PROGRESSION_CHALLENGES),
            ("progression_messages", class::PROGRESSION_MESSAGES),
            ("fe_locations", class::FE_LOCATIONS),
        ] {
            assert_eq!(hash64(name), key, "{name}");
        }
        assert_eq!(hash32("cMsgStateEnter"), 4_040_627_377);
        assert_eq!(hash32("cMsgHUDYes"), 3_836_229_679);
        assert_eq!(format!("h_{:016X}", hash64("cMsgStateEnter")), "h_14C74A99A0B2F33A");
    }
}
