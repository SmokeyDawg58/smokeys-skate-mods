use super::*;
#[test]
fn ordered_duplicates_reverse_points_and_ignored_timing() {
    let patterns = InputPatterns::parse(b"global_tolerance_dist .4 global_tolerance_time 99 global_tolerance_speed 2 global_anticipation_delay 8 pattern Ollie tolerance_dist .55 coord -0.0 1 coord 0 -1 min_time 900 pattern Ollie coord 1 0 coord -1 0").unwrap();
    let records = patterns.patterns();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].name(), records[1].name());
    assert!(!Arc::ptr_eq(&records[0], &records[1]));
    assert_eq!(
        records[0].points()[0].map(f32::to_bits),
        [0, (-1.0_f32).to_bits()]
    );
    assert_eq!(records[0].points()[1][0].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(
        records[0].squared_tolerance().to_bits(),
        (0.55_f32 * 0.55).to_bits()
    );
    assert_eq!(
        records[1].squared_tolerance().to_bits(),
        (0.4_f32 * 0.4).to_bits()
    );
}
#[test]
fn unsupported_and_malformed_inputs_do_not_publish_collection() {
    for bytes in [
        b"pattern Empty".as_slice(),
        b"pattern One coord 0 0",
        b"coord 0 0",
        b"pattern X clock 3 1",
        b"pattern X coord NaN 0",
        b"pattern X coord 0",
        b"# comment",
        b"pattern X coord 0x1 0",
        b"pattern X coord 1e0 0",
        b"pattern X\x00",
    ] {
        assert!(InputPatterns::parse(bytes).is_err(), "{bytes:?}");
    }
}
/// Called by root's existing original-data corpus, no ignored standalone test.
pub(crate) fn check_original(files: &[Vec<u8>]) {
    let counts = [78, 78, 78, 21, 14, 10, 6];
    assert_eq!(files.len(), counts.len());
    for (bytes, count) in files.iter().zip(counts) {
        let parsed = InputPatterns::parse(bytes).unwrap();
        assert_eq!(parsed.patterns().len(), count);
        assert!(
            parsed
                .patterns()
                .iter()
                .all(|p| (2..=4).contains(&p.points().len()))
        );
    }
}
