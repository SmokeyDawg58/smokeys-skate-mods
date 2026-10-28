//! `TokenFile` whitespace tokens plus the original finite decimal subset.
use super::{InputPattern, InputPatterns};
use std::{fmt, sync::Arc};
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InputPatternParseError {
    Encoding,
    MissingOperand,
    Number(String),
    Unsupported(String),
    MissingPattern,
    Name,
    Capacity,
    PointCount,
}
impl fmt::Display for InputPatternParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding => f.write_str("pattern requires ASCII source tokens"),
            Self::MissingOperand => f.write_str("pattern operand missing"),
            Self::Number(n) => write!(f, "unsupported original decimal token: {n}"),
            Self::Unsupported(n) => write!(f, "unsupported pattern directive: {n}"),
            Self::MissingPattern => f.write_str("pattern directive precedes its owner"),
            Self::Name => f.write_str("pattern name exceeds bounded source name storage"),
            Self::Capacity => {
                f.write_str("pattern collection/ring exceeds bounded native capacity")
            }
            Self::PointCount => {
                f.write_str("pattern requires two to five points before cache reset")
            }
        }
    }
}
fn number(tokens: &mut std::str::SplitAsciiWhitespace<'_>) -> Result<f32, InputPatternParseError> {
    let token = tokens
        .next()
        .ok_or(InputPatternParseError::MissingOperand)?;
    crate::token_file_decimal::finite_decimal(token)
        .ok_or_else(|| InputPatternParseError::Number(token.to_owned()))
}
pub(super) fn parse(bytes: &[u8]) -> Result<InputPatterns, InputPatternParseError> {
    if !bytes.is_ascii()
        || bytes
            .iter()
            .any(|b| b.is_ascii_control() && !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return Err(InputPatternParseError::Encoding);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| InputPatternParseError::Encoding)?;
    let mut tokens = text.split_ascii_whitespace();
    let mut records: Vec<InputPattern> = Vec::new();
    let mut tolerance = f32::from_bits(0x3e23_d70b);
    while let Some(token) = tokens.next() {
        match token {
            "global_tolerance_dist" => {
                let value = number(&mut tokens)?;
                tolerance = value * value;
            }
            "global_tolerance_time"
            | "global_tolerance_speed"
            | "global_anticipation_delay"
            | "min_time" => {
                number(&mut tokens)?;
            }
            "pattern" => {
                let name = tokens
                    .next()
                    .ok_or(InputPatternParseError::MissingOperand)?;
                if name.len() >= 32 {
                    return Err(InputPatternParseError::Name);
                }
                if records.len() == 80 {
                    return Err(InputPatternParseError::Capacity);
                }
                records.push(InputPattern {
                    name: name.to_owned(),
                    points: Vec::new(),
                    squared_tolerance: tolerance,
                });
            }
            "tolerance_dist" => {
                let pattern = records
                    .last_mut()
                    .ok_or(InputPatternParseError::MissingPattern)?;
                let value = number(&mut tokens)?;
                pattern.squared_tolerance = value * value;
            }
            "coord" => {
                let x = number(&mut tokens)?;
                let y = number(&mut tokens)?;
                let pattern = records
                    .last_mut()
                    .ok_or(InputPatternParseError::MissingPattern)?;
                if pattern.points.len() == 5 {
                    return Err(InputPatternParseError::Capacity);
                }
                pattern.points.insert(0, [x, y]);
            }
            other => return Err(InputPatternParseError::Unsupported(other.to_owned())),
        }
    }
    if records.iter().any(|p| p.points.len() < 2) {
        return Err(InputPatternParseError::PointCount);
    }
    Ok(InputPatterns {
        patterns: records.into_iter().map(Arc::new).collect(),
    })
}
