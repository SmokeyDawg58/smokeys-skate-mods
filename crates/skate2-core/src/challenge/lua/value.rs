//! Lua 5.1 values (`lobject.h` `TValue`) with heap objects addressed by
//! arena handles, so a whole state is `Send` and owned by one runtime.

use std::{fmt, hash::Hash, sync::Arc};

/// Immutable Lua string bytes. Equality is by content, like interned
/// `TString` pointer equality in the native state.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LuaStr(Arc<[u8]>);

impl LuaStr {
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        Self(Arc::from(bytes))
    }
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    /// Lossless for the ASCII identifiers the shipped scripts use.
    #[must_use]
    pub fn to_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }
}

impl fmt::Debug for LuaStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_str_lossy())
    }
}

impl From<&str> for LuaStr {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableId(pub(crate) u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(pub(crate) u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UserDataId(pub(crate) u32);

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Boolean(bool),
    Number(f32),
    String(LuaStr),
    Table(TableId),
    Function(FunctionId),
    UserData(UserDataId),
    LightUserData(usize),
}

impl Value {
    /// `l_isfalse`.
    #[must_use]
    pub const fn is_falsy(&self) -> bool {
        matches!(self, Self::Nil | Self::Boolean(false))
    }
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }
    /// `luaT_typenames`.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Table(_) => "table",
            Self::Function(_) => "function",
            Self::UserData(_) | Self::LightUserData(_) => "userdata",
        }
    }
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::String(LuaStr::from(s))
    }
    #[must_use]
    pub fn bytes(s: &[u8]) -> Self {
        Self::String(LuaStr::new(s))
    }
    #[must_use]
    pub const fn as_number(&self) -> Option<f32> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
    #[must_use]
    pub const fn as_table(&self) -> Option<TableId> {
        match self {
            Self::Table(t) => Some(*t),
            _ => None,
        }
    }
    #[must_use]
    pub fn as_str(&self) -> Option<&LuaStr> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    /// `luaO_rawequalObj` (`0x8299ac00`).
    #[must_use]
    pub fn raw_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            #[expect(clippy::float_cmp, reason = "luai_numeq is C ==")]
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Table(a), Self::Table(b)) => a == b,
            (Self::Function(a), Self::Function(b)) => a == b,
            (Self::UserData(a), Self::UserData(b)) => a == b,
            (Self::LightUserData(a), Self::LightUserData(b)) => a == b,
            _ => false,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}
impl From<f32> for Value {
    fn from(n: f32) -> Self {
        Self::Number(n)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::string(s)
    }
}
impl From<TableId> for Value {
    fn from(t: TableId) -> Self {
        Self::Table(t)
    }
}

/// A non-nil, non-NaN table key. Numbers compare by `==`, so `-0` and `0`
/// share a slot, as `luaH_get` finds them equal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Boolean(bool),
    Number(u32),
    String(LuaStr),
    Table(TableId),
    Function(FunctionId),
    UserData(UserDataId),
    LightUserData(usize),
}

impl Key {
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        Some(match value {
            Value::Nil => return None,
            Value::Boolean(b) => Self::Boolean(*b),
            Value::Number(n) => {
                if n.is_nan() {
                    return None;
                }
                Self::Number(if *n == 0.0 { 0 } else { n.to_bits() })
            }
            Value::String(s) => Self::String(s.clone()),
            Value::Table(t) => Self::Table(*t),
            Value::Function(f) => Self::Function(*f),
            Value::UserData(u) => Self::UserData(*u),
            Value::LightUserData(p) => Self::LightUserData(*p),
        })
    }
    #[must_use]
    pub fn to_value(&self) -> Value {
        match self {
            Self::Boolean(b) => Value::Boolean(*b),
            Self::Number(bits) => Value::Number(f32::from_bits(*bits)),
            Self::String(s) => Value::String(s.clone()),
            Self::Table(t) => Value::Table(*t),
            Self::Function(f) => Value::Function(*f),
            Self::UserData(u) => Value::UserData(*u),
            Self::LightUserData(p) => Value::LightUserData(*p),
        }
    }
}

/// `luaH_getnum` accepts a number key only when it is an exact `int`.
#[must_use]
pub fn array_index(n: f32) -> Option<usize> {
    #[expect(clippy::cast_possible_truncation, reason = "lua_number2int then exact compare")]
    let i = n as i32;
    #[expect(clippy::cast_precision_loss, reason = "cast_num(k) == nvalue(key)")]
    #[expect(clippy::float_cmp, reason = "exact integer test")]
    let exact = i as f32 == n;
    (exact && i >= 1).then(|| usize::try_from(i).expect("positive"))
}
