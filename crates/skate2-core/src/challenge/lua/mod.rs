//! Skate 2's embedded Lua 5.1 (`lua_Number` = `float`), reconstructed for the
//! shipped `ChallengeCore`/`progressioncore` bytecode. There is no parser:
//! the game only executes precompiled chunks (`luaL_loadbuffer` of a Vault
//! blob in `cMessageHandler::Attach` `0x82ce38c8`).
//!
//! Heap objects live in arenas addressed by handles so a state is `Send` and
//! owned by one runtime. Natives receive the state and the runtime's host
//! object `H`, standing in for the native closures' `lightuserdata`
//! upvalues. Collection only runs between top-level calls; a host keeps
//! long-lived values in the registry (`luaL_ref`) as the native code does.

pub mod compiler;
mod lexer;
pub mod number;
pub mod stdlib;
pub mod table;
pub mod value;
mod vm;

#[cfg(test)]
mod tests;

pub use table::Table;
pub use value::{FunctionId, Key, LuaStr, TableId, UserDataId, Value};

use skate2_formats::lua_chunk::{self, Constant, Prototype};
use std::{any::Any, collections::BTreeMap, fmt, sync::Arc};

#[derive(Clone, Debug)]
pub struct LuaError {
    pub message: String,
    /// `chunk:pc` frames, innermost first.
    pub traceback: Vec<String>,
}

impl LuaError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), traceback: Vec::new() }
    }
}

impl fmt::Display for LuaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        for frame in &self.traceback {
            write!(f, "\n\tat {frame}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LuaError {}

pub type LuaResult<T> = Result<T, LuaError>;

/// Arguments of a native call and the closure's upvalues
/// (`lua_upvalueindex`).
pub struct NativeCall {
    pub args: Vec<Value>,
    pub upvalues: Vec<Value>,
}

impl NativeCall {
    #[must_use]
    pub fn arg(&self, index: usize) -> &Value {
        self.args.get(index).unwrap_or(&Value::Nil)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.args.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

pub type NativeFn<H> = fn(&mut Lua<H>, &mut H, &NativeCall) -> LuaResult<Vec<Value>>;

/// Executable prototype (`Proto`) with constants converted to values.
#[derive(Debug)]
pub struct Proto {
    pub code: Vec<u32>,
    pub constants: Vec<Value>,
    pub protos: Vec<Arc<Proto>>,
    pub parameter_count: usize,
    pub vararg: bool,
    pub max_stack: usize,
    pub upvalue_count: usize,
    pub source: Arc<str>,
    pub line_defined: i32,
}

impl Proto {
    fn from_chunk(p: &Prototype, name: &Arc<str>) -> Self {
        let source: Arc<str> = p
            .source
            .as_ref()
            .map_or_else(|| Arc::clone(name), |s| Arc::from(String::from_utf8_lossy(s).as_ref()));
        Self {
            code: p.code.clone(),
            constants: p
                .constants
                .iter()
                .map(|k| match k {
                    Constant::Nil => Value::Nil,
                    Constant::Boolean(b) => Value::Boolean(*b),
                    Constant::Number(n) => Value::Number(*n),
                    Constant::String(s) => Value::bytes(s),
                })
                .collect(),
            protos: p.prototypes.iter().map(|c| Arc::new(Self::from_chunk(c, &source))).collect(),
            parameter_count: usize::from(p.parameter_count),
            // VARARG_ISVARARG.
            vararg: p.vararg & 2 != 0,
            max_stack: usize::from(p.max_stack_size).max(2),
            upvalue_count: usize::from(p.upvalue_count),
            source,
            line_defined: p.line_defined,
        }
    }
}

pub(crate) enum Function<H> {
    Lua { proto: Arc<Proto>, upvalues: Vec<usize> },
    Native { function: NativeFn<H>, upvalues: Vec<Value>, name: &'static str },
}

pub(crate) enum Upvalue {
    Open(usize),
    Closed(Value),
}

/// Full userdata (`Udata`): an opaque host payload and an optional metatable.
pub struct UserData {
    pub data: Box<dyn Any + Send>,
    pub(crate) metatable: Option<TableId>,
}

struct Slab<T> {
    slots: Vec<Option<T>>,
    marks: Vec<bool>,
    free: Vec<u32>,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self { slots: Vec::new(), marks: Vec::new(), free: Vec::new() }
    }
}

impl<T> Slab<T> {
    fn insert(&mut self, value: T) -> u32 {
        if let Some(slot) = self.free.pop() {
            self.slots[slot as usize] = Some(value);
            slot
        } else {
            self.slots.push(Some(value));
            self.marks.push(false);
            u32::try_from(self.slots.len() - 1).expect("Lua heap exceeds u32 handles")
        }
    }
    fn get(&self, id: u32) -> &T {
        self.slots[id as usize].as_ref().expect("live Lua object")
    }
    fn get_mut(&mut self, id: u32) -> &mut T {
        self.slots[id as usize].as_mut().expect("live Lua object")
    }
    fn live(&self) -> usize {
        self.slots.len() - self.free.len()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FrameInfo {
    source: Arc<str>,
    pc: usize,
}

pub struct Lua<H> {
    tables: Slab<Table>,
    functions: Slab<Function<H>>,
    userdata: Slab<UserData>,
    upvalues: Slab<Upvalue>,
    pub(crate) stack: Vec<Value>,
    pub(crate) top: usize,
    pub(crate) open_upvalues: BTreeMap<usize, usize>,
    globals: TableId,
    registry: TableId,
    string_metatable: Option<TableId>,
    pub(crate) depth: usize,
    pub(crate) frames: Vec<FrameInfo>,
    allocations_since_collect: usize,
    free_references: Vec<usize>,
    /// `print` sink installed by `cLuaStateManager::CreateState`.
    pub print: Option<fn(&str)>,
}

/// `LUAI_MAXCCALLS`.
const MAX_CALL_DEPTH: usize = 200;
const COLLECT_EVERY: usize = 8192;

impl<H> Default for Lua<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H> Lua<H> {
    /// `lua_newstate` + `luaL_openlibs` (`0x82edd3f0`) for the library
    /// functions reachable from the shipped scripts.
    #[must_use]
    pub fn new() -> Self {
        let mut tables = Slab::default();
        let globals = TableId(tables.insert(Table::default()));
        let registry = TableId(tables.insert(Table::default()));
        let mut lua = Self {
            tables,
            functions: Slab::default(),
            userdata: Slab::default(),
            upvalues: Slab::default(),
            stack: vec![Value::Nil; 256],
            top: 0,
            open_upvalues: BTreeMap::new(),
            globals,
            registry,
            string_metatable: None,
            depth: 0,
            frames: Vec::new(),
            allocations_since_collect: 0,
            free_references: Vec::new(),
            print: None,
        };
        lua.table_mut(globals).set_str("_G", Value::Table(globals));
        stdlib::open(&mut lua);
        lua
    }

    #[must_use]
    pub const fn globals(&self) -> TableId {
        self.globals
    }
    #[must_use]
    pub const fn registry(&self) -> TableId {
        self.registry
    }

    #[must_use]
    pub fn table(&self, id: TableId) -> &Table {
        self.tables.get(id.0)
    }
    pub fn table_mut(&mut self, id: TableId) -> &mut Table {
        self.tables.get_mut(id.0)
    }
    pub fn create_table(&mut self) -> TableId {
        self.create_table_with(0, 0)
    }
    pub fn create_table_with(&mut self, array: usize, hash: usize) -> TableId {
        self.allocations_since_collect += 1;
        TableId(self.tables.insert(Table::with_capacity(array, hash)))
    }

    pub fn create_userdata(&mut self, data: Box<dyn Any + Send>, metatable: Option<TableId>) -> UserDataId {
        self.allocations_since_collect += 1;
        UserDataId(self.userdata.insert(UserData { data, metatable }))
    }
    #[must_use]
    pub fn userdata(&self, id: UserDataId) -> &UserData {
        self.userdata.get(id.0)
    }
    pub fn userdata_mut(&mut self, id: UserDataId) -> &mut UserData {
        self.userdata.get_mut(id.0)
    }

    /// `lua_pushcclosure`.
    pub fn create_native(&mut self, name: &'static str, function: NativeFn<H>, upvalues: Vec<Value>) -> Value {
        self.allocations_since_collect += 1;
        Value::Function(FunctionId(self.functions.insert(Function::Native { function, upvalues, name })))
    }

    pub(crate) fn create_closure(&mut self, proto: Arc<Proto>, upvalues: Vec<usize>) -> FunctionId {
        self.allocations_since_collect += 1;
        FunctionId(self.functions.insert(Function::Lua { proto, upvalues }))
    }

    /// `luaL_loadbuffer` for a binary chunk: the main function closure.
    pub fn load(&mut self, chunk: &[u8], name: &str) -> LuaResult<Value> {
        let (main, _) = lua_chunk::parse(chunk).map_err(|e| LuaError::new(format!("{name}: {e}")))?;
        let name: Arc<str> = Arc::from(name);
        let proto = Arc::new(Proto::from_chunk(&main, &name));
        if proto.upvalue_count != 0 {
            return Err(LuaError::new(format!("{name}: main function declares upvalues")));
        }
        Ok(Value::Function(self.create_closure(proto, Vec::new())))
    }

    /// `luaL_loadstring`: compile Lua source; the chunk name is the source.
    pub fn load_source(&mut self, source: &[u8], name: &str) -> LuaResult<Value> {
        let main = compiler::compile(source, name).map_err(LuaError::new)?;
        let name: Arc<str> = Arc::from(name);
        let proto = Arc::new(Proto::from_chunk(&main, &name));
        Ok(Value::Function(self.create_closure(proto, Vec::new())))
    }

    pub fn get_global(&self, name: &str) -> Value {
        self.table(self.globals).get_str(name)
    }
    pub fn set_global(&mut self, name: &str, value: Value) {
        let globals = self.globals;
        self.table_mut(globals).set_str(name, value);
    }

    /// `lua_getmetatable` for any value.
    #[must_use]
    pub fn metatable_of(&self, value: &Value) -> Option<TableId> {
        match value {
            Value::Table(t) => self.table(*t).metatable,
            Value::UserData(u) => self.userdata(*u).metatable,
            Value::String(_) => self.string_metatable,
            _ => None,
        }
    }
    pub fn set_metatable(&mut self, value: &Value, metatable: Option<TableId>) {
        match value {
            Value::Table(t) => self.table_mut(*t).metatable = metatable,
            Value::UserData(u) => self.userdata_mut(*u).metatable = metatable,
            Value::String(_) => self.string_metatable = metatable,
            _ => {}
        }
    }

    /// `luaT_gettmbyobj`.
    #[must_use]
    pub fn metamethod(&self, value: &Value, event: &str) -> Value {
        self.metatable_of(value).map_or(Value::Nil, |mt| self.table(mt).get_str(event))
    }

    /// `luaL_ref` on the registry.
    pub fn reference(&mut self, value: Value) -> usize {
        let registry = self.registry;
        let slot = self.free_references.pop().unwrap_or_else(|| self.table(registry).len() + 1);
        self.table_mut(registry).set_int(slot, value);
        slot
    }
    /// `luaL_unref`.
    pub fn unreference(&mut self, slot: usize) {
        let registry = self.registry;
        self.table_mut(registry).set_int(slot, Value::Nil);
        self.free_references.push(slot);
    }
    #[must_use]
    pub fn referenced(&self, slot: usize) -> Value {
        self.table(self.registry).get_int(slot)
    }

    /// `luaL_checkudata`-style payload access.
    #[must_use]
    pub fn userdata_payload<T: 'static>(&self, value: &Value) -> Option<&T> {
        match value {
            Value::UserData(u) => self.userdata(*u).data.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// `tostring` for values without a `__tostring` metamethod.
    #[must_use]
    pub fn display(&self, value: &Value) -> String {
        match value {
            Value::Nil => "nil".into(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => number::number_to_string(*n),
            Value::String(s) => s.to_str_lossy().into_owned(),
            Value::Table(t) => format!("table: {:08X}", t.0),
            Value::Function(f) => format!("function: {:08X}", f.0),
            Value::UserData(u) => format!("userdata: {:08X}", u.0),
            Value::LightUserData(p) => format!("userdata: {p:08X}"),
        }
    }

    /// Mark-and-sweep from the globals, registry, string metatable, live stack
    /// and open upvalues. Only valid between top-level calls.
    pub fn collect_garbage(&mut self) {
        self.collect_garbage_with_roots(&[]);
    }

    fn collect_garbage_with_roots(&mut self, roots: &[Value]) {
        if self.depth != 0 {
            return;
        }
        let mut grey: Vec<Value> = vec![Value::Table(self.globals), Value::Table(self.registry)];
        if let Some(mt) = self.string_metatable {
            grey.push(Value::Table(mt));
        }
        grey.extend(self.stack[..self.top].iter().cloned());
        grey.extend(roots.iter().cloned());
        for m in &mut self.tables.marks {
            *m = false;
        }
        for m in &mut self.functions.marks {
            *m = false;
        }
        for m in &mut self.userdata.marks {
            *m = false;
        }
        for m in &mut self.upvalues.marks {
            *m = false;
        }
        let mut grey_upvalues: Vec<usize> = self.open_upvalues.values().copied().collect();
        while !grey.is_empty() || !grey_upvalues.is_empty() {
            while let Some(u) = grey_upvalues.pop() {
                if std::mem::replace(&mut self.upvalues.marks[u], true) {
                    continue;
                }
                if let Upvalue::Closed(v) = self.upvalues.get(u32::try_from(u).expect("handle")) {
                    grey.push(v.clone());
                }
            }
            let Some(value) = grey.pop() else { continue };
            match value {
                Value::Table(t) => {
                    if std::mem::replace(&mut self.tables.marks[t.0 as usize], true) {
                        continue;
                    }
                    let table = self.tables.get(t.0);
                    if let Some(mt) = table.metatable {
                        grey.push(Value::Table(mt));
                    }
                    table.for_each_reference(|v| grey.push(v.clone()));
                }
                Value::Function(f) => {
                    if std::mem::replace(&mut self.functions.marks[f.0 as usize], true) {
                        continue;
                    }
                    match self.functions.get(f.0) {
                        Function::Lua { upvalues, .. } => grey_upvalues.extend(upvalues.iter().copied()),
                        Function::Native { upvalues, .. } => grey.extend(upvalues.iter().cloned()),
                    }
                }
                Value::UserData(u) => {
                    if std::mem::replace(&mut self.userdata.marks[u.0 as usize], true) {
                        continue;
                    }
                    if let Some(mt) = self.userdata.get(u.0).metatable {
                        grey.push(Value::Table(mt));
                    }
                }
                _ => {}
            }
        }
        fn sweep<T>(slab: &mut Slab<T>) {
            for (i, slot) in slab.slots.iter_mut().enumerate() {
                if slot.is_some() && !slab.marks[i] {
                    *slot = None;
                    slab.free.push(u32::try_from(i).expect("handle"));
                }
            }
        }
        sweep(&mut self.tables);
        sweep(&mut self.functions);
        sweep(&mut self.userdata);
        sweep(&mut self.upvalues);
        self.allocations_since_collect = 0;
    }

    /// Live objects, for tests and diagnostics.
    #[must_use]
    pub fn live_objects(&self) -> usize {
        self.tables.live() + self.functions.live() + self.userdata.live() + self.upvalues.live()
    }

    pub(crate) fn maybe_collect(&mut self) {
        self.maybe_collect_with_roots(&[]);
    }

    fn maybe_collect_with_roots(&mut self, roots: &[Value]) {
        if self.depth == 0 && self.allocations_since_collect >= COLLECT_EVERY {
            self.collect_garbage_with_roots(roots);
        }
    }

    pub(crate) fn function(&self, id: FunctionId) -> &Function<H> {
        self.functions.get(id.0)
    }

    pub(crate) fn upvalue(&self, id: usize) -> &Upvalue {
        self.upvalues.get(u32::try_from(id).expect("handle"))
    }
    pub(crate) fn upvalue_mut(&mut self, id: usize) -> &mut Upvalue {
        self.upvalues.get_mut(u32::try_from(id).expect("handle"))
    }
    pub(crate) fn create_upvalue(&mut self, upvalue: Upvalue) -> usize {
        self.upvalues.insert(upvalue) as usize
    }

    pub(crate) fn error(&self, message: impl Into<String>) -> LuaError {
        let mut e = LuaError::new(message);
        e.traceback = self.frames.iter().rev().map(|f| format!("{}:{}", f.source, f.pc)).collect();
        e
    }

    /// Native name, for diagnostics.
    #[must_use]
    pub fn function_name(&self, value: &Value) -> Option<String> {
        match value {
            Value::Function(f) => Some(match self.function(*f) {
                Function::Native { name, .. } => (*name).to_string(),
                Function::Lua { proto, .. } => format!("{}:{}", proto.source, proto.line_defined),
            }),
            _ => None,
        }
    }

    pub(crate) const fn max_call_depth() -> usize {
        MAX_CALL_DEPTH
    }
}
