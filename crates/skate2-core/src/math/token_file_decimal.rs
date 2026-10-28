//! Shared authored decimal admission for native `TokenFile` consumers.
//! This finite whole-token subset does not claim malformed CRT prefix parsing.
#[expect(
    clippy::cast_possible_truncation,
    reason = "native atof double then frsp"
)]
pub(crate) fn finite_decimal(token: &str) -> Option<f32> {
    let body = token.strip_prefix(['-', '+']).unwrap_or(token);
    let mut digits = 0;
    let mut dots = 0;
    for byte in body.bytes() {
        if byte.is_ascii_digit() {
            digits += 1;
        } else if byte == b'.' {
            dots += 1;
        } else {
            return None;
        }
    }
    if digits == 0 || dots > 1 {
        return None;
    }
    let value = token.parse::<f64>().ok()? as f32;
    value.is_finite().then_some(value)
}
