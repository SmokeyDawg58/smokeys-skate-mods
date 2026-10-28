use super::*;

fn fixture(
    width_rows: usize,
    payload: &[u8],
) -> (AndaleVbrCompressionHeader, AndaleVbrPrelude, Vec<u8>) {
    let header = AndaleVbrCompressionHeader {
        constant_channel_map_run_count: 0,
        frame_count: 8,
        dct_minimum: -1.0,
        dct_maximum: 1.0,
        channel_blocks: Vec::new(),
        constant_palette: Vec::new(),
        frame_block_sizes: Vec::new(),
        reserved: [0; 2],
        alignment_padding: Vec::new(),
        trailing_bytes: Vec::new(),
    };
    let prelude = AndaleVbrPrelude {
        constant_values: Vec::new(),
        constant_entries: Vec::new(),
        used_table: vec![0x11; width_rows * 4],
        frame_data_offset: 0,
        frame_blocks: std::iter::once(0..payload.len()).collect(),
        trailing_bytes: Vec::new(),
    };
    (header, prelude, payload.to_vec())
}

#[test]
fn zero_count_overlaps_streams_and_preserves_negative_zero() {
    let mut payload = vec![0; 34];
    payload[1] = 4; // Only flag2 present; coefficient begins at SAME byte: code00.
    payload[33] = 0xa5; // Retained unused suffix, not coefficient stream start.
    let (header, prelude, bytes) = fixture(32, &payload);
    let block = header.decode_frame_block(&prelude, &bytes, 0).unwrap();
    assert_eq!(block.declared_significance_bit_count, 0);
    assert_eq!(block.significance_bits, bytes[1..33]);
    assert_eq!(block.coefficient_payload, bytes[1..]);
    let rows = AndaleVbrCompressionHeader::unpack_float_coefficient_rows(&prelude, block.borrowed()).unwrap();
    assert_eq!(rows.len(), 32);
    assert_eq!(
        rows[0].map(f32::to_bits),
        [0, 0, 0x8000_0000, 0, 0, 0, 0, 0]
    );
    assert!(rows[1..].iter().flatten().all(|value| value.to_bits() == 0));
    assert_eq!(
        header
            .unpack_frame_block_coefficients(&prelude, &block)
            .unwrap(),
        vec![[0; 8]; 32]
    );
}

#[test]
fn overlap_still_requires_both_logical_streams_in_bounds() {
    let (header, prelude, bytes) = fixture(32, &[0; 32]);
    assert!(matches!(
        header.decode_frame_block(&prelude, &bytes, 0),
        Err(AndaleError::Truncated { .. })
    ));
    let mut payload = vec![0xff; 33];
    payload[0] = 0;
    let (header, prelude, bytes) = fixture(32, &payload);
    let block = header.decode_frame_block(&prelude, &bytes, 0).unwrap();
    assert!(AndaleVbrCompressionHeader::unpack_float_coefficient_rows(&prelude, block.borrowed()).is_err());
}

#[test]
fn other_count_mismatches_and_ordinary_trailing_payload_remain_rejected() {
    let (header, prelude, bytes) = fixture(1, &[0, 0]);
    assert_eq!(
        header.decode_frame_block(&prelude, &bytes, 0),
        Err(AndaleError::VbrSignificanceBitCountMismatch {
            expected: 8,
            actual: 0
        })
    );
    let (header, prelude, bytes) = fixture(1, &[8, 0, 0xa5]);
    let block = header.decode_frame_block(&prelude, &bytes, 0).unwrap();
    assert_eq!(
        header.unpack_frame_block_coefficients(&prelude, &block),
        Err(AndaleError::VbrCoefficientPayloadSizeMismatch {
            expected: 0,
            actual: 1
        })
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_runout_zero_count_streams_and_all_frames_decode() {
    use crate::mount::MountedBig;
    let root = std::env::var("SKATE2_GAME_ROOT").expect("original external root");
    let archive =
        MountedBig::open(std::path::Path::new(&root).join("data/big/anim.big"), true).unwrap();
    let bytes = archive.read_path("data\\anim/OffBoard.abin").unwrap();
    let database = AndaleDatabase::parse(&bytes).unwrap();
    let clip = database
        .clips
        .iter()
        .find(|clip| clip.name == "ofb_runout_run_0_toloco")
        .unwrap();
    let part = &clip.parts[4];
    let AndaleCompressionHeader::Vbr(header) = &part.compression else {
        panic!("original VBR")
    };
    let prelude = part.vbr_prelude.as_ref().unwrap();
    assert_eq!(header.frame_count, 61);
    assert_eq!(prelude.frame_blocks.len(), 8);
    let mut negative_zeros = Vec::new();
    for index in 0..8 {
        let block = header
            .decode_frame_block(prelude, &part.compressed_data, index)
            .unwrap();
        assert_eq!(block.declared_significance_bit_count, 0);
        assert_eq!(block.significance_bits, block.coefficient_payload[..32]);
        let rows =
            AndaleVbrCompressionHeader::unpack_float_coefficient_rows(prelude, block.borrowed()).unwrap();
        negative_zeros.push(
            rows.iter()
                .flatten()
                .filter(|value| value.to_bits() == 0x8000_0000)
                .count(),
        );
    }
    assert_eq!(negative_zeros, [4, 5, 3, 5, 5, 6, 5, 6]);
    for frame in 0..61 {
        let sqt = sample_vbr_sqt(
            header,
            prelude,
            &part.compressed_data,
            part.bone_count(),
            frame,
            &mut None,
        )
        .unwrap();
        assert_eq!(sqt.len(), part.bone_count());
        assert!(
            sqt.iter()
                .flat_map(|value| value
                    .scale
                    .iter()
                    .chain(&value.rotation)
                    .chain(&value.translation))
                .all(|value| value.is_finite())
        );
    }
}
