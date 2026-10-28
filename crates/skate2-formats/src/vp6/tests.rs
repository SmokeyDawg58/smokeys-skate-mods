use super::*;

#[test]
fn equiprobable_reads_from_a_fresh_coder_return_the_raw_bytes() {
    let bytes = [0x2d, 0x50, 0x2d, 0x50, 0x35, 0x26, 0x60, 0x2a, 0xbd, 0xb8];
    let mut br = BoolCoder::new(&bytes, 0);
    assert_eq!(
        [br.bits(8), br.bits(8), br.bits(8), br.bits(8)],
        [0x2d, 0x50, 0x2d, 0x50]
    );
    assert_eq!(br.range, 256);
}

#[test]
fn huffman_tree_merges_the_two_lowest_frequencies() {
    let tree = HuffTree::build(&[1, 1, 2, 4]);
    let lengths = (0..4u8)
        .map(|token| {
            let mut depth = 0;
            let mut found = None;
            let mut stack = vec![(0usize, 1)];
            while let Some((node, d)) = stack.pop() {
                for child in tree.nodes[node] {
                    match child {
                        HuffRef::Leaf(t) if t == token => found = Some(d),
                        HuffRef::Node(n) => stack.push((usize::from(n), d + 1)),
                        HuffRef::Leaf(_) => {}
                    }
                }
                depth = depth.max(d);
            }
            found.unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(lengths, vec![3, 3, 2, 1]);
}

#[test]
fn zero_count_tokens_still_get_codes() {
    let tree = HuffTree::build(&token_frequencies(&[255; 11]));
    let mut seen = [false; 12];
    let mut stack = vec![0usize];
    while let Some(node) = stack.pop() {
        for child in tree.nodes[node] {
            match child {
                HuffRef::Leaf(t) => seen[usize::from(t)] = true,
                HuffRef::Node(n) => stack.push(usize::from(n)),
            }
        }
    }
    assert!(seen.iter().all(|&s| s));
}

#[test]
fn zero_block_runs_follow_the_native_escape_codes() {
    // 01 | 10 11 | 11 0 11 | 11 1 111100
    let mut reader = BitReader {
        buffer: &[0b0110_1111, 0b0111_1111, 0b1100_0000],
        bit: 0,
    };
    assert_eq!(zero_block_run(&mut reader), 1);
    assert_eq!(zero_block_run(&mut reader), 2 + 0b11);
    assert_eq!(zero_block_run(&mut reader), 6 + 0b11);
    assert_eq!(zero_block_run(&mut reader), 10 + 0b11_1100);
}

#[test]
fn inter_frames_need_a_key_frame() {
    let mut decoder = Vp6Decoder::new();
    assert_eq!(
        decoder.decode(&[0xf9, 0, 0x65, 0, 0, 0, 0, 0]),
        Err(Vp6Error::MissingKeyFrame)
    );
}

/// Decodes the start of a shipped movie, whose frames were compared with an
/// independent decoder's (docs/evidence/career-fmv-playback-2026-10-09.md).
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_tutorial_movie_decodes() {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let path = std::path::Path::new(&root).join("data/movies/Tutorial_Skate01_english_ntsc.vp6");
    let bytes = std::fs::read(path).unwrap();
    let movie = crate::ea_movie::EaMovie::parse_header(&bytes).unwrap();
    assert_eq!(
        (movie.width, movie.height, movie.frame_count),
        (1280, 720, 988)
    );
    let mut decoder = Vp6Decoder::new();
    let mut frames = 0;
    for chunk in crate::ea_movie::chunks(&bytes) {
        let chunk = chunk.unwrap();
        if chunk.kind(*b"EN") == crate::ea_movie::ChunkKind::Video {
            decoder.decode(chunk.payload).unwrap();
            frames += 1;
            if frames == 30 {
                break;
            }
        }
    }
    let frame = decoder.copy_display_frame().unwrap();
    assert_eq!(
        (frame.width, frame.height, frame.y.len(), frame.u.len()),
        (1280, 720, 1280 * 720, 640 * 360)
    );
}
