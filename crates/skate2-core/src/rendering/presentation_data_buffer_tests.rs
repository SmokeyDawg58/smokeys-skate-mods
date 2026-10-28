use super::*;
#[test]
fn reset_retains_tail_and_alignment_only_zeros_reached_gap() {
    let mut buffer = PresentationDataBuffer::new(32).unwrap();
    buffer.add(&[0x80; 16], 2).unwrap();
    buffer.reset_metadata();
    buffer.add(&[1, 2, 3], 1).unwrap();
    buffer.add(&[9, 10], 8).unwrap();
    assert_eq!(buffer.size(), 10);
    assert_eq!(buffer.alignment(), 8);
    assert_eq!(
        &buffer.backing()[..12],
        &[1, 2, 3, 0, 0, 0, 0, 0, 9, 10, 0x80, 0x80]
    );
}
#[test]
fn failed_payload_preserves_gap_and_alignment_publication() {
    let mut buffer = PresentationDataBuffer::new(10).unwrap();
    buffer.add(&[0xff; 10], 1).unwrap();
    buffer.reset_metadata();
    buffer.add(&[3; 3], 1).unwrap();
    assert!(matches!(
        buffer.add(&[4; 3], 8),
        Err(PresentationBufferError::Bounds { .. })
    ));
    assert_eq!(buffer.size(), 3);
    assert_eq!(buffer.alignment(), 8);
    assert_eq!(&buffer.backing()[3..8], &[0; 5]);
    assert_eq!(&buffer.backing()[8..10], &[0xff; 2]);
}
#[test]
fn retained_gather_tail_is_real_and_recycled_views_are_rejected() {
    let mut buffers = SelectedPresentationBuffers::new_uncompressed_host_policy(64, 96, 3).unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&[0x80; 32], 16).unwrap();
    let first = buffers.finish_uncompressed().unwrap();
    buffers.begin_submission().unwrap();
    buffers.submit_data(&[1; 4], 4).unwrap();
    let second = buffers.finish_uncompressed().unwrap();
    assert_eq!(&buffers.backing_for(&first).unwrap()[..32], &[0x80; 32]);
    buffers.begin_submission().unwrap();
    buffers.submit_data(&[2; 4], 16).unwrap();
    let third = buffers.finish_uncompressed().unwrap();
    assert!(matches!(
        buffers.backing_for(&first),
        Err(PresentationBufferError::StaleFrame)
    ));
    assert_eq!(&buffers.backing_for(&second).unwrap()[..4], &[1; 4]);
    assert_eq!(
        &buffers.backing_for(&third).unwrap()[..6],
        &[2, 2, 2, 2, 0x80, 0x80]
    );
    let packed = u32::from_be_bytes(
        buffers.gathers[third.buffer_index].backing()[..4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(packed, (4 << 13) | (3 << 8) | (12 << 1));
}
