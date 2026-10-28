//! `GI::GIShell` (contextsystem `gishell.cpp`, 0x82ab1988-0x82ab46e0): the interpreter of a
//! context system's condition and parameter scripts, `script.css` (`CSS_` chunk). Arguments,
//! locals and globals live in the loaded file, so they keep their values between runs.

/// The `CSS_` chunk tag `BindHeaderData` looks for.
const TAG_CSS: u32 = 0x4353_535F;
/// `GIHeaderChunk`, `GIScriptChunk` and `GILineChunk` sizes, and a `GIButtonChunk`'s.
const HEADER_BYTES: usize = 16;
const SCRIPT_BYTES: usize = 28;
const LINE_BYTES: usize = 8;
const BUTTON_BYTES: usize = 12;
/// `GIButtonChunk::mFuncArg1` operand kinds.
const OPERAND_GLOBAL: i16 = 5;
const OPERAND_ARGUMENT: i16 = 10;
const OPERAND_LOCAL: i16 = 15;
const OPERAND_QUERY: i16 = 25;
/// A line's `mModuleIndex`: 0 runs it, 5 stops the script, anything else skips it.
const MODULE_EXECUTE: i16 = 0;
const MODULE_STOP: i16 = 5;
/// `ExecuteLine`'s commands.
const COMMAND_SET: i16 = 5;
const COMMAND_IF: i16 = 10;
const COMMAND_ELSE_IF: i16 = 11;
const COMMAND_ELSE: i16 = 12;
const COMMAND_RETURN: i16 = 40;
/// `ExSET`'s game function marker and the bases of its variable and parameter ids.
const SET_FUNCTION: i16 = 202;
const VARIABLE_BASE: i32 = 1000;
const PARAMETER_BASE: i32 = 2000;
/// The context and context group queries (`ContextQuery::GetValue` / `GetGroupValue`).
const QUERY_CONTEXT: std::ops::RangeInclusive<i16> = 502..=504;
const QUERY_GROUP: std::ops::RangeInclusive<i16> = 512..=514;

/// What a script asks of the context system it runs in (`GI::IContextQuery`).
pub(super) trait Query {
    /// `ContextQuery::GetValue` 0x82ab1968: `QueryContext(funcIndex, contextID)`.
    fn context(&self, func: i32, id: i32) -> i32;
}

/// What a script asks of the game (`Contexts::GameInterface::GetValue`).
pub(super) trait Game {
    /// The variable's value for the `(id, value)` parameter tags.
    fn value(&mut self, variable: i32, params: &[(i32, i32)]) -> i32;
}

/// `GIButtonChunk`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Button {
    module: i16,
    func: i16,
    arg1: i16,
    arg2: i16,
    value: i32,
}

/// `GIExecContext` for the bound script: byte offsets into the file.
#[derive(Debug, Clone, Copy, Default)]
struct ExecContext {
    script: usize,
    arguments: usize,
    locals: usize,
    line: usize,
    line_size: usize,
}

/// `GI::GIShell` over its `pMemory`.
#[derive(Debug, Default)]
pub(super) struct GiShell {
    memory: Vec<u8>,
    /// `pHeader`, `pScriptOffsets` and `pGlobals`.
    header: usize,
    script_offsets: usize,
    globals: usize,
    /// `mScriptIds`: each script id's index, -1 for none.
    script_ids: Vec<i32>,
    exec: ExecContext,
    /// `mCurrentLine`, `mBranch` and `mReturnVal`.
    current_line: i32,
    branch: bool,
    return_value: i32,
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    bytes.get(at..at + 4).map_or(0, |word| u32::from_be_bytes([word[0], word[1], word[2], word[3]]))
}

fn read_i32(bytes: &[u8], at: usize) -> i32 {
    read_u32(bytes, at) as i32
}

fn read_i16(bytes: &[u8], at: usize) -> i16 {
    bytes.get(at..at + 2).map_or(0, |half| i16::from_be_bytes([half[0], half[1]]))
}

impl GiShell {
    /// `AddScriptData` 0x82ab3330 / `BindHeaderData` 0x82ab1988: the header is the last `CSS_`
    /// chunk; every script's id maps to its index (ids from 0 to the largest, -1 between).
    pub(super) fn new(memory: Vec<u8>) -> Self {
        let chunks = read_u32(&memory, 4) as usize;
        let mut header = 0;
        for chunk in 0..chunks {
            if read_u32(&memory, 8 + 8 * chunk) == TAG_CSS {
                header = read_u32(&memory, 12 + 8 * chunk) as usize;
            }
        }
        let script_offsets = header + HEADER_BYTES;
        let globals = header + read_u32(&memory, header + 12) as usize;
        let count = read_i32(&memory, header + 8).max(0) as usize;
        let ids: Vec<i32> = (0..count).map(|index| read_i32(&memory, read_u32(&memory, script_offsets + 4 * index) as usize)).collect();
        let largest = ids.iter().copied().fold(0, i32::max);
        let mut script_ids = vec![-1; largest as usize + 1];
        for (index, &id) in ids.iter().enumerate() {
            if (0..=largest).contains(&id) {
                script_ids[id as usize] = index as i32;
            }
        }
        Self { memory, header, script_offsets, globals, script_ids, ..Self::default() }
    }

    fn global_count(&self) -> i32 {
        read_i32(&self.memory, self.header + 4)
    }

    fn arg_count(&self) -> i32 {
        read_i32(&self.memory, self.exec.script + 8)
    }

    fn local_count(&self) -> i32 {
        read_i32(&self.memory, self.exec.script + 12)
    }

    /// `BindScriptData` 0x82ab1b10: the script's arguments, locals and first line.
    fn bind_script(&mut self, id: i32) -> bool {
        let Some(&index) = usize::try_from(id).ok().and_then(|id| self.script_ids.get(id)) else { return false };
        if index <= -1 {
            self.exec = ExecContext::default();
            return false;
        }
        let script = read_u32(&self.memory, self.script_offsets + 4 * index as usize) as usize;
        let arguments = script + SCRIPT_BYTES;
        let locals = arguments + 8 * read_i32(&self.memory, script + 8).max(0) as usize;
        let line = locals + 8 * read_i32(&self.memory, script + 12).max(0) as usize;
        self.exec = ExecContext { script, arguments, locals, line, line_size: read_i32(&self.memory, line).max(0) as usize };
        true
    }

    fn button(&self, index: i32) -> Button {
        let Some(at) = usize::try_from(index).ok().map(|index| self.exec.line + LINE_BYTES + BUTTON_BYTES * index) else { return Button::default() };
        Button {
            module: read_i16(&self.memory, at),
            func: read_i16(&self.memory, at + 2),
            arg1: read_i16(&self.memory, at + 4),
            arg2: read_i16(&self.memory, at + 6),
            value: read_i32(&self.memory, at + 8),
        }
    }

    fn button_count(&self) -> i32 {
        read_i32(&self.memory, self.exec.line + 4)
    }

    /// The `mValue` word of the `index`th `GIDataChunk` of the array at `base`.
    fn data_value_at(base: usize, index: i32) -> Option<usize> {
        usize::try_from(base as isize + 8 * index as isize + 4).ok()
    }

    fn data_value(&self, base: usize, index: i32) -> i32 {
        Self::data_value_at(base, index).map_or(0, |at| read_i32(&self.memory, at))
    }

    fn set_data_value(&mut self, base: usize, index: i32, value: i32) {
        if let Some(word) = Self::data_value_at(base, index).and_then(|at| self.memory.get_mut(at..at + 4)) {
            word.copy_from_slice(&value.to_be_bytes());
        }
    }

    /// `GetValue` 0x82ab1c48: a global, argument or local; a context query (the context id's
    /// high half in the button's value, its low half in `mFuncArg2`; no context group is ever
    /// loaded, so a group query answers 0); else the button's value.
    fn get_value(&self, button: i32, query: &dyn Query) -> i32 {
        let b = self.button(button);
        match b.arg1 {
            OPERAND_QUERY if QUERY_CONTEXT.contains(&b.func) => {
                let id = ((b.value as u32) << 16 | u32::from(b.arg2 as u16)) as i32;
                query.context(i32::from(b.func), id)
            }
            OPERAND_QUERY if QUERY_GROUP.contains(&b.func) => 0,
            OPERAND_GLOBAL => self.data_value(self.globals, i32::from(b.arg2)),
            OPERAND_ARGUMENT => self.data_value(self.exec.arguments, i32::from(b.arg2)),
            OPERAND_LOCAL => self.data_value(self.exec.locals, i32::from(b.arg2)),
            _ => b.value,
        }
    }

    /// `SetValue` 0x82ab1d40: a global, argument or local below its count.
    fn set_value(&mut self, button: i32, value: i32) {
        let b = self.button(button);
        let arg2 = i32::from(b.arg2);
        match b.arg1 {
            OPERAND_GLOBAL if arg2 < self.global_count() => self.set_data_value(self.globals, arg2, value),
            OPERAND_ARGUMENT if arg2 < self.arg_count() => self.set_data_value(self.exec.arguments, arg2, value),
            OPERAND_LOCAL if arg2 < self.local_count() => self.set_data_value(self.exec.locals, arg2, value),
            _ => {}
        }
    }

    /// `ComputeSetFunc` 0x82ab1de8: button 1 takes the game variable of button 3 with the
    /// `(id, value)` pairs that follow.
    fn compute_set_func(&mut self, query: &dyn Query, game: &mut dyn Game) {
        let count = (self.button_count() - 4) / 2;
        let variable = i32::from(self.button(3).func) - VARIABLE_BASE;
        let params: Vec<(i32, i32)> = (0..count.max(0)).map(|k| (i32::from(self.button(4 + 2 * k).func) - PARAMETER_BASE, self.get_value(5 + 2 * k, query))).collect();
        let value = game.value(variable, &params);
        self.set_value(1, value);
    }

    /// `ComputeOperation` 0x82ab1ed0.
    fn compute_operation(&self, button: i32, query: &dyn Query) -> i32 {
        let op = self.button(button + 1).func;
        if !(250..=279).contains(&op) {
            return 0;
        }
        if op == 251 {
            return self.get_value(button, query);
        }
        let (a, b) = (self.get_value(button, query), self.get_value(button + 2, query));
        match op {
            255 => b.wrapping_add(a),
            256 => a.wrapping_sub(b),
            257 => b.wrapping_mul(a),
            258 if b != 0 => a.wrapping_div(b),
            _ => 0,
        }
    }

    /// `ComputeCompare` 0x82ab1ff0.
    fn compute_compare(&self, button: i32, query: &dyn Query) -> bool {
        let op = self.button(button + 1).func;
        if !(300..=349).contains(&op) {
            return false;
        }
        if op >= 340 {
            let (a, lo, hi) = (self.get_value(button, query), self.get_value(button + 2, query), self.get_value(button + 3, query));
            let between = if lo > hi { a >= hi && a <= lo } else { a >= lo && a <= hi };
            return match op {
                341 => between,
                342 => !between,
                _ => false,
            };
        }
        if op >= 330 {
            let (a, b) = (self.get_value(button, query), self.get_value(button + 2, query));
            return match op {
                331 => a < b,
                332 => a <= b,
                333 => a == b,
                334 => a != b,
                335 => a >= b,
                336 => a > b,
                337 => a & b != 0,
                _ => false,
            };
        }
        let value = self.get_value(button, query);
        match op {
            310 => value < 0,
            311 => value <= 0,
            312 => value == 0,
            313 => value != 0,
            314 => value >= 0,
            315 => value > 0,
            _ => false,
        }
    }

    /// `ComputeCommand` 0x82ab3488: the operand at `button` assigned, added to, scaled by,
    /// divided by (unless 0), raised to or lowered to the operation at `button + 2`.
    fn compute_command(&mut self, button: i32, query: &dyn Query) {
        let mut value = self.get_value(button, query);
        let operand = self.compute_operation(button + 2, query);
        match self.button(button + 1).func {
            201 => value = operand,
            205 | 215 => value = value.wrapping_add(operand),
            206 | 216 => value = value.wrapping_sub(operand),
            207 => value = value.wrapping_mul(operand),
            208 if operand != 0 => value = value.wrapping_div(operand),
            220 if value < operand => value = operand,
            221 if value > operand => value = operand,
            _ => {}
        }
        self.set_value(button, value);
    }

    /// `ExecuteLine` 0x82ab4080 (`ExSET` 0x82ab3f30, `ExIF` 0x82ab33a0, `ExELSEIF` 0x82ab3408).
    /// A return moves to `mNumLines`, which nothing in the library writes; taken as 0, no later
    /// line runs.
    fn execute_line(&mut self, query: &dyn Query, game: &mut dyn Game) {
        let first = self.button(0);
        match first.func {
            COMMAND_SET => {
                if self.button(2).func == SET_FUNCTION {
                    self.compute_set_func(query, game);
                } else {
                    self.compute_command(1, query);
                }
                self.current_line += 1;
                self.branch = false;
            }
            COMMAND_IF => {
                if self.compute_compare(1, query) {
                    self.current_line += 1;
                    self.branch = false;
                } else {
                    self.current_line = i32::from(first.arg1);
                    self.branch = true;
                }
            }
            COMMAND_ELSE_IF => {
                if !self.branch {
                    self.current_line = i32::from(self.button(1).arg1);
                    self.branch = false;
                } else if self.compute_compare(1, query) {
                    self.current_line += 1;
                    self.branch = false;
                } else {
                    self.current_line = i32::from(first.arg1);
                    self.branch = true;
                }
            }
            COMMAND_ELSE => {
                self.current_line = if self.branch { self.current_line + 1 } else { i32::from(first.arg1) };
                self.branch = false;
            }
            COMMAND_RETURN => {
                self.return_value = self.get_value(1, query);
                self.current_line = 0;
                self.branch = false;
            }
            _ => {
                self.current_line += 1;
                self.branch = false;
            }
        }
    }

    /// `ExecuteScript` 0x82ab46e0: argument `i` lands in the argument its `mDefValue` names; each
    /// line runs when it is the current one. The script's return value, 0 when it is not loaded.
    pub(super) fn execute(&mut self, id: i32, params: &[i32], query: &dyn Query, game: &mut dyn Game) -> i32 {
        self.return_value = 0;
        if !self.bind_script(id) {
            return 0;
        }
        let arg_count = self.arg_count();
        for index in 0..arg_count.max(0) {
            let target = read_i32(&self.memory, self.exec.arguments + 8 * index as usize);
            if target < arg_count {
                let value = params.get(index as usize).copied().unwrap_or(0);
                self.set_data_value(self.exec.arguments, target, value);
            }
        }
        self.current_line = 0;
        let line_count = read_i32(&self.memory, self.exec.script + 20);
        for line in 0..line_count.max(0) {
            if line == self.current_line {
                match self.button(0).module {
                    MODULE_EXECUTE => self.execute_line(query, game),
                    MODULE_STOP => {}
                    _ => {
                        self.branch = false;
                        self.current_line += 1;
                    }
                }
            }
            self.exec.line += self.exec.line_size;
            self.exec.line_size = read_i32(&self.memory, self.exec.line).max(0) as usize;
        }
        self.return_value
    }
}
