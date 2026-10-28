//! `cIniFile` Read82a88de0 and custom `rw::core::stdc` conversions82bc3ce0/82bc3e68.
//! Byte names are case-sensitive. This selected reader exposes only Config keys.
use super::InputCvarError;
use std::collections::BTreeMap;

fn c_string(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}
fn whitespace(byte: u8) -> bool {
    matches!(byte, 9..=13 | 32 | 160)
}
/// Whole-line trim only: spaces around '=' remain part of the key/value.
pub(super) fn parse(bytes: &[u8]) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, InputCvarError> {
    let mut values = BTreeMap::new();
    let mut selected = false;
    for line in bytes.split(|byte| *byte == b'\n') {
        // ReadLine returns NULL when the first consumed byte is LF.
        if line.is_empty() {
            break;
        }
        let line = c_string(line);
        let start = line
            .iter()
            .position(|byte| !whitespace(*byte))
            .unwrap_or(line.len());
        let end = line
            .iter()
            .rposition(|byte| !whitespace(*byte))
            .map_or(start, |index| index + 1);
        let line = &line[start..end];
        if line.first() == Some(&b';') {
            continue;
        }
        if line.len() >= 3 && line.first() == Some(&b'[') && line.last() == Some(&b']') {
            let section = &line[1..line.len() - 1];
            if section.len() >= 256 {
                return Err(InputCvarError::IniFieldTooLong);
            }
            selected = section == b"Config";
        } else if selected && line.len() >= 3 {
            if let Some(split) = line.iter().position(|byte| *byte == b'=') {
                if split == 0 {
                    continue;
                }
                let key = &line[..split];
                let value = &line[split + 1..];
                if key.len() >= 256 || value.len() >= 256 {
                    return Err(InputCvarError::IniFieldTooLong);
                }
                values.entry(key.to_vec()).or_insert_with(|| value.to_vec());
            }
        }
    }
    Ok(values)
}
/// `ConvertAToI` scans arbitrary prefixes for the first minus or decimal digit.
pub(super) fn integer(bytes: &[u8]) -> i32 {
    let bytes = c_string(bytes);
    let start = bytes
        .iter()
        .position(|byte| *byte == b'-' || byte.is_ascii_digit())
        .unwrap_or(bytes.len());
    let mut digits = &bytes[start..];
    let negative = digits.first() == Some(&b'-');
    if negative {
        digits = &digits[1..];
    }
    let mut value = 0i32;
    for byte in digits.iter().take_while(|byte| byte.is_ascii_digit()) {
        value = value.wrapping_mul(10).wrapping_add(i32::from(*byte - b'0'));
    }
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}
/// `ConvertAToF` raw fmadds/fdivs, with source exponent chunks of four.
pub(super) fn float(bytes: &[u8]) -> f32 {
    let bytes = c_string(bytes);
    let mut index = bytes
        .iter()
        .position(|byte| matches!(*byte, b'-' | b'.') || byte.is_ascii_digit())
        .unwrap_or(bytes.len());
    let mut negative = false;
    let mut fractional = false;
    let mut numerator = 0.0f32;
    let mut divisor = 1.0f32;
    while let Some(byte) = bytes.get(index).copied() {
        match byte {
            b'-' => negative = true,
            b'.' => fractional = true,
            b'0'..=b'9' => {
                numerator = numerator.mul_add(10.0, f32::from(byte - b'0'));
                if fractional {
                    divisor *= 10.0;
                }
            }
            _ => break,
        }
        index += 1;
    }
    let mut value = numerator / divisor;
    if negative {
        value = -value;
    }
    if !matches!(bytes.get(index), Some(b'e' | b'E')) {
        return value;
    }
    index += 1;
    let mut exponent = 0i32;
    let mut negative_exponent = false;
    while let Some(byte) = bytes.get(index).copied() {
        match byte {
            b'-' => negative_exponent = true,
            b'0'..=b'9' => {
                exponent = exponent
                    .wrapping_mul(10)
                    .wrapping_add(i32::from(byte - b'0'));
            }
            _ => break,
        }
        index += 1;
    }
    if negative_exponent {
        exponent = exponent.wrapping_neg();
    }
    let magnitude = exponent.unsigned_abs();
    let (group, single) = if exponent < 0 {
        (f32::from_bits(0x38d1_b717), f32::from_bits(0x3dcc_cccd))
    } else {
        (10_000.0, 10.0)
    };
    value = repeat_multiply(value, group, magnitude / 4);
    repeat_multiply(value, single, magnitude % 4)
}
fn repeat_multiply(mut value: f32, factor: f32, count: u32) -> f32 {
    for _ in 0..count {
        let next = value * factor;
        // Once IEEE multiplication is bitwise stationary, remaining source
        // iterations cannot change the value (also bounds giant wrapped exponents).
        if next.to_bits() == value.to_bits() {
            break;
        }
        value = next;
    }
    value
}

/// `GetVector282a89790` counts nonempty comma-separated tokens, converting the
/// original tail at each start (the token length is deliberately not passed).
pub(super) fn vector2(bytes: &[u8]) -> Option<[f32; 2]> {
    let bytes = c_string(bytes);
    let mut starts = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        while bytes.get(index) == Some(&b',') {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }
        starts.push(index);
        while index < bytes.len() && bytes[index] != b',' {
            index += 1;
        }
    }
    if starts.len() < 2 {
        return None;
    }
    Some([float(&bytes[starts[0]..]), float(&bytes[starts[1]..])])
}
