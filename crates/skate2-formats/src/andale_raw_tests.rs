#![expect(
    clippy::float_cmp,
    reason = "exact copied float lanes and identity channels"
)]
use super::*;
use crate::andale::AndaleRawCompressionHeader;

fn two_frames() -> AndaleClipPart {
    let mut bytes = Vec::new();
    for frame in [1.0_f32, 9.0] {
        for scale in [frame, 2.0, 3.0] {
            bytes.extend_from_slice(&scale.to_be_bytes());
        }
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        for translation in [4.0_f32, 5.0, frame] {
            bytes.extend_from_slice(&translation.to_be_bytes());
        }
    }
    AndaleClipPart {
        header: 1 << 6,
        encoded_offset: 0,
        part_name_index: 0,
        compressed_data_size: 56,
        compression_header_offset: 16,
        compressed_data_offset: 48,
        compression_header_size: 32,
        trailing_bytes_size: 0,
        compression: AndaleCompressionHeader::RawDiscretize(AndaleRawCompressionHeader {
            channel_masks: [1, 0, 1, 0, 2, 0],
            frame_stride: 28,
            last_bone_index: 1,
            format_flags: [0, 1],
            padding: [0xcd; 3],
        }),
        compression_header: vec![],
        compressed_data: bytes,
        trailing_data: vec![],
        vbr_prelude: None,
    }
}

#[test]
fn selects_second_stride_and_preserves_unselected_channels() {
    let part = two_frames();
    let first = part.sample_raw_frame_sqt(0).unwrap();
    let second = part.sample_raw_frame_sqt(1).unwrap();
    assert_eq!(first[0].scale, [1.0, 2.0, 3.0, 1.0]);
    assert_eq!(second[0].scale, [9.0, 2.0, 3.0, 1.0]);
    assert_eq!(second[1].translation, [4.0, 5.0, 9.0, 1.0]);
    assert_eq!(second[0].translation, identity_sqt().translation);
    assert_eq!(second[1].rotation, identity_sqt().rotation);
    assert!(matches!(
        part.sample_raw_frame_sqt(2),
        Err(AndaleRawError::Frame { .. })
    ));
}

#[test]
fn rejects_unknown_selectors_and_truncated_later_frame() {
    let mut part = two_frames();
    part.compressed_data.pop();
    assert!(matches!(
        part.sample_raw_frame_sqt(0),
        Err(AndaleRawError::PayloadFrames { .. })
    ));
    let AndaleCompressionHeader::RawDiscretize(header) = &mut part.compression else {
        unreachable!()
    };
    header.channel_masks[1] = 1;
    assert_eq!(
        part.sample_raw_frame_sqt(0),
        Err(AndaleRawError::UnsupportedSelectors)
    );
}

#[test]
fn vehicle_bind_pose_preserves_scale_and_interleaved_wheel_channels() {
    use crate::andale::AndaleAnimationPart;
    // All four mask families present in Jan17 VehiclesDB. Synthetic values
    // distinguish a skipped root scale from misaligned wheel translations.
    for (scale, rotation) in [(1, 0_u32), (0, 0), (1, 12), (1, 220)] {
        let mut bytes = Vec::new();
        for bone in 0..8 {
            if scale & (1 << bone) != 0 {
                for value in [0.01_f32, 0.02, 0.03] {
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
            }
            if rotation & (1 << bone) != 0 {
                bytes.extend_from_slice(&0_u32.to_be_bytes());
            }
            if bone >= 2 {
                for value in [bone as f32, -(bone as f32), 10.0] {
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
            }
        }
        let mut part = AndaleAnimationPart {
            header: 7 << 6,
            encoded_offset: 0,
            part_name_index: 0,
            compressed_data_size: bytes.len() as u32,
            compression_header_offset: 16,
            compressed_data_offset: 48,
            compression: AndaleCompressionHeader::RawDiscretize(AndaleRawCompressionHeader {
                channel_masks: [scale, 0, rotation, 0, 252, 0],
                frame_stride: bytes.len() as u16,
                last_bone_index: 7,
                format_flags: [0, 1],
                padding: [0; 3],
            }),
            compression_header: vec![],
            compressed_data: bytes,
            padding: vec![],
            vbr_prelude: None,
        };
        let pose = part.sample_raw_pose_sqt().unwrap();
        assert_eq!(pose.len(), 8);
        assert_eq!(
            pose[0].scale,
            if scale == 1 {
                [0.01, 0.02, 0.03, 1.0]
            } else {
                [1.0; 4]
            }
        );
        for (bone, sqt) in pose.iter().enumerate() {
            assert_eq!(sqt.rotation, identity_sqt().rotation);
            if bone >= 2 {
                assert_eq!(sqt.translation, [bone as f32, -(bone as f32), 10.0, 1.0]);
            } else {
                assert_eq!(sqt.translation, identity_sqt().translation);
            }
            if bone != 0 {
                assert_eq!(sqt.scale, [1.0; 4]);
            }
        }
        part.compressed_data.pop();
        assert!(matches!(
            part.sample_raw_pose_sqt(),
            Err(AndaleError::RawPoseDataSizeMismatch { .. })
        ));
    }
}

#[test]
fn empty_cas_pose_part_keeps_identity_with_unused_unpacked_selectors() {
    use crate::andale::AndaleAnimationPart;
    let mut part = AndaleAnimationPart {
        header: 7 << 6,
        encoded_offset: 0,
        part_name_index: 0,
        compressed_data_size: 0,
        compression_header_offset: 16,
        compressed_data_offset: 48,
        compression: AndaleCompressionHeader::RawDiscretize(AndaleRawCompressionHeader {
            channel_masks: [0; 6], frame_stride: 0, last_bone_index: 7,
            format_flags: [0, 0], padding: [0; 3],
        }),
        compression_header: vec![], compressed_data: vec![], padding: vec![],
        vbr_prelude: None,
    };
    assert_eq!(part.sample_raw_pose_sqt().unwrap(), vec![identity_sqt(); 8]);
    part.compressed_data.push(0);
    assert!(matches!(part.sample_raw_pose_sqt(), Err(AndaleError::RawPoseDataSizeMismatch { .. })));
    part.compressed_data.clear();
    let AndaleCompressionHeader::RawDiscretize(header) = &mut part.compression else { unreachable!() };
    header.channel_masks[2] = 1;
    assert!(matches!(part.sample_raw_pose_sqt(), Err(AndaleError::RawPoseDataSizeMismatch { .. })));
}

#[test]
fn unpacked_cas_rotations_preserve_xyzw_and_frame_stride() {
    let mut part = two_frames();
    part.compressed_data.clear();
    for frame in [1.0_f32, 9.0] {
        // Scale, unnormalized XYZW, translation on the second bone.
        for value in [frame, 2.0, 3.0, 0.25, -0.5, 0.75, frame, 4.0, 5.0, frame] {
            part.compressed_data.extend_from_slice(&value.to_be_bytes());
        }
    }
    let AndaleCompressionHeader::RawDiscretize(header) = &mut part.compression else { unreachable!() };
    header.format_flags = [0, 0];
    header.frame_stride = 40;
    let pose = part.sample_raw_frame_sqt(1).unwrap();
    assert_eq!(pose[0].scale, [9.0, 2.0, 3.0, 1.0]);
    assert_eq!(pose[0].rotation, [0.25, -0.5, 0.75, 9.0]);
    assert_eq!(pose[1].translation, [4.0, 5.0, 9.0, 1.0]);
    let AndaleCompressionHeader::RawDiscretize(header) = &mut part.compression else { unreachable!() };
    header.channel_masks = [0; 6];
    header.frame_stride = 0;
    part.compressed_data.clear();
    assert_eq!(part.sample_raw_frame_sqt(123).unwrap(), vec![identity_sqt(); 2]);
}
