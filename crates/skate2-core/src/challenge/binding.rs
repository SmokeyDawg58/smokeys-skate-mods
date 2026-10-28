//! `Lua::Binding` argument helpers (`0x82aaec90`..): the shipping build's
//! semantics, where a wrongly typed argument reads as Lua's coercion would
//! (the debug build asserts instead).

use super::lua::{NativeCall, Value, number::number_to_string, number::string_to_number};

fn to_number(value: &Value) -> Option<f32> {
    match value {
        Value::Number(n) => Some(*n),
        Value::String(s) => string_to_number(s.as_bytes()),
        _ => None,
    }
}

/// `GetFloat(lua, i)`: `lua_tonumber`.
#[must_use]
pub fn get_float(call: &NativeCall, index: usize) -> f32 {
    to_number(call.arg(index)).unwrap_or(0.0)
}

/// `GetFloat(lua, i, default)`.
#[must_use]
pub fn get_float_or(call: &NativeCall, index: usize, default: f32) -> f32 {
    to_number(call.arg(index)).unwrap_or(default)
}

/// The rounding in `GetInt`: `(int)(float)(v ± 0.5)`.
#[expect(clippy::cast_possible_truncation, reason = "C (int) conversion")]
fn round(v: f32) -> i32 {
    if v < 0.0 { (v - 0.5) as i32 } else { (v + 0.5) as i32 }
}

/// `GetInt(lua, i)`.
#[must_use]
pub fn get_int(call: &NativeCall, index: usize) -> i32 {
    round(to_number(call.arg(index)).unwrap_or(0.0))
}

/// `GetInt(lua, i, default)`.
#[must_use]
pub fn get_int_or(call: &NativeCall, index: usize, default: i32) -> i32 {
    to_number(call.arg(index)).map_or(default, round)
}

/// `GetBool(lua, i)`: `lua_toboolean`.
#[must_use]
pub fn get_bool(call: &NativeCall, index: usize) -> bool {
    !matches!(call.arg(index), Value::Nil | Value::Boolean(false))
}

/// `GetBool(lua, i, default)`: a boolean argument, else the default.
#[must_use]
pub fn get_bool_or(call: &NativeCall, index: usize, default: bool) -> bool {
    match call.arg(index) {
        Value::Boolean(b) => *b,
        _ => default,
    }
}

/// `GetString(lua, i)`: `lua_tolstring` (numbers convert).
#[must_use]
pub fn get_string(call: &NativeCall, index: usize) -> Option<String> {
    match call.arg(index) {
        Value::String(s) => Some(s.to_str_lossy().into_owned()),
        Value::Number(n) => Some(number_to_string(*n)),
        _ => None,
    }
}

/// `GetString(lua, i, default)`.
#[must_use]
pub fn get_string_or(call: &NativeCall, index: usize, default: &str) -> String {
    get_string(call, index).unwrap_or_else(|| default.to_owned())
}

/// `PushInt`: `(float)int`.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "lua_Number")]
pub fn int(value: i32) -> Value {
    Value::Number(value as f32)
}

/// `PushInt` of an unsigned hash (`GetHashValue32`), as the native passes it.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "lua_Number")]
pub fn int_u32(value: u32) -> Value {
    Value::Number(value.cast_signed() as f32)
}
