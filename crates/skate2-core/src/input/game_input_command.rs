//! Sparse `PlayerInputData` codec82a8c070/82a8c330. Typed failure stops at native
//! stream assertions; prior stream/target writes remain visible.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected gameplay command services await offline scheduling and Actor wiring"
    )
)]
use crate::{
    game_input_player::PlayerInputData,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
};
use std::fmt;

pub(crate) trait GameInputCommandNumeric: ReckoningNumeric {
    fn truncate_i64(&mut self, value: f32) -> Result<i64, Self::Error>;
}
impl GameInputCommandNumeric for HostReckoningNumeric {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite signed64 range is checked before native truncation"
    )]
    fn truncate_i64(&mut self, value: f32) -> Result<i64, Self::Error> {
        // First out-of-range positive value is2^63; -2^63 remains valid.
        if !value.is_finite()
            || value < f32::from_bits(0xdf00_0000)
            || value >= f32::from_bits(0x5f00_0000)
        {
            return Err(
                crate::reckoning_numeric::ReckoningHostNumericError::IntegerConversion {
                    bits: value.to_bits(),
                },
            );
        }
        Ok(value as i64)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GameInputStreamError {
    pub(crate) position: usize,
    pub(crate) capacity: usize,
}
impl fmt::Display for GameInputStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "input byte {} exceeds stream capacity {}",
            self.position, self.capacity
        )
    }
}
#[derive(Debug)]
pub(crate) enum GameInputPackError<E> {
    Numeric(E),
    Stream(GameInputStreamError),
}
impl<E: fmt::Debug> fmt::Display for GameInputPackError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "input packing: {self:?}")
    }
}
/// Selected attached stream, initially position0/errorfalse. No byte is read
/// implicitly; an empty payload is valid at the outer `Unpack` branch.
pub(crate) struct GameInputRead<'a> {
    bytes: &'a [u8],
    position: usize,
    error: bool,
}
impl<'a> GameInputRead<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            error: false,
        }
    }
    pub(crate) const fn position(&self) -> usize {
        self.position
    }
    pub(crate) const fn has_error(&self) -> bool {
        self.error
    }
    fn byte(&mut self) -> Result<u8, GameInputStreamError> {
        let Some(value) = self.bytes.get(self.position).copied() else {
            self.error = true;
            return Err(GameInputStreamError {
                position: self.position,
                capacity: self.bytes.len(),
            });
        };
        self.position += 1;
        Ok(value)
    }
}
/// Each successful source `WriteBytes` advances separately; capacity failure
/// retains the already written prefix and does not advance the failed byte.
pub(crate) struct GameInputWrite<'a> {
    bytes: &'a mut [u8],
    position: usize,
    error: bool,
}
impl<'a> GameInputWrite<'a> {
    pub(crate) const fn new(bytes: &'a mut [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            error: false,
        }
    }
    pub(crate) const fn position(&self) -> usize {
        self.position
    }
    pub(crate) const fn has_error(&self) -> bool {
        self.error
    }
    fn byte(&mut self, value: u8) -> Result<(), GameInputStreamError> {
        let capacity = self.bytes.len();
        let Some(destination) = self.bytes.get_mut(self.position) else {
            self.error = true;
            return Err(GameInputStreamError {
                position: self.position,
                capacity,
            });
        };
        *destination = value;
        self.position += 1;
        Ok(())
    }
}
/// Pack82a8c070: focused input quantizes before sparse mask/field writes.
/// Unfocused input writes nothing, without numeric calls. Count/`allowSensor`
/// are not serialized by this native codec.
pub(crate) fn try_pack_player_input<N: GameInputCommandNumeric>(
    input: &PlayerInputData,
    stream: &mut GameInputWrite<'_>,
    numeric: &mut N,
) -> Result<(), GameInputPackError<N::Error>> {
    if !input.has_focus {
        return Ok(());
    }
    let values = &input.sample.values;
    let mut fields = [0_u8; 8];
    for (bit, channel) in [12, 13, 14, 15, 6, 7, 8, 9].into_iter().enumerate() {
        if values[channel] > 0.0 {
            fields[6] |= 1 << bit;
        }
    }
    for (bit, value) in values[..6].iter().enumerate() {
        if *value > 0.0 {
            fields[7] |= 1 << bit;
        }
    }
    let lx = values[16] - values[17];
    let ly = values[18] - values[19];
    let rx = values[20] - values[21];
    let ry = values[22] - values[23];
    let lt = values[10] * 255.0;
    let lx = lx * 127.0;
    let rt = values[11] * 255.0;
    let ly = ly * 127.0;
    let rx = rx * 127.0;
    let ry = ry * 127.0;
    // Native fctidz/fctiwz interleaving; low byte extraction is not saturation.
    let lt = numeric
        .truncate_i64(lt)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    let lx = numeric
        .truncate_i32(lx)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    let rt = numeric
        .truncate_i64(rt)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    let ly = numeric
        .truncate_i32(ly)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    let rx = numeric
        .truncate_i32(rx)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    let ry = numeric
        .truncate_i32(ry)
        .map_err(GameInputPackError::Numeric)?
        .to_le_bytes()[0];
    fields[..6].copy_from_slice(&[lx, ly, rx, ry, lt, rt]);
    let mut mask = 0_u8;
    for (bit, value) in fields.iter().enumerate() {
        if *value != 0 {
            mask |= 1 << bit;
        }
    }
    stream.byte(mask).map_err(GameInputPackError::Stream)?;
    for (bit, value) in fields.into_iter().enumerate() {
        if mask & (1 << bit) != 0 {
            stream.byte(value).map_err(GameInputPackError::Stream)?;
        }
    }
    Ok(())
}
/// `Unpack`82a8c330: focus commits before any read; channels commit only after
/// the local compressed record is read. Count and `allowSensor` remain untouched.
pub(crate) fn try_unpack_player_input(
    output: &mut PlayerInputData,
    stream: &mut GameInputRead<'_>,
) -> Result<(), GameInputStreamError> {
    output.has_focus = !stream.bytes.is_empty();
    if !output.has_focus {
        output.sample.values.fill(0.0);
        output.sample.values[13] = 1.0;
        return Ok(());
    }
    let mut fields = [0_u8; 8];
    let mask = stream.byte()?;
    for (bit, value) in fields.iter_mut().enumerate() {
        if mask & (1 << bit) != 0 {
            *value = stream.byte()?;
        }
    }
    for (bit, channel) in [12, 13, 14, 15, 6, 7, 8, 9].into_iter().enumerate() {
        output.sample.values[channel] = if fields[6] & (1 << bit) != 0 {
            1.0
        } else {
            0.0
        };
    }
    for bit in 0..6 {
        output.sample.values[bit] = if fields[7] & (1 << bit) != 0 {
            1.0
        } else {
            0.0
        };
    }
    output.sample.values[10] = f32::from(fields[4]) * f32::from_bits(0x3b80_8081);
    output.sample.values[11] = f32::from(fields[5]) * f32::from_bits(0x3b80_8081);
    for (axis, byte) in fields[..4].iter().enumerate() {
        let value = i8::from_ne_bytes([*byte]);
        output.sample.values[16 + axis * 2] = if value > 0 {
            f32::from(value) * f32::from_bits(0x3c01_0204)
        } else {
            0.0
        };
        output.sample.values[17 + axis * 2] = if value < 0 {
            f32::from(value) * f32::from_bits(0xbc01_0204)
        } else {
            0.0
        };
    }
    Ok(())
}
#[cfg(test)]
#[path = "game_input_command_tests.rs"]
mod tests;
