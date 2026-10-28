//! Original TU3 assembly/part collision groups shared by world and actor queries.
pub(super) fn pair_allowed(a: u32, b: u32) -> bool {
    //82765EF0..827679AC, Island.flags==3: inverted native culling bits.
    //GroundPipeline8277BD40..BD7C uses this same table before surface filtering.
    //Skate2 827B8C10 has19 groups and different pairs; do not import it.
    const ALLOWED: [u32; 21] = [
        0x1876fd, 0, 0x120001, 1, 0x79d1, 0x1079e1, 0x1079f1, 0x279f1, 0x1676f0, 0x7101, 0x5101,
        0x1008f0, 0x1d77f1, 0x233f1, 0xa57f1, 0, 0x1000, 0x6184, 0x1100, 0x5001, 0x1965,
    ];
    b < 21
        && ALLOWED
            .get(a as usize)
            .is_some_and(|row| row & (1 << b) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_biped_parts_exclude_world_but_keep_hand_and_controller_contacts() {
        for group in [17,18] {assert!(!pair_allowed(group,0));}
        for group in [4,5,6,7,19,20] {assert!(pair_allowed(group,0));}
        assert!(pair_allowed(7,17));
        assert!(!pair_allowed(4,17));
        assert!(pair_allowed(8,18));
        assert!(!pair_allowed(21,0));
    }
    #[test]
    fn original_collision_table_is_symmetric() {
        for a in 0..21 {for b in 0..21 {assert_eq!(pair_allowed(a,b),pair_allowed(b,a));}}
    }
}
