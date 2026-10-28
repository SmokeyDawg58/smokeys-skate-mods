//! `IIniReader::ReadIniLine82a75708` and `SetJuiceVars82a74ce8`.
use super::{JuiceConfig, JuiceIniError};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Seeking,
    Section,
    Key,
    Value,
    Comment,
    Done,
}

fn push(bytes: &mut Vec<u8>, byte: u8, capacity: usize) -> Result<(), JuiceIniError> {
    if bytes.len() + 1 >= capacity {
        return Err(JuiceIniError::FieldCapacity(capacity));
    }
    bytes.push(byte);
    Ok(())
}
fn c_string(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&v| v == 0).unwrap_or(bytes.len())]
}
/// Unlike the cvar reader, whitespace within section/key names is skipped,
/// duplicate values apply in order, and // comments end a value even in quotes.
pub(super) fn apply(config: &mut JuiceConfig, source: &[u8]) -> Result<(), JuiceIniError> {
    let mut bytes = source.to_vec();
    if bytes.starts_with(&[239, 187, 191]) {
        bytes[..3].fill(13);
    }
    let mut offset = 0;
    let mut section = Vec::new();
    while offset < bytes.len() {
        let mut key = Vec::new();
        let mut value = Vec::new();
        let mut field = Field::Seeking;
        let mut quoted = false;
        while offset < bytes.len() && field != Field::Done {
            let byte = bytes[offset];
            match byte {
                b' ' | b'\t' => {
                    if field == Field::Value && (!value.is_empty() || quoted) {
                        push(&mut value, byte, 1088)?;
                    }
                }
                b'\r' => {}
                b'\n' => {
                    // Native retains a shared cursor here. Switching an unfinished
                    // key/section to a different destination can read unknown bytes.
                    if matches!(field, Field::Key | Field::Section) {
                        return Err(JuiceIniError::UnterminatedKey);
                    }
                    field = if field == Field::Value {
                        Field::Done
                    } else {
                        Field::Seeking
                    };
                    if bytes.get(offset + 1) == Some(&b'\n') {
                        offset += 1;
                    }
                }
                b'"' => {
                    if field == Field::Value {
                        quoted = !quoted;
                    }
                }
                b'/' if bytes.get(offset + 1) == Some(&b'/') => {
                    if matches!(field, Field::Key | Field::Section) {
                        return Err(JuiceIniError::UnterminatedKey);
                    }
                    if field == Field::Value {
                        field = Field::Done;
                        continue;
                    }
                    field = Field::Comment;
                }
                b'/' => match field {
                    Field::Value => push(&mut value, byte, 1088)?,
                    Field::Section => push(&mut section, byte, 256)?,
                    _ => {}
                },
                b'=' if field == Field::Key => {
                    field = Field::Value;
                }
                b'[' if field == Field::Seeking => {
                    section.clear();
                    field = Field::Section;
                }
                b']' if field == Field::Section => {
                    field = Field::Seeking;
                }
                b'=' | b'[' | b']' => {
                    if field == Field::Value {
                        push(&mut value, byte, 1088)?;
                    }
                }
                _ => match field {
                    Field::Seeking => {
                        field = Field::Key;
                        push(&mut key, byte, 256)?;
                    }
                    Field::Section => push(&mut section, byte, 256)?,
                    Field::Key => push(&mut key, byte, 256)?,
                    Field::Value => push(&mut value, byte, 1088)?,
                    Field::Comment | Field::Done => {}
                },
            }
            offset += 1;
        }
        if c_string(&key).is_empty() {
            break;
        }
        if !matches!(field, Field::Value | Field::Done) {
            return Err(JuiceIniError::UnterminatedKey);
        }
        if c_string(&section).eq_ignore_ascii_case(b"JUICECONFIG") {
            apply_setting(config, c_string(&key), c_string(&value))?;
        }
    }
    Ok(())
}
fn apply_setting(config: &mut JuiceConfig, key: &[u8], value: &[u8]) -> Result<(), JuiceIniError> {
    if key.eq_ignore_ascii_case(b"enable") {
        config.enabled = crate::input_cvar_runtime::native_ini_integer(value) == 1;
    } else if key.eq_ignore_ascii_case(b"serverport") {
        config.port = crate::input_cvar_runtime::native_ini_integer(value);
    } else {
        for (name, destination) in [
            (b"serverip".as_slice(), &mut config.server),
            (b"serverip_HTC".as_slice(), &mut config.htc),
            (b"serverip_Madrid".as_slice(), &mut config.madrid),
            (b"serverip_VMC".as_slice(), &mut config.vmc),
        ] {
            if key.eq_ignore_ascii_case(name) {
                // StringnCopy(...,31); preserve unknown terminator boundary.
                if value.len() >= 31 {
                    return Err(JuiceIniError::UnterminatedAddress);
                }
                *destination = Some(value.to_vec());
                break;
            }
        }
    }
    Ok(())
}
