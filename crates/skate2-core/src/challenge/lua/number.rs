//! `lua_Number` conversions of Skate 2's float Lua (`luaconf.h`):
//! `lua_number2str` is `sprintf("%.14g", (double)n)` (`luaV_tostring`
//! `0x8299e6a8`) and `lua_str2number` is `strtod` narrowed to `float`.

/// Exact decimal expansion of a finite double: (negative, digits, exponent of
/// the first digit). `digits` has no leading zeros unless the value is zero.
fn decimal_digits(value: f64) -> (bool, Vec<u8>, i32) {
    let negative = value.is_sign_negative();
    let text = format!("{:.1100e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').expect("exponent form");
    let exponent: i32 = exponent.parse().expect("exponent digits");
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|d| d - b'0').collect();
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    (negative, digits, exponent)
}

/// Round `digits` (first digit at decimal exponent `exponent`) to `keep`
/// significant digits, ties away from zero as the Xbox 360 CRT does.
fn round_significant(digits: &[u8], exponent: i32, keep: usize) -> (Vec<u8>, i32) {
    if digits.len() <= keep {
        let mut out = digits.to_vec();
        out.resize(keep, 0);
        return (out, exponent);
    }
    let mut out = digits[..keep].to_vec();
    if digits[keep] >= 5 {
        let mut index = keep;
        loop {
            if index == 0 {
                out.insert(0, 1);
                out.truncate(keep);
                return (out, exponent + 1);
            }
            index -= 1;
            if out[index] == 9 {
                out[index] = 0;
            } else {
                out[index] += 1;
                break;
            }
        }
    }
    (out, exponent)
}

fn special(value: f64) -> Option<String> {
    if value.is_nan() {
        return Some(if value.is_sign_negative() { "-1.#IND".into() } else { "1.#QNAN".into() });
    }
    if value.is_infinite() {
        return Some(if value < 0.0 { "-1.#INF".into() } else { "1.#INF".into() });
    }
    None
}

fn exponent_suffix(out: &mut String, exponent: i32, upper: bool) {
    out.push(if upper { 'E' } else { 'e' });
    out.push(if exponent < 0 { '-' } else { '+' });
    // The 360 CRT prints at least three exponent digits.
    out.push_str(&format!("{:03}", exponent.unsigned_abs()));
}

/// C `%.<precision>e` body (no width/flags).
#[must_use]
pub fn format_e(value: f64, precision: usize, upper: bool) -> String {
    if let Some(s) = special(value) {
        return s;
    }
    let (negative, digits, exponent) = decimal_digits(value);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if value == 0.0 {
        out.push('0');
        if precision > 0 {
            out.push('.');
            out.extend(std::iter::repeat_n('0', precision));
        }
        exponent_suffix(&mut out, 0, upper);
        return out;
    }
    let (rounded, exponent) = round_significant(&digits, exponent, precision + 1);
    out.push(char::from(b'0' + rounded[0]));
    if precision > 0 {
        out.push('.');
        out.extend(rounded[1..].iter().map(|d| char::from(b'0' + d)));
    }
    exponent_suffix(&mut out, exponent, upper);
    out
}

/// C `%.<precision>f` body (no width/flags).
#[must_use]
pub fn format_f(value: f64, precision: usize) -> String {
    if let Some(s) = special(value) {
        return s;
    }
    let (negative, digits, exponent) = decimal_digits(value);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if value == 0.0 {
        out.push('0');
        if precision > 0 {
            out.push('.');
            out.extend(std::iter::repeat_n('0', precision));
        }
        return out;
    }
    // Significant digits kept: integer digits + precision fraction digits.
    let keep = exponent + 1 + i32::try_from(precision).unwrap_or(i32::MAX);
    let (rounded, exponent) = if keep <= 0 {
        // Everything rounds into the first fraction position or below.
        let first_dropped_is_half = keep == 0 && digits[0] >= 5;
        if first_dropped_is_half { (vec![1], exponent + 1) } else { (vec![0], exponent) }
    } else {
        round_significant(&digits, exponent, usize::try_from(keep).unwrap_or(usize::MAX))
    };
    let digit_at = |position: i32| -> u8 {
        // position is a decimal exponent; rounded[0] sits at `exponent`.
        let index = exponent - position;
        if index < 0 {
            return 0;
        }
        rounded.get(usize::try_from(index).unwrap_or(usize::MAX)).copied().unwrap_or(0)
    };
    if exponent < 0 {
        out.push('0');
    } else {
        for position in (0..=exponent).rev() {
            out.push(char::from(b'0' + digit_at(position)));
        }
    }
    if precision > 0 {
        out.push('.');
        for k in 1..=i32::try_from(precision).unwrap_or(i32::MAX) {
            out.push(char::from(b'0' + digit_at(-k)));
        }
    }
    out
}

/// C `%.<precision>g` body (no `#` flag: trailing zeros removed).
#[must_use]
pub fn format_g(value: f64, precision: usize, upper: bool) -> String {
    if let Some(s) = special(value) {
        return s;
    }
    let precision = precision.max(1);
    if value == 0.0 {
        return if value.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let (_, digits, exponent) = decimal_digits(value);
    let (_, exponent) = round_significant(&digits, exponent, precision);
    let p = i32::try_from(precision).unwrap_or(i32::MAX);
    let mut text = if exponent < -4 || exponent >= p {
        format_e(value, precision - 1, upper)
    } else {
        format_f(value, usize::try_from(p - 1 - exponent).unwrap_or(0))
    };
    let (body, suffix) = match text.find(['e', 'E']) {
        Some(at) => {
            let suffix = text[at..].to_string();
            text.truncate(at);
            (text, suffix)
        }
        None => (text, String::new()),
    };
    let mut body = body;
    if body.contains('.') {
        while body.ends_with('0') {
            body.pop();
        }
        if body.ends_with('.') {
            body.pop();
        }
    }
    body + &suffix
}

/// `lua_number2str`.
#[must_use]
pub fn number_to_string(n: f32) -> String {
    format_g(f64::from(n), 14, false)
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `luaO_str2d` (`0x8299ac80`): `strtod`, then `strtoul` for `0x` prefixes;
/// only trailing whitespace may follow.
#[must_use]
pub fn string_to_number(bytes: &[u8]) -> Option<f32> {
    let start = bytes.iter().position(|&b| !is_space(b)).unwrap_or(bytes.len());
    let trimmed_end = bytes.iter().rposition(|&b| !is_space(b)).map_or(start, |i| i + 1);
    let text = std::str::from_utf8(&bytes[start..trimmed_end.max(start)]).ok()?;
    if text.is_empty() {
        return None;
    }
    let (negative, unsigned) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if let Some(hex) = unsigned.strip_prefix("0x").or_else(|| unsigned.strip_prefix("0X")) {
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(hex, 16).unwrap_or(u32::MAX);
        let value = if negative { value.wrapping_neg() } else { value };
        #[expect(clippy::cast_precision_loss, reason = "lua_Number is float")]
        return Some(value as f32);
    }
    let lower = unsigned.to_ascii_lowercase();
    let value = match lower.as_str() {
        "inf" | "infinity" => f64::INFINITY,
        "nan" => f64::NAN,
        _ => {
            let valid = !lower.is_empty()
                && lower.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'-' | b'+'))
                && lower.bytes().next().is_some_and(|b| b.is_ascii_digit() || b == b'.');
            if !valid {
                return None;
            }
            lower.parse::<f64>().ok()?
        }
    };
    let value = if negative { -value } else { value };
    #[expect(clippy::cast_possible_truncation, reason = "strtod result narrowed to lua_Number")]
    Some(value as f32)
}

/// `(long)n` on the 360: `fctiwz` saturates and maps NaN to `INT_MIN`.
#[must_use]
pub fn to_c_long(n: f64) -> i32 {
    if n.is_nan() {
        return i32::MIN;
    }
    #[expect(clippy::cast_possible_truncation, reason = "saturating C conversion")]
    {
        n.trunc().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    }
}

/// `(unsigned long)n` on the 360 (`fctidz` then low word).
#[must_use]
pub fn to_c_ulong(n: f64) -> u32 {
    if n.is_nan() {
        return 0;
    }
    #[expect(clippy::cast_possible_truncation, reason = "C conversion keeps the low word")]
    {
        (n.trunc().clamp(-9.223_372_036_854_776e18, 9.223_372_036_854_776e18) as i64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_sprintf_14g() {
        assert_eq!(number_to_string(1.0), "1");
        assert_eq!(number_to_string(0.1), "0.10000000149012");
        assert_eq!(number_to_string(-2.5), "-2.5");
        assert_eq!(number_to_string(3.0e-5), "2.9999999242136e-005");
        assert_eq!(number_to_string(123_456_789.0), "123456792");
        assert_eq!(number_to_string(1.0e20), "1.0000000200409e+020");
        assert_eq!(number_to_string(0.0), "0");
        assert_eq!(number_to_string(-0.0), "-0");
    }

    #[test]
    fn fixed_rounds_ties_away_from_zero() {
        assert_eq!(format_f(0.125, 2), "0.13");
        assert_eq!(format_f(2.5, 0), "3");
        assert_eq!(format_f(0.004, 2), "0.00");
        assert_eq!(format_f(0.005, 2), "0.01");
        assert_eq!(format_f(99.996, 2), "100.00");
        assert_eq!(format_f(1.5, 6), "1.500000");
    }

    #[test]
    fn parses_like_luao_str2d() {
        assert_eq!(string_to_number(b" 12 "), Some(12.0));
        assert_eq!(string_to_number(b"0x10"), Some(16.0));
        assert_eq!(string_to_number(b"1e2"), Some(100.0));
        assert_eq!(string_to_number(b".5"), Some(0.5));
        assert_eq!(string_to_number(b"12a"), None);
        assert_eq!(string_to_number(b""), None);
    }
}
