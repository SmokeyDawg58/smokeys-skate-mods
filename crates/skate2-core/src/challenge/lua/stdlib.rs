//! The `luaL_openlibs` (`0x82edd3f0`) functions reachable from the shipped
//! scripts: base (`lbaselib.c`), `string`, `table`, and `math`. The scripts
//! themselves only call `print`, `string.format`, `string.len`, `table.getn`,
//! `table.insert`, `table.sort`, `math.abs`, `math.floor` and `math.max`.

use super::{Lua, LuaError, LuaResult, NativeCall, NativeFn, TableId, Value, number, vm::to_number};

fn register<H>(lua: &mut Lua<H>, table: TableId, name: &'static str, function: NativeFn<H>) {
    let f = lua.create_native(name, function, Vec::new());
    lua.table_mut(table).set_str(name, f);
}

pub(crate) fn open<H>(lua: &mut Lua<H>) {
    let g = lua.globals();
    for (name, f) in [
        ("print", print::<H> as NativeFn<H>),
        ("tostring", tostring::<H>),
        ("tonumber", tonumber::<H>),
        ("type", type_of::<H>),
        ("next", next::<H>),
        ("pairs", pairs::<H>),
        ("ipairs", ipairs::<H>),
        ("select", select::<H>),
        ("unpack", unpack::<H>),
        ("assert", assert::<H>),
        ("error", error::<H>),
        ("pcall", pcall::<H>),
        ("rawget", rawget::<H>),
        ("rawset", rawset::<H>),
        ("rawequal", rawequal::<H>),
        ("setmetatable", setmetatable::<H>),
        ("getmetatable", getmetatable::<H>),
    ] {
        register(lua, g, name, f);
    }
    let string = lua.create_table();
    for (name, f) in [
        ("format", string_format::<H> as NativeFn<H>),
        ("len", string_len::<H>),
        ("sub", string_sub::<H>),
        ("upper", string_upper::<H>),
        ("lower", string_lower::<H>),
        ("rep", string_rep::<H>),
        ("byte", string_byte::<H>),
        ("char", string_char::<H>),
    ] {
        register(lua, string, name, f);
    }
    lua.set_global("string", Value::Table(string));
    // luaopen_string's createmetatable: strings index the string table.
    let meta = lua.create_table();
    lua.table_mut(meta).set_str("__index", Value::Table(string));
    lua.set_metatable(&Value::string(""), Some(meta));

    let table = lua.create_table();
    for (name, f) in [
        ("getn", table_getn::<H> as NativeFn<H>),
        ("insert", table_insert::<H>),
        ("remove", table_remove::<H>),
        ("sort", table_sort::<H>),
        ("concat", table_concat::<H>),
        ("maxn", table_maxn::<H>),
    ] {
        register(lua, table, name, f);
    }
    lua.set_global("table", Value::Table(table));

    let math = lua.create_table();
    for (name, f) in [
        ("abs", math_abs::<H> as NativeFn<H>),
        ("ceil", math_ceil::<H>),
        ("floor", math_floor::<H>),
        ("max", math_max::<H>),
        ("min", math_min::<H>),
        ("sqrt", math_sqrt::<H>),
        ("fmod", math_fmod::<H>),
        ("mod", math_fmod::<H>),
        ("pow", math_pow::<H>),
    ] {
        register(lua, math, name, f);
    }
    lua.table_mut(math).set_str("pi", Value::Number(std::f32::consts::PI));
    lua.table_mut(math).set_str("huge", Value::Number(f32::INFINITY));
    lua.set_global("math", Value::Table(math));
}

fn arg_error<H>(lua: &Lua<H>, n: usize, function: &str, message: &str) -> LuaError {
    lua.error(format!("bad argument #{n} to '{function}' ({message})"))
}

pub(crate) fn type_error<H>(lua: &Lua<H>, call: &NativeCall, n: usize, function: &str, expected: &str) -> LuaError {
    let got = if n > call.len() { "no value" } else { call.arg(n - 1).type_name() };
    arg_error(lua, n, function, &format!("{expected} expected, got {got}"))
}

/// `luaL_checknumber`.
pub fn check_number<H>(lua: &Lua<H>, call: &NativeCall, n: usize, function: &str) -> LuaResult<f32> {
    to_number(call.arg(n - 1)).ok_or_else(|| type_error(lua, call, n, function, "number"))
}

/// `luaL_checklstring`: numbers convert with `lua_number2str`.
pub fn check_string<H>(lua: &Lua<H>, call: &NativeCall, n: usize, function: &str) -> LuaResult<Vec<u8>> {
    match call.arg(n - 1) {
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Number(x) => Ok(number::number_to_string(*x).into_bytes()),
        _ => Err(type_error(lua, call, n, function, "string")),
    }
}

fn check_table<H>(lua: &Lua<H>, call: &NativeCall, n: usize, function: &str) -> LuaResult<TableId> {
    call.arg(n - 1).as_table().ok_or_else(|| type_error(lua, call, n, function, "table"))
}

fn opt_int<H>(lua: &Lua<H>, call: &NativeCall, n: usize, function: &str, default: i64) -> LuaResult<i64> {
    if call.arg(n - 1).is_nil() {
        return Ok(default);
    }
    Ok(i64::from(number::to_c_long(f64::from(check_number(lua, call, n, function)?))))
}

#[expect(clippy::cast_precision_loss, reason = "lua_pushinteger narrows to lua_Number")]
fn int(n: i64) -> Value {
    Value::Number(n as f32)
}

/// `luaB_tostring` with `__tostring`.
pub fn to_display<H>(lua: &mut Lua<H>, host: &mut H, value: &Value) -> LuaResult<Vec<u8>> {
    let tm = lua.metamethod(value, "__tostring");
    if !tm.is_nil() {
        let r = lua.call_function(host, tm, std::slice::from_ref(value), Some(1))?;
        return match r.first() {
            Some(Value::String(s)) => Ok(s.as_bytes().to_vec()),
            Some(Value::Number(x)) => Ok(number::number_to_string(*x).into_bytes()),
            _ => Err(lua.error("'__tostring' must return a string")),
        };
    }
    Ok(match value {
        Value::String(s) => s.as_bytes().to_vec(),
        other => lua.display(other).into_bytes(),
    })
}

fn print<H>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let tostring = lua.get_global("tostring");
    let mut line = Vec::new();
    for (i, v) in call.args.iter().enumerate() {
        let s = lua.call_function(host, tostring.clone(), std::slice::from_ref(v), Some(1))?;
        let Some(Value::String(s)) = s.first() else {
            return Err(lua.error("'tostring' must return a string to 'print'"));
        };
        if i > 0 {
            line.push(b'\t');
        }
        line.extend_from_slice(s.as_bytes());
    }
    if let Some(sink) = lua.print {
        sink(&String::from_utf8_lossy(&line));
    }
    Ok(Vec::new())
}

fn tostring<H>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if call.is_empty() {
        return Err(arg_error(lua, 1, "tostring", "value expected"));
    }
    Ok(vec![Value::bytes(&to_display(lua, host, call.arg(0))?)])
}

fn tonumber<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let base = opt_int(lua, call, 2, "tonumber", 10)?;
    if base == 10 {
        return Ok(vec![to_number(call.arg(0)).map_or(Value::Nil, Value::Number)]);
    }
    if !(2..=36).contains(&base) {
        return Err(arg_error(lua, 2, "tonumber", "base out of range"));
    }
    let text = check_string(lua, call, 1, "tonumber")?;
    let text = String::from_utf8_lossy(&text);
    let trimmed = text.trim();
    let parsed = u32::from_str_radix(trimmed, u32::try_from(base).expect("checked"));
    #[expect(clippy::cast_precision_loss, reason = "strtoul result as lua_Number")]
    Ok(vec![parsed.map_or(Value::Nil, |n| Value::Number(n as f32))])
}

fn type_of<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if call.is_empty() {
        return Err(arg_error(lua, 1, "type", "value expected"));
    }
    Ok(vec![Value::string(call.arg(0).type_name())])
}

fn next<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "next")?;
    match lua.table(t).next(call.arg(1)) {
        Ok(Some((k, v))) => Ok(vec![k, v]),
        Ok(None) => Ok(vec![Value::Nil]),
        Err(()) => Err(lua.error("invalid key to 'next'")),
    }
}

fn pairs<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    check_table(lua, call, 1, "pairs")?;
    Ok(vec![lua.get_global("next"), call.arg(0).clone(), Value::Nil])
}

fn ipairs_step<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "ipairs")?;
    let i = usize::try_from(number::to_c_long(f64::from(check_number(lua, call, 2, "ipairs")?)) + 1).unwrap_or(0);
    let v = lua.table(t).get_int(i);
    #[expect(clippy::cast_precision_loss, reason = "lua_Number index")]
    Ok(if v.is_nil() { vec![Value::Nil] } else { vec![Value::Number(i as f32), v] })
}

fn ipairs<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    check_table(lua, call, 1, "ipairs")?;
    let step = lua.create_native("ipairs_aux", ipairs_step::<H>, Vec::new());
    Ok(vec![step, call.arg(0).clone(), Value::Number(0.0)])
}

fn select<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let rest = call.len().saturating_sub(1);
    if let Value::String(s) = call.arg(0)
        && s.as_bytes() == b"#"
    {
        return Ok(vec![int(i64::try_from(rest).unwrap_or(i64::MAX))]);
    }
    let n = i64::from(number::to_c_long(f64::from(check_number(lua, call, 1, "select")?)));
    let rest = i64::try_from(rest).unwrap_or(i64::MAX);
    let n = if n < 0 { rest + n + 1 } else { n };
    if n < 1 {
        return Err(arg_error(lua, 1, "select", "index out of range"));
    }
    Ok(call.args.iter().skip(usize::try_from(n).unwrap_or(usize::MAX)).cloned().collect())
}

fn unpack<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "unpack")?;
    let i = opt_int(lua, call, 2, "unpack", 1)?;
    let len = i64::try_from(lua.table(t).len()).unwrap_or(i64::MAX);
    let j = opt_int(lua, call, 3, "unpack", len)?;
    Ok((i..=j).map(|k| lua.table(t).get_int(usize::try_from(k).unwrap_or(0))).collect())
}

fn assert<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if call.arg(0).is_falsy() {
        let message = match call.arg(1) {
            Value::Nil => "assertion failed!".to_string(),
            other => lua.display(other),
        };
        return Err(lua.error(message));
    }
    Ok(call.args.clone())
}

fn error<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let message = lua.display(call.arg(0));
    Err(lua.error(message))
}

fn pcall<H>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if call.is_empty() {
        return Err(arg_error(lua, 1, "pcall", "value expected"));
    }
    let frames = lua.frames.len();
    let depth = lua.depth;
    let top = lua.top;
    match lua.call_function(host, call.arg(0).clone(), &call.args[1..], None) {
        Ok(mut results) => {
            results.insert(0, Value::Boolean(true));
            Ok(results)
        }
        Err(e) => {
            lua.close_upvalues(top);
            lua.frames.truncate(frames);
            lua.depth = depth;
            lua.top = top;
            Ok(vec![Value::Boolean(false), Value::string(&e.message)])
        }
    }
}

fn rawget<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "rawget")?;
    Ok(vec![lua.table(t).get(call.arg(1))])
}

fn rawset<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "rawset")?;
    lua.table_mut(t).set(call.arg(1).clone(), call.arg(2).clone()).map_err(|e| lua.error(e.message()))?;
    Ok(vec![call.arg(0).clone()])
}

fn rawequal<H>(_: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(vec![Value::Boolean(call.arg(0).raw_equal(call.arg(1)))])
}

fn setmetatable<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "setmetatable")?;
    let mt = match call.arg(1) {
        Value::Nil => None,
        Value::Table(mt) => Some(*mt),
        _ => return Err(arg_error(lua, 2, "setmetatable", "nil or table expected")),
    };
    if let Some(old) = lua.table(t).metatable
        && !lua.table(old).get_str("__metatable").is_nil()
    {
        return Err(lua.error("cannot change a protected metatable"));
    }
    lua.table_mut(t).metatable = mt;
    Ok(vec![call.arg(0).clone()])
}

fn getmetatable<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let Some(mt) = lua.metatable_of(call.arg(0)) else { return Ok(vec![Value::Nil]) };
    let protected = lua.table(mt).get_str("__metatable");
    Ok(vec![if protected.is_nil() { Value::Table(mt) } else { protected }])
}

fn string_len<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "len")?;
    Ok(vec![int(i64::try_from(s.len()).unwrap_or(i64::MAX))])
}

/// `posrelat`.
fn relative(pos: i64, len: usize) -> i64 {
    let len = i64::try_from(len).unwrap_or(i64::MAX);
    if pos < 0 { len + pos + 1 } else { pos }
}

fn string_sub<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "sub")?;
    let len = i64::try_from(s.len()).unwrap_or(i64::MAX);
    let mut start = relative(opt_int(lua, call, 2, "sub", 1)?, s.len());
    let mut end = relative(opt_int(lua, call, 3, "sub", -1)?, s.len());
    if start < 1 {
        start = 1;
    }
    if end > len {
        end = len;
    }
    if start > end {
        return Ok(vec![Value::string("")]);
    }
    let (start, end) = (usize::try_from(start - 1).unwrap_or(0), usize::try_from(end).unwrap_or(0));
    Ok(vec![Value::bytes(&s[start..end])])
}

fn string_upper<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "upper")?;
    Ok(vec![Value::bytes(&s.to_ascii_uppercase())])
}

fn string_lower<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "lower")?;
    Ok(vec![Value::bytes(&s.to_ascii_lowercase())])
}

fn string_rep<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "rep")?;
    let n = number::to_c_long(f64::from(check_number(lua, call, 2, "rep")?));
    Ok(vec![Value::bytes(&s.repeat(usize::try_from(n).unwrap_or(0)))])
}

fn string_byte<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let s = check_string(lua, call, 1, "byte")?;
    let start = relative(opt_int(lua, call, 2, "byte", 1)?, s.len()).max(1);
    let end = relative(opt_int(lua, call, 3, "byte", start)?, s.len()).min(i64::try_from(s.len()).unwrap_or(0));
    Ok((start..=end).map(|i| int(i64::from(s[usize::try_from(i - 1).unwrap_or(0)]))).collect())
}

fn string_char<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let mut out = Vec::with_capacity(call.len());
    for n in 1..=call.len() {
        let c = number::to_c_long(f64::from(check_number(lua, call, n, "char")?));
        out.push(u8::try_from(c).map_err(|_| arg_error(lua, n, "char", "invalid value"))?);
    }
    Ok(vec![Value::bytes(&out)])
}

/// C `printf` padding for one converted field.
fn pad(body: String, flags: &str, width: usize, zero_fill: bool) -> String {
    let len = body.len();
    if len >= width {
        return body;
    }
    let fill = width - len;
    if flags.contains('-') {
        return body + &" ".repeat(fill);
    }
    if zero_fill && flags.contains('0') {
        let sign_len = usize::from(body.starts_with(['-', '+', ' ']));
        let rest = &body[sign_len..];
        let prefix_len = sign_len + if rest.starts_with("0x") || rest.starts_with("0X") { 2 } else { 0 };
        return format!("{}{}{}", &body[..prefix_len], "0".repeat(fill), &body[prefix_len..]);
    }
    " ".repeat(fill) + &body
}

fn sign(flags: &str, negative: bool) -> &'static str {
    if negative {
        "-"
    } else if flags.contains('+') {
        "+"
    } else if flags.contains(' ') {
        " "
    } else {
        ""
    }
}

/// `str_format` (`lstrlib.c`) with the 360 CRT's `sprintf` conversions.
fn string_format<H>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let format = check_string(lua, call, 1, "format")?;
    let mut out = Vec::with_capacity(format.len() + 16);
    let mut arg = 1;
    let mut i = 0;
    while i < format.len() {
        if format[i] != b'%' {
            out.push(format[i]);
            i += 1;
            continue;
        }
        i += 1;
        if format.get(i) == Some(&b'%') {
            out.push(b'%');
            i += 1;
            continue;
        }
        let spec_start = i;
        while i < format.len() && b"-+ #0".contains(&format[i]) {
            i += 1;
        }
        if i - spec_start > 5 {
            return Err(lua.error("invalid format (repeated flags)"));
        }
        let flags = String::from_utf8_lossy(&format[spec_start..i]).into_owned();
        let width_start = i;
        while i < format.len() && format[i].is_ascii_digit() {
            i += 1;
        }
        let width_digits = i - width_start;
        let width: usize = std::str::from_utf8(&format[width_start..i]).unwrap_or("").parse().unwrap_or(0);
        let mut precision: Option<usize> = None;
        let mut precision_digits = 0;
        if format.get(i) == Some(&b'.') {
            i += 1;
            let p_start = i;
            while i < format.len() && format[i].is_ascii_digit() {
                i += 1;
            }
            precision_digits = i - p_start;
            precision = Some(std::str::from_utf8(&format[p_start..i]).unwrap_or("").parse().unwrap_or(0));
        }
        if width_digits > 2 || precision_digits > 2 {
            return Err(lua.error("invalid format (width or precision too long)"));
        }
        let Some(&conversion) = format.get(i) else {
            return Err(lua.error("invalid option '' to 'format'"));
        };
        i += 1;
        arg += 1;
        let field = match conversion {
            b'c' => {
                let c = number::to_c_long(f64::from(check_number(lua, call, arg, "format")?));
                #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation, reason = "C (char) conversion")]
                let byte = c as u8;
                pad(char::from(byte).to_string(), &flags, width, false)
            }
            b'd' | b'i' => {
                let n = number::to_c_long(f64::from(check_number(lua, call, arg, "format")?));
                let mut digits = n.unsigned_abs().to_string();
                if let Some(p) = precision {
                    if digits.len() < p {
                        digits = "0".repeat(p - digits.len()) + &digits;
                    }
                    if p == 0 && n == 0 {
                        digits.clear();
                    }
                }
                let body = format!("{}{digits}", sign(&flags, n < 0));
                pad(body, &flags, width, precision.is_none())
            }
            b'o' | b'u' | b'x' | b'X' => {
                let n = number::to_c_ulong(f64::from(check_number(lua, call, arg, "format")?));
                let mut digits = match conversion {
                    b'o' => format!("{n:o}"),
                    b'u' => n.to_string(),
                    b'x' => format!("{n:x}"),
                    _ => format!("{n:X}"),
                };
                if let Some(p) = precision
                    && digits.len() < p
                {
                    digits = "0".repeat(p - digits.len()) + &digits;
                }
                if flags.contains('#') && n != 0 {
                    digits = match conversion {
                        b'o' => format!("0{digits}"),
                        b'x' => format!("0x{digits}"),
                        b'X' => format!("0X{digits}"),
                        _ => digits,
                    };
                }
                pad(digits, &flags, width, precision.is_none())
            }
            b'e' | b'E' | b'f' | b'g' | b'G' => {
                let n = f64::from(check_number(lua, call, arg, "format")?);
                let p = precision.unwrap_or(6);
                let mut body = match conversion {
                    b'e' | b'E' => number::format_e(n, p, conversion == b'E'),
                    b'f' => number::format_f(n, p),
                    _ => number::format_g(n, p, conversion == b'G'),
                };
                if !body.starts_with('-') {
                    body = format!("{}{body}", sign(&flags, false));
                }
                pad(body, &flags, width, n.is_finite())
            }
            b'q' => {
                let s = check_string(lua, call, arg, "format")?;
                let mut q = vec![b'"'];
                for &b in &s {
                    match b {
                        b'"' | b'\\' | b'\n' => {
                            q.push(b'\\');
                            q.push(b);
                        }
                        b'\r' => q.extend_from_slice(b"\\r"),
                        0 => q.extend_from_slice(b"\\000"),
                        _ => q.push(b),
                    }
                }
                q.push(b'"');
                out.extend_from_slice(&q);
                continue;
            }
            b's' => {
                let value = call.arg(arg - 1).clone();
                if arg > call.len() {
                    return Err(type_error(lua, call, arg, "format", "string"));
                }
                let s = match value {
                    Value::String(s) => s.as_bytes().to_vec(),
                    Value::Number(x) => number::number_to_string(x).into_bytes(),
                    other => {
                        let _ = to_display(lua, host, &other)?;
                        return Err(type_error(lua, call, arg, "format", "string"));
                    }
                };
                if precision.is_none() && s.len() >= 100 {
                    out.extend_from_slice(&s);
                    continue;
                }
                let s = match precision {
                    Some(p) if p < s.len() => &s[..p],
                    _ => &s[..],
                };
                let text = String::from_utf8_lossy(s).into_owned();
                pad(text, &flags, width, false)
            }
            other => {
                return Err(lua.error(format!("invalid option '{}' to 'format'", char::from(other))));
            }
        };
        out.extend_from_slice(field.as_bytes());
    }
    Ok(vec![Value::bytes(&out)])
}

/// `aux_getn`: `luaL_checktype(table)` then `lua_objlen`.
fn aux_getn<H>(lua: &Lua<H>, call: &NativeCall, function: &str) -> LuaResult<(TableId, i64)> {
    let t = check_table(lua, call, 1, function)?;
    Ok((t, i64::try_from(lua.table(t).len()).unwrap_or(i64::MAX)))
}

fn table_getn<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (_, n) = aux_getn(lua, call, "getn")?;
    Ok(vec![int(n)])
}

fn table_maxn<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let t = check_table(lua, call, 1, "maxn")?;
    let mut max = 0.0f32;
    let mut key = Value::Nil;
    while let Ok(Some((k, _))) = lua.table(t).next(&key) {
        if let Value::Number(n) = k
            && n > max
        {
            max = n;
        }
        key = k;
    }
    Ok(vec![Value::Number(max)])
}

fn to_index(i: i64) -> usize {
    usize::try_from(i).unwrap_or(0)
}

fn table_insert<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (t, n) = aux_getn(lua, call, "insert")?;
    let e = n + 1;
    let position = match call.len() {
        2 => e,
        3 => {
            let pos = i64::from(number::to_c_long(f64::from(check_number(lua, call, 2, "insert")?)));
            let top = pos.max(e);
            for i in (pos + 1..=top).rev() {
                let v = lua.table(t).get_int(to_index(i - 1));
                lua.table_mut(t).set_int(to_index(i), v);
            }
            pos
        }
        _ => return Err(lua.error("wrong number of arguments to 'insert'")),
    };
    let value = call.arg(call.len() - 1).clone();
    lua.table_mut(t).set_int(to_index(position), value);
    Ok(Vec::new())
}

fn table_remove<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (t, e) = aux_getn(lua, call, "remove")?;
    let pos = opt_int(lua, call, 2, "remove", e)?;
    if e == 0 {
        return Ok(Vec::new());
    }
    let removed = lua.table(t).get_int(to_index(pos));
    for i in pos..e {
        let v = lua.table(t).get_int(to_index(i + 1));
        lua.table_mut(t).set_int(to_index(i), v);
    }
    lua.table_mut(t).set_int(to_index(e), Value::Nil);
    Ok(vec![removed])
}

fn table_concat<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let separator = if call.arg(1).is_nil() { Vec::new() } else { check_string(lua, call, 2, "concat")? };
    let t = check_table(lua, call, 1, "concat")?;
    let i = opt_int(lua, call, 3, "concat", 1)?;
    let last = i64::try_from(lua.table(t).len()).unwrap_or(0);
    let j = opt_int(lua, call, 4, "concat", last)?;
    let mut out = Vec::new();
    for k in i..=j {
        match lua.table(t).get_int(to_index(k)) {
            Value::String(s) => out.extend_from_slice(s.as_bytes()),
            Value::Number(x) => out.extend_from_slice(number::number_to_string(x).as_bytes()),
            _ => return Err(lua.error("invalid value (at index ".to_string() + &k.to_string() + ") in table for 'concat'")),
        }
        if k != j {
            out.extend_from_slice(&separator);
        }
    }
    Ok(vec![Value::bytes(&out)])
}

struct Sorter<'a, H> {
    lua: &'a mut Lua<H>,
    host: &'a mut H,
    table: TableId,
    comparator: Value,
}

impl<H> Sorter<'_, H> {
    fn get(&self, i: i64) -> Value {
        self.lua.table(self.table).get_int(to_index(i))
    }
    fn set(&mut self, i: i64, v: Value) {
        self.lua.table_mut(self.table).set_int(to_index(i), v);
    }
    /// `sort_comp(a, b)`: `a < b`.
    fn less(&mut self, a: &Value, b: &Value) -> LuaResult<bool> {
        if self.comparator.is_nil() {
            return self.lua.less_than(self.host, a, b);
        }
        let r = self.lua.call_function(self.host, self.comparator.clone(), &[a.clone(), b.clone()], Some(1))?;
        Ok(!r[0].is_falsy())
    }
    /// `auxsort` (`0x82ee3738`).
    fn sort(&mut self, mut l: i64, mut u: i64) -> LuaResult<()> {
        while l < u {
            let (al, au) = (self.get(l), self.get(u));
            if self.less(&au, &al)? {
                self.set(l, au);
                self.set(u, al);
            }
            if u - l == 1 {
                break;
            }
            let mut i = (l + u) / 2;
            let (ai, al) = (self.get(i), self.get(l));
            if self.less(&ai, &al)? {
                self.set(i, al);
                self.set(l, ai);
            } else {
                let au = self.get(u);
                if self.less(&au, &ai)? {
                    self.set(i, au);
                    self.set(u, ai);
                }
            }
            if u - l == 2 {
                break;
            }
            let pivot = self.get(i);
            let au1 = self.get(u - 1);
            self.set(i, au1);
            self.set(u - 1, pivot.clone());
            i = l;
            let mut j = u - 1;
            loop {
                let a_i = loop {
                    i += 1;
                    let v = self.get(i);
                    if !self.less(&v, &pivot)? {
                        break v;
                    }
                    if i > u {
                        return Err(self.lua.error("invalid order function for sorting"));
                    }
                };
                let a_j = loop {
                    j -= 1;
                    let v = self.get(j);
                    if !self.less(&pivot, &v)? {
                        break v;
                    }
                    if j < l {
                        return Err(self.lua.error("invalid order function for sorting"));
                    }
                };
                if j < i {
                    break;
                }
                self.set(i, a_j);
                self.set(j, a_i);
            }
            let (au1, ai) = (self.get(u - 1), self.get(i));
            self.set(u - 1, ai);
            self.set(i, au1);
            let (lo, hi);
            if i - l < u - i {
                lo = l;
                hi = i - 1;
                l = i + 1;
            } else {
                lo = i + 1;
                hi = u;
                u = i - 1;
            }
            self.sort(lo, hi)?;
        }
        Ok(())
    }
}

fn table_sort<H>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (t, n) = aux_getn(lua, call, "sort")?;
    let comparator = call.arg(1).clone();
    if !comparator.is_nil() && !matches!(comparator, Value::Function(_)) {
        return Err(type_error(lua, call, 2, "sort", "function"));
    }
    Sorter { lua, host, table: t, comparator }.sort(1, n)?;
    Ok(Vec::new())
}

#[expect(clippy::cast_possible_truncation, reason = "C double results narrowed to lua_Number")]
fn unary_math<H>(lua: &Lua<H>, call: &NativeCall, name: &str, f: fn(f64) -> f64) -> LuaResult<Vec<Value>> {
    let x = check_number(lua, call, 1, name)?;
    Ok(vec![Value::Number(f(f64::from(x)) as f32)])
}

fn math_abs<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    unary_math(lua, call, "abs", f64::abs)
}
fn math_ceil<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    unary_math(lua, call, "ceil", f64::ceil)
}
fn math_floor<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    unary_math(lua, call, "floor", f64::floor)
}
fn math_sqrt<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    unary_math(lua, call, "sqrt", f64::sqrt)
}

#[expect(clippy::cast_possible_truncation, reason = "C double results narrowed to lua_Number")]
fn math_fmod<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let a = check_number(lua, call, 1, "fmod")?;
    let b = check_number(lua, call, 2, "fmod")?;
    Ok(vec![Value::Number((f64::from(a) % f64::from(b)) as f32)])
}

#[expect(clippy::cast_possible_truncation, reason = "C double results narrowed to lua_Number")]
fn math_pow<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let a = check_number(lua, call, 1, "pow")?;
    let b = check_number(lua, call, 2, "pow")?;
    Ok(vec![Value::Number(f64::from(a).powf(f64::from(b)) as f32)])
}

fn math_max<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let mut best = check_number(lua, call, 1, "max")?;
    for n in 2..=call.len() {
        let d = check_number(lua, call, n, "max")?;
        if d > best {
            best = d;
        }
    }
    Ok(vec![Value::Number(best)])
}

fn math_min<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let mut best = check_number(lua, call, 1, "min")?;
    for n in 2..=call.len() {
        let d = check_number(lua, call, n, "min")?;
        if d < best {
            best = d;
        }
    }
    Ok(vec![Value::Number(best)])
}
