use super::*;
#[test]
fn header_word_updates_retain_reserved_bits() {
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    header.set_word(188, 0x1234_abcd);
    header.bits(188, 0xf0, 3 << 4);
    header.bits(188, 0xff00, 0x42 << 8);
    header.bits(188, 0xffff_0000, 0x100 << 16);
    assert_eq!(header.word(188), 0x0100_423d);
    header.set_word(192, 0xabcd_0fed);
    header.bits(192, 0xf000, 0x9000);
    header.bits(192, 0xffff_0000, 4 << 16);
    assert_eq!(header.word(192), 0x0004_9fed);
}
#[test]
fn frame_view_uses_actual_gather_tail_after_relative_pose_offset() {
    let mut buffers =
        SelectedPresentationBuffers::new_uncompressed_host_policy(256, 320, 0).unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&1_u32.to_be_bytes(), 4).unwrap();
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    header.bits(188, 0xffff_0000, 208 << 16);
    let hips = std::array::from_fn(|i| f32::from(u16::try_from(i).unwrap()));
    header.matrix(0, hips);
    buffers.submit_data(&header.bytes, 16).unwrap();
    buffers.submit_data(&[0, 0, 0, 0, 0, 0], 2).unwrap();
    let frame = SubmittedPresentationFrame {
        gather: buffers.finish_uncompressed().unwrap(),
        root_position_w: vec![None],
    };
    let mut producer = SelectedPresentationFrameProducer {
        buffers,
        header,
        opaque_root_bytes: [Vec::new(), Vec::new()],
        matrix_rotations: vec![],
        rotations: vec![],
        translations: vec![],
    };
    let view = producer.read_frame(&frame, 0).unwrap();
    assert_eq!(view.compressed_offset, 208);
    assert_eq!(view.local_hips.map(f32::to_bits), hips.map(f32::to_bits));
    assert_eq!(
        view.read_base.as_ptr(),
        producer.buffers.backing_for(&frame.gather).unwrap()[16..].as_ptr()
    );
    assert_eq!(&view.read_base[208..216], &[0; 8]);
    assert!(producer.read_frame(&frame, 1).is_err());
    assert!(producer.root_position(&frame, 0).unwrap().w.is_none());
    for _ in 0..2 {
        producer.buffers.begin_submission().unwrap();
        producer
            .buffers
            .submit_data(&0_u32.to_be_bytes(), 4)
            .unwrap();
        producer.buffers.finish_uncompressed().unwrap();
    }
    assert!(matches!(
        producer.root_position(&frame, 0),
        Err(PresentationFrameError::Buffer(
            PresentationBufferError::StaleFrame
        ))
    ));
}

#[test]
fn root_w_carrier_never_changes_semantic_knowledge_and_known_bits_survive() {
    let unknown = crate::skeleton_root_transforms::SkeletonRootVector {
        xyz: [1., 2., 3.],
        w: None,
    };
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    assert_eq!(header.root_position(unknown), Err("known root position W"));
    header.root_w_policy = RootPositionWTransport::ReadSkaterOpaque { bits: 0x7fc0_1234 };
    assert_eq!(header.root_position(unknown).unwrap(), None);
    assert_eq!(header.word(156), 0x7fc0_1234);
    let mut known = unknown;
    known.w = Some(f32::from_bits(0x8000_0000));
    assert_eq!(header.root_position(known).unwrap(), Some(0x8000_0000));
    assert_eq!(header.word(156), 0x8000_0000);
}

#[test]
fn compressed_view_rejects_offsets_into_opaque_header_lanes() {
    let mut buffers =
        SelectedPresentationBuffers::new_uncompressed_host_policy(256, 320, 0).unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&1_u32.to_be_bytes(), 4).unwrap();
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    header.bits(188, 0xffff_0000, 156 << 16);
    buffers.submit_data(&header.bytes, 16).unwrap();
    let frame = SubmittedPresentationFrame {
        gather: buffers.finish_uncompressed().unwrap(),
        root_position_w: vec![None],
    };
    let producer = SelectedPresentationFrameProducer {
        buffers,
        header,
        opaque_root_bytes: [Vec::new(), Vec::new()],
        matrix_rotations: vec![],
        rotations: vec![],
        translations: vec![],
    };
    assert!(matches!(
        producer.read_frame(&frame, 0),
        Err(PresentationFrameError::HeaderBounds)
    ));
}

#[test]
fn semantic_header_frame_cannot_be_read_from_a_foreign_byte_owner() {
    let mut first = SelectedPresentationBuffers::new_uncompressed_host_policy(16, 32, 0).unwrap();
    let mut second = SelectedPresentationBuffers::new_uncompressed_host_policy(16, 32, 0).unwrap();
    first.begin_submission().unwrap();
    first.submit_data(&[1; 4], 4).unwrap();
    let first_frame = first.finish_uncompressed().unwrap();
    second.begin_submission().unwrap();
    second.submit_data(&[2; 4], 4).unwrap();
    second.finish_uncompressed().unwrap();
    assert!(matches!(
        second.backing_for(&first_frame),
        Err(PresentationBufferError::StaleFrame)
    ));
}

#[test]
fn retained_opaque_tail_cannot_feed_native_adjacent_halfword_read() {
    let mut buffers =
        SelectedPresentationBuffers::new_uncompressed_host_policy(512, 560, 0).unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&1_u32.to_be_bytes(), 4).unwrap();
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    header.bits(188, 0xffff_0000, 208 << 16);
    buffers.submit_data(&header.bytes, 16).unwrap();
    buffers.submit_data(&[0; 156], 2).unwrap();
    let frame = SubmittedPresentationFrame {
        gather: buffers.finish_uncompressed().unwrap(),
        root_position_w: vec![None],
    };
    let tail = frame.gather.payload.end;
    let mut opaque_root_bytes = [Vec::new(), Vec::new()];
    opaque_root_bytes[frame.gather.buffer_index].push(tail..tail + 4);
    let producer = SelectedPresentationFrameProducer {
        buffers,
        header,
        opaque_root_bytes,
        matrix_rotations: vec![],
        rotations: vec![],
        translations: vec![[0; 3]; 26],
    };
    assert!(matches!(
        producer.read_frame(&frame, 0),
        Err(PresentationFrameError::Missing(_))
    ));
}

#[test]
fn same_frame_root_overwrites_opaque_w_without_promoting_header_knowledge() {
    let mut buffers =
        SelectedPresentationBuffers::new_uncompressed_host_policy(256, 320, 0).unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&1_u32.to_be_bytes(), 4).unwrap();
    let mut header = PresentationHeaderScratch::new_host_zero_policy();
    header.bits(188, 0xffff_0000, 208 << 16);
    header.set_word(140, 1.0_f32.to_bits());
    header.root_w_policy = RootPositionWTransport::ReadSkaterOpaque { bits: 0x7fc0_1234 };
    header
        .root_position(crate::skeleton_root_transforms::SkeletonRootVector {
            xyz: [2., 3., 4.],
            w: None,
        })
        .unwrap();
    buffers.submit_data(&header.bytes, 16).unwrap();
    let frame = SubmittedPresentationFrame {
        gather: buffers.finish_uncompressed().unwrap(),
        root_position_w: vec![None],
    };
    let producer = SelectedPresentationFrameProducer {
        buffers,
        header,
        opaque_root_bytes: [vec![], vec![]],
        matrix_rotations: vec![],
        rotations: vec![],
        translations: vec![],
    };
    let root = producer.same_frame_root(&frame, 0).unwrap();
    assert_eq!(&root[12..], &[2., 3., 4., 1.]);
    assert!(root.iter().all(|v| v.is_finite()));
    assert!(producer.root_position(&frame, 0).unwrap().w.is_none());
}
