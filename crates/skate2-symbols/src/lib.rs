//! Parsers and indices for reconstruction evidence.

use std::fmt;

/// A public symbol record from the Microsoft linker map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapSymbol {
    pub segment: u16,
    pub offset: u32,
    pub decorated_name: String,
    pub address: u32,
    pub attribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapLineError {
    InvalidSegment,
    InvalidOffset,
    MissingName,
    InvalidAddress,
}

impl fmt::Display for MapLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid linker-map symbol line: {self:?}")
    }
}

impl std::error::Error for MapLineError {}

/// Parses a line from the `Publics by Value` section.
///
/// Non-symbol lines return `Ok(None)` so callers can stream an entire map file.
///
/// # Errors
///
/// Returns [`MapLineError`] when a line has the shape of a symbol record but one
/// of its hexadecimal fields or required columns is malformed.
pub fn parse_public_symbol(line: &str) -> Result<Option<MapSymbol>, MapLineError> {
    let trimmed = line.trim_start();
    let Some((location, remainder)) = split_field(trimmed) else {
        return Ok(None);
    };
    let Some((segment, offset)) = location.split_once(':') else {
        return Ok(None);
    };
    if segment.len() != 4 || offset.len() != 8 {
        return Ok(None);
    }

    let segment = u16::from_str_radix(segment, 16).map_err(|_| MapLineError::InvalidSegment)?;
    let offset = u32::from_str_radix(offset, 16).map_err(|_| MapLineError::InvalidOffset)?;
    let (decorated_name, remainder) = split_field(remainder).ok_or(MapLineError::MissingName)?;
    let (address, attribution) = split_field(remainder).ok_or(MapLineError::InvalidAddress)?;
    // Section-layout records begin with the same `ssss:oooooooo` location but
    // their third field is a section name rather than an absolute address.
    if address.len() != 8 || !address.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let address = u32::from_str_radix(address, 16).map_err(|_| MapLineError::InvalidAddress)?;

    Ok(Some(MapSymbol {
        segment,
        offset,
        decorated_name: decorated_name.to_owned(),
        address,
        attribution: attribution.trim().to_owned(),
    }))
}

fn split_field(input: &str) -> Option<(&str, &str)> {
    let end = input.find(char::is_whitespace)?;
    let (field, tail) = input.split_at(end);
    Some((field, tail.trim_start()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decorated_function() {
        let line = " 0003:00351c90       ?InitDirtySock@PlasmaSystemPlatform@Net@Sk8@@MAAXXZ 82781c90 f   net_lib.obj";
        let symbol = parse_public_symbol(line).unwrap().unwrap();
        assert_eq!(symbol.segment, 3);
        assert_eq!(symbol.offset, 0x0035_1c90);
        assert_eq!(symbol.address, 0x8278_1c90);
        assert_eq!(
            symbol.decorated_name,
            "?InitDirtySock@PlasmaSystemPlatform@Net@Sk8@@MAAXXZ"
        );
        assert_eq!(symbol.attribution, "f   net_lib.obj");
    }

    #[test]
    fn ignores_headers() {
        assert_eq!(
            parse_public_symbol(" Address Publics by Value").unwrap(),
            None
        );
    }

    #[test]
    fn ignores_section_layout_record() {
        let line = " 0003:00000000 00b1a1c4H .text                   CODE";
        assert_eq!(parse_public_symbol(line).unwrap(), None);
    }
}
