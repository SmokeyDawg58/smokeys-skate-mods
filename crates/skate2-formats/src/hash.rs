//! String identifiers used by the game and `RenderWare` asset registries.

pub const RW_HASH64_INITIAL: u64 = 0xCBF2_9CE4_8422_2325;
pub const RW_HASH64_PRIME: u64 = 0x0000_0100_0000_01B3;
pub const RW_HASH32_INITIAL: u32 = 0x811C_9DC5;
pub const RW_HASH32_PRIME: u32 = 0x0100_0193;

/// Reconstructs `tHashName32::getHashValue` at `0x82a74438` and its public
/// `GetHashValue32` wrapper at `0x82a74490`.
///
/// The reference consumes bytes as signed `char` values, stops at the first
/// NUL, and applies every operation with 32-bit wrapping semantics.
#[must_use]
pub fn sk8_hash_value_32(value: &[u8]) -> u32 {
    let mut hash = 0_u32;
    let mut index = 0;
    while index < value.len() {
        let byte = value[index];
        if byte == 0 {
            break;
        }
        let signed_byte = u32::from(byte).wrapping_sub(u32::from(byte >> 7).wrapping_shl(8));
        hash = hash.wrapping_shl(4).wrapping_add(signed_byte);
        let high_bits = hash & 0xf000_0000;
        if high_bits != 0 {
            hash ^= (high_bits >> 23) ^ high_bits;
        }
        index += 1;
    }
    hash
}

/// Reconstructs `rw::RwHash32String` at `0x82bb57c8`.
#[must_use]
pub fn rw_hash32_string(value: &str) -> u32 {
    value.bytes().fold(RW_HASH32_INITIAL, |hash, byte| {
        hash.wrapping_mul(RW_HASH32_PRIME) ^ u32::from(byte)
    })
}

/// Reconstructs `rw::RwHash64String` at `0x82bb5800`.
#[must_use]
pub fn rw_hash64_string(value: &str) -> u64 {
    value.bytes().fold(RW_HASH64_INITIAL, |hash, byte| {
        hash.wrapping_mul(RW_HASH64_PRIME) ^ u64::from(byte)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_main_dmo_hull_name_like_sk8() {
        assert_eq!(
            sk8_hash_value_32(b"data\\content\\world\\dmo\\DMO"),
            0x048f_3c1f
        );
    }

    #[test]
    fn sk8_hash_uses_signed_bytes_and_stops_at_nul() {
        assert_eq!(sk8_hash_value_32(&[0xff]), 0x0fff_fe1f);
        assert_eq!(sk8_hash_value_32(b"DMO\0ignored"), 0x0000_491f);
    }

    #[test]
    fn hashes_default_recipe_name_like_renderware() {
        assert_eq!(rw_hash64_string("default"), 0xDEAD_4915_C9C6_C0D8);
    }

    #[test]
    fn hashes_instance_type_names_like_renderware() {
        assert_eq!(rw_hash32_string("Instance_DMO"), 0x06D0_51D1);
        assert_eq!(rw_hash32_string("Instance_LivingWorld"), 0xE7F4_E308);
        assert_eq!(rw_hash32_string("Instance_Environment"), 0xACB3_1C9A);
    }
}
