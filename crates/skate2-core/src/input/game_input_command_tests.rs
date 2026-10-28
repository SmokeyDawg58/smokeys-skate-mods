use super::*;
use crate::input_pad_cache::PublishedInputPadSample;
fn data(focus: bool) -> PlayerInputData {
    PlayerInputData {
        sample: PublishedInputPadSample {
            count: 19,
            values: [0.0; 24],
        },
        allow_sensor: true,
        has_focus: focus,
    }
}
#[test]
fn empty_focus_and_focused_zero_have_distinct_wire_and_decode() {
    let mut bytes = [0xaa; 9];
    let mut stream = GameInputWrite::new(&mut bytes);
    try_pack_player_input(&data(false), &mut stream, &mut HostReckoningNumeric).unwrap();
    assert_eq!(stream.position(), 0);
    try_pack_player_input(&data(true), &mut stream, &mut HostReckoningNumeric).unwrap();
    assert_eq!(stream.position(), 1);
    assert!(!stream.has_error());
    assert_eq!(bytes[0], 0);
    let mut output = data(true);
    output.sample.values.fill(7.0);
    try_unpack_player_input(&mut output, &mut GameInputRead::new(&[])).unwrap();
    assert!(!output.has_focus);
    assert_eq!(output.sample.values[13].to_bits(), 1.0_f32.to_bits());
    assert_eq!(output.sample.values[12].to_bits(), 0);
    assert_eq!(output.sample.count, 19);
    assert!(output.allow_sensor);
    try_unpack_player_input(&mut output, &mut GameInputRead::new(&[0])).unwrap();
    assert!(output.has_focus);
    assert_eq!(output.sample.values.map(f32::to_bits), [0; 24]);
}
#[test]
fn full_mask_field_order_and_truncating_quantization() {
    let mut input = data(true);
    input.sample.values[..16].fill(1.0);
    input.sample.values[16] = 0.5;
    input.sample.values[19] = 0.5;
    input.sample.values[20] = 1.0;
    input.sample.values[23] = 1.0;
    input.sample.values[10] = 0.5;
    input.sample.values[11] = 0.25;
    let mut bytes = [0; 9];
    let mut write = GameInputWrite::new(&mut bytes);
    try_pack_player_input(&input, &mut write, &mut HostReckoningNumeric).unwrap();
    assert_eq!(write.position(), 9);
    assert_eq!(bytes, [255, 63, 193, 127, 129, 127, 63, 255, 63]);
    let mut output = data(false);
    let mut read = GameInputRead::new(&bytes);
    try_unpack_player_input(&mut output, &mut read).unwrap();
    assert_eq!(read.position(), 9);
    assert!(!read.has_error());
    assert_eq!(
        output.sample.values[16].to_bits(),
        (63.0 * f32::from_bits(0x3c01_0204)).to_bits()
    );
    assert_eq!(
        output.sample.values[19].to_bits(),
        output.sample.values[16].to_bits()
    );
    assert_ne!(output.sample.values[16].to_bits(), 0.5_f32.to_bits());
}
#[test]
fn digital_nan_and_signed_zero_are_false_and_unused_high_bits_are_ignored() {
    let mut input = data(true);
    input.sample.values[..16].fill(f32::NAN);
    input.sample.values[10] = 0.0;
    input.sample.values[11] = -0.0;
    let mut bytes = [0xff; 9];
    let mut write = GameInputWrite::new(&mut bytes);
    try_pack_player_input(&input, &mut write, &mut HostReckoningNumeric).unwrap();
    assert_eq!(write.position(), 1);
    assert_eq!(bytes[0], 0);
    let mut output = data(false);
    try_unpack_player_input(&mut output, &mut GameInputRead::new(&[128, 192, 77])).unwrap();
    assert_eq!(output.sample.values.map(f32::to_bits), [0; 24]);
}
#[test]
fn decode_negative128_does_not_clamp_and_zero_halves_are_positive() {
    let mut output = data(false);
    try_unpack_player_input(&mut output, &mut GameInputRead::new(&[1, 128])).unwrap();
    assert_eq!(output.sample.values[16].to_bits(), 0);
    assert_eq!(
        output.sample.values[17].to_bits(),
        (-128.0 * f32::from_bits(0xbc01_0204)).to_bits()
    );
    assert!(output.sample.values[17] > 1.0);
    assert_eq!(output.sample.values[18].to_bits(), 0);
    assert_eq!(output.sample.values[19].to_bits(), 0);
}
#[test]
fn short_read_commits_focus_and_consumed_bytes_but_not_channel_values() {
    let mut output = data(false);
    output.sample.values.fill(9.0);
    let mut read = GameInputRead::new(&[3, 17]);
    assert_eq!(
        try_unpack_player_input(&mut output, &mut read),
        Err(GameInputStreamError {
            position: 2,
            capacity: 2
        })
    );
    assert_eq!(read.position(), 2);
    assert!(read.has_error());
    assert!(output.has_focus);
    assert_eq!(
        output.sample.values.map(f32::to_bits),
        [9.0_f32.to_bits(); 24]
    );
    assert_eq!(output.sample.count, 19);
}
#[test]
fn short_write_keeps_mask_and_first_field() {
    let mut input = data(true);
    input.sample.values[16] = 1.0;
    input.sample.values[18] = 1.0;
    let mut bytes = [0xaa; 2];
    let mut write = GameInputWrite::new(&mut bytes);
    assert!(matches!(
        try_pack_player_input(&input, &mut write, &mut HostReckoningNumeric),
        Err(GameInputPackError::Stream(GameInputStreamError {
            position: 2,
            capacity: 2
        }))
    ));
    assert_eq!(write.position(), 2);
    assert!(write.has_error());
    assert_eq!(bytes, [3, 127]);
}
struct Numeric {
    calls: Vec<(&'static str, u32)>,
    fail: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("not a codec operation")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("not a codec operation")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("not a codec operation")
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.calls.push(("i32", value.to_bits()));
        if self.calls.len() == self.fail {
            Err("convert")
        } else {
            Ok(0)
        }
    }
}
impl GameInputCommandNumeric for Numeric {
    fn truncate_i64(&mut self, value: f32) -> Result<i64, Self::Error> {
        self.calls.push(("i64", value.to_bits()));
        if self.calls.len() == self.fail {
            Err("convert")
        } else {
            Ok(0)
        }
    }
}
#[test]
fn conversion_order_and_failure_precede_any_output() {
    let mut input = data(true);
    input.sample.values[10] = 1.0;
    input.sample.values[16] = 1.0;
    input.sample.values[11] = 0.5;
    let mut numeric = Numeric {
        calls: vec![],
        fail: 3,
    };
    let mut bytes = [0xaa; 9];
    let mut write = GameInputWrite::new(&mut bytes);
    assert!(matches!(
        try_pack_player_input(&input, &mut write, &mut numeric),
        Err(GameInputPackError::Numeric("convert"))
    ));
    assert_eq!(
        numeric.calls,
        [
            ("i64", 255.0_f32.to_bits()),
            ("i32", 127.0_f32.to_bits()),
            ("i64", 127.5_f32.to_bits())
        ]
    );
    assert_eq!(write.position(), 0);
    assert!(!write.has_error());
    assert_eq!(bytes, [0xaa; 9]);
}
#[test]
fn checked_host_i64_refuses_exceptional_conversion_and_preserves_low_byte() {
    for value in [
        f32::NAN,
        f32::INFINITY,
        f32::from_bits(0x5f00_0000),
        f32::NEG_INFINITY,
    ] {
        assert!(HostReckoningNumeric.truncate_i64(value).is_err());
    }
    assert_eq!(
        HostReckoningNumeric
            .truncate_i64(f32::from_bits(0xdf00_0000))
            .unwrap(),
        i64::MIN
    );
    assert_eq!(HostReckoningNumeric.truncate_i64(-1.75).unwrap(), -1);
    let mut input = data(true);
    input.sample.values[10] = 2.0;
    let mut bytes = [0; 9];
    let mut write = GameInputWrite::new(&mut bytes);
    try_pack_player_input(&input, &mut write, &mut HostReckoningNumeric).unwrap();
    assert_eq!(write.position(), 2);
    assert_eq!(bytes[..2], [16, 254]);
}
