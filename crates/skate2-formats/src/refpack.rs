//! `RefPack` buffer decompression reconstructed from the reference implementation.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefPackError {
    TruncatedHeader,
    InvalidSignature(u16),
    TruncatedInput,
    InvalidBackReference { distance: usize, output_len: usize },
    OutputTooLarge { declared: usize, actual: usize },
    SizeMismatch { declared: usize, actual: usize },
}

impl fmt::Display for RefPackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid RefPack stream: {self:?}")
    }
}

impl std::error::Error for RefPackError {}

/// Reads the decompressed-size field from a `RefPack` buffer.
///
/// # Errors
///
/// Returns [`RefPackError`] when the signature or variable-length header is
/// invalid.
pub fn decompressed_size(input: &[u8]) -> Result<usize, RefPackError> {
    let (size, _) = parse_header(input)?;
    Ok(size)
}

/// Decompresses one complete `RefPack` buffer.
///
/// Reconstructed from `rw::core::codec::RefpackBuf::Decompress` at
/// `0x82bc9140`.
///
/// # Errors
///
/// Returns [`RefPackError`] for malformed headers, truncated commands, invalid
/// backward copies, or output that disagrees with the declared size.
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, RefPackError> {
    let (declared_size, mut cursor) = parse_header(input)?;
    let mut output = Vec::with_capacity(declared_size);

    loop {
        let control = take_byte(input, &mut cursor)?;
        match control {
            0x00..=0x7f => {
                let offset_low = take_byte(input, &mut cursor)?;
                copy_literals(
                    input,
                    &mut cursor,
                    &mut output,
                    usize::from(control & 3),
                    declared_size,
                )?;
                let length = usize::from((control >> 2) & 7) + 3;
                let distance =
                    usize::from((u16::from(control) << 3) & 0x300) + usize::from(offset_low) + 1;
                copy_back_reference(&mut output, distance, length, declared_size)?;
            }
            0x80..=0xbf => {
                let second = take_byte(input, &mut cursor)?;
                let third = take_byte(input, &mut cursor)?;
                copy_literals(
                    input,
                    &mut cursor,
                    &mut output,
                    usize::from(second >> 6),
                    declared_size,
                )?;
                let length = usize::from(control & 0x3f) + 4;
                let distance = usize::from(second & 0x3f) * 256 + usize::from(third) + 1;
                copy_back_reference(&mut output, distance, length, declared_size)?;
            }
            0xc0..=0xdf => {
                let second = take_byte(input, &mut cursor)?;
                let third = take_byte(input, &mut cursor)?;
                let fourth = take_byte(input, &mut cursor)?;
                copy_literals(
                    input,
                    &mut cursor,
                    &mut output,
                    usize::from(control & 3),
                    declared_size,
                )?;
                let length =
                    usize::from((u16::from(control) << 6) & 0x300) + usize::from(fourth) + 5;
                let distance = usize::from(control & 0x10) * 4096
                    + usize::from(second) * 256
                    + usize::from(third)
                    + 1;
                copy_back_reference(&mut output, distance, length, declared_size)?;
            }
            0xe0..=0xfb => {
                let length = 4 * (usize::from(control & 0x1f) + 1);
                copy_literals(input, &mut cursor, &mut output, length, declared_size)?;
            }
            0xfc..=0xff => {
                copy_literals(
                    input,
                    &mut cursor,
                    &mut output,
                    usize::from(control & 3),
                    declared_size,
                )?;
                break;
            }
        }
    }

    if output.len() != declared_size {
        return Err(RefPackError::SizeMismatch {
            declared: declared_size,
            actual: output.len(),
        });
    }
    Ok(output)
}

fn parse_header(input: &[u8]) -> Result<(usize, usize), RefPackError> {
    let prefix = input.get(..2).ok_or(RefPackError::TruncatedHeader)?;
    let flags = u16::from_be_bytes([prefix[0], prefix[1]]);
    if flags & 0x3eff != 0x10fb {
        return Err(RefPackError::InvalidSignature(flags));
    }

    let wide_size = flags & 0x8000 != 0;
    let includes_compressed_size = flags & 0x0100 != 0;
    let size_width = if wide_size { 4 } else { 3 };
    let mut cursor = 2;
    if includes_compressed_size {
        cursor += size_width;
    }
    let end = cursor + size_width;
    let field = input
        .get(cursor..end)
        .ok_or(RefPackError::TruncatedHeader)?;
    let size = field
        .iter()
        .fold(0_usize, |value, byte| (value << 8) | usize::from(*byte));
    Ok((size, end))
}

fn take_byte(input: &[u8], cursor: &mut usize) -> Result<u8, RefPackError> {
    let byte = input
        .get(*cursor)
        .copied()
        .ok_or(RefPackError::TruncatedInput)?;
    *cursor += 1;
    Ok(byte)
}

fn copy_literals(
    input: &[u8],
    cursor: &mut usize,
    output: &mut Vec<u8>,
    length: usize,
    declared_size: usize,
) -> Result<(), RefPackError> {
    let end = cursor
        .checked_add(length)
        .ok_or(RefPackError::TruncatedInput)?;
    let literals = input
        .get(*cursor..end)
        .ok_or(RefPackError::TruncatedInput)?;
    ensure_output_capacity(output.len(), length, declared_size)?;
    output.extend_from_slice(literals);
    *cursor = end;
    Ok(())
}

fn copy_back_reference(
    output: &mut Vec<u8>,
    distance: usize,
    length: usize,
    declared_size: usize,
) -> Result<(), RefPackError> {
    if distance == 0 || distance > output.len() {
        return Err(RefPackError::InvalidBackReference {
            distance,
            output_len: output.len(),
        });
    }
    ensure_output_capacity(output.len(), length, declared_size)?;
    for _ in 0..length {
        let source = output.len() - distance;
        output.push(output[source]);
    }
    Ok(())
}

fn ensure_output_capacity(
    current: usize,
    added: usize,
    declared: usize,
) -> Result<(), RefPackError> {
    let actual = current
        .checked_add(added)
        .ok_or(RefPackError::OutputTooLarge {
            declared,
            actual: usize::MAX,
        })?;
    if actual > declared {
        return Err(RefPackError::OutputTooLarge { declared, actual });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_literal_runs_and_terminal_literals() {
        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        assert_eq!(decompress(stream).unwrap(), b"abcdef");
    }

    #[test]
    fn supports_overlapping_short_back_references() {
        let stream = b"\x10\xfb\x00\x00\x06\x03\x02abc\xfc";
        assert_eq!(decompress(stream).unwrap(), b"abcabc");
    }

    #[test]
    fn rejects_a_back_reference_before_output() {
        let stream = b"\x10\xfb\x00\x00\x03\x00\x00";
        assert!(matches!(
            decompress(stream),
            Err(RefPackError::InvalidBackReference { .. })
        ));
    }
}
