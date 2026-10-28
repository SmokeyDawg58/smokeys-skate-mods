//! Lua 5.1 parser and code generator (`lparser.c`, `lcode.c`), producing
//! the same prototypes as the game's `luaL_loadstring`. The shipped
//! progression handlers are both source and bytecode; the tests compile the
//! sources and compare against the shipped functions.

use super::lexer::{Lexer, Token};
use skate2_formats::lua_chunk::{Constant, LocalVariable, Prototype};
use std::collections::HashMap;

const OP_MOVE: u32 = 0;
const OP_LOADK: u32 = 1;
const OP_LOADBOOL: u32 = 2;
const OP_LOADNIL: u32 = 3;
const OP_GETUPVAL: u32 = 4;
const OP_GETGLOBAL: u32 = 5;
const OP_GETTABLE: u32 = 6;
const OP_SETGLOBAL: u32 = 7;
const OP_SETUPVAL: u32 = 8;
const OP_SETTABLE: u32 = 9;
const OP_NEWTABLE: u32 = 10;
const OP_SELF: u32 = 11;
const OP_ADD: u32 = 12;
const OP_SUB: u32 = 13;
const OP_MUL: u32 = 14;
const OP_DIV: u32 = 15;
const OP_MOD: u32 = 16;
const OP_POW: u32 = 17;
const OP_UNM: u32 = 18;
const OP_NOT: u32 = 19;
const OP_LEN: u32 = 20;
const OP_CONCAT: u32 = 21;
const OP_JMP: u32 = 22;
const OP_EQ: u32 = 23;
const OP_LT: u32 = 24;
const OP_LE: u32 = 25;
const OP_TEST: u32 = 26;
const OP_TESTSET: u32 = 27;
const OP_CALL: u32 = 28;
const OP_TAILCALL: u32 = 29;
const OP_RETURN: u32 = 30;
const OP_FORLOOP: u32 = 31;
const OP_FORPREP: u32 = 32;
const OP_TFORLOOP: u32 = 33;
const OP_SETLIST: u32 = 34;
const OP_CLOSE: u32 = 35;
const OP_CLOSURE: u32 = 36;
const OP_VARARG: u32 = 37;

const NO_JUMP: i32 = -1;
const NO_REG: i32 = 255;
const MAXARG_SBX: i32 = 131_071;
const BITRK: i32 = 256;
const MAXINDEXRK: i32 = 255;
const LFIELDS_PER_FLUSH: i32 = 50;
const MULTRET: i32 = -1;
const MAXSTACK: usize = 250;
const LUAI_MAXCCALLS: usize = 200;
const LUAI_MAXVARS: usize = 200;
const VARARG_HASARG: u8 = 1;
const VARARG_ISVARARG: u8 = 2;
const VARARG_NEEDSARG: u8 = 4;

const fn get_op(i: u32) -> u32 {
    i & 0x3f
}
const fn get_a(i: u32) -> i32 {
    ((i >> 6) & 0xff) as i32
}
const fn get_b(i: u32) -> i32 {
    ((i >> 23) & 0x1ff) as i32
}
const fn get_c(i: u32) -> i32 {
    ((i >> 14) & 0x1ff) as i32
}
const fn get_sbx(i: u32) -> i32 {
    (i >> 14) as i32 - MAXARG_SBX
}
#[expect(clippy::cast_sign_loss, reason = "instruction fields")]
const fn create_abc(o: u32, a: i32, b: i32, c: i32) -> u32 {
    o | ((a as u32) << 6) | ((b as u32) << 23) | ((c as u32) << 14)
}
#[expect(clippy::cast_sign_loss, reason = "instruction fields")]
const fn create_abx(o: u32, a: i32, bx: u32) -> u32 {
    o | ((a as u32) << 6) | (bx << 14)
}
const fn set_a(i: u32, a: i32) -> u32 {
    (i & !(0xff << 6)) | (((a as u32) & 0xff) << 6)
}
const fn set_b(i: u32, b: i32) -> u32 {
    (i & !(0x1ff << 23)) | (((b as u32) & 0x1ff) << 23)
}
const fn set_c(i: u32, c: i32) -> u32 {
    (i & !(0x1ff << 14)) | (((c as u32) & 0x1ff) << 14)
}
#[expect(clippy::cast_sign_loss, reason = "biased sBx")]
const fn set_sbx(i: u32, sbx: i32) -> u32 {
    (i & 0x3fff) | (((sbx + MAXARG_SBX) as u32) << 14)
}
const fn set_op(i: u32, o: u32) -> u32 {
    (i & !0x3f) | o
}
/// `testTMode`.
const fn test_t_mode(op: u32) -> bool {
    matches!(op, OP_EQ | OP_LT | OP_LE | OP_TEST | OP_TESTSET | OP_TFORLOOP)
}
const fn isk(x: i32) -> bool {
    x & BITRK != 0
}

/// `luaO_int2fb`.
fn int2fb(mut x: u32) -> i32 {
    let mut e = 0;
    while x >= 16 {
        x = (x + 1) >> 1;
        e += 1;
    }
    #[expect(clippy::cast_possible_wrap, reason = "small")]
    if x < 8 { x as i32 } else { ((e + 1) << 3) | (x as i32 - 8) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Void,
    Nil,
    True,
    False,
    K,
    KNum,
    Local,
    Upval,
    Global,
    Indexed,
    Jmp,
    Relocable,
    NonReloc,
    Call,
    Vararg,
}

#[derive(Clone, Copy, Debug)]
struct Exp {
    k: Kind,
    info: i32,
    aux: i32,
    nval: f32,
    t: i32,
    f: i32,
}

impl Exp {
    const fn new(k: Kind, info: i32) -> Self {
        Self { k, info, aux: 0, nval: 0.0, t: NO_JUMP, f: NO_JUMP }
    }
    const fn has_jumps(&self) -> bool {
        self.t != self.f
    }
    const fn is_numeral(&self) -> bool {
        matches!(self.k, Kind::KNum) && self.t == NO_JUMP && self.f == NO_JUMP
    }
    const fn has_multret(&self) -> bool {
        matches!(self.k, Kind::Call | Kind::Vararg)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BinOpr {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    Ne,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOpr {
    /// `priority[]` (left, right).
    const fn priority(self) -> (u32, u32) {
        match self {
            Self::Add | Self::Sub => (6, 6),
            Self::Mul | Self::Div | Self::Mod => (7, 7),
            Self::Pow => (10, 9),
            Self::Concat => (5, 4),
            Self::Ne | Self::Eq | Self::Lt | Self::Le | Self::Gt | Self::Ge => (3, 3),
            Self::And => (2, 2),
            Self::Or => (1, 1),
        }
    }
}

const UNARY_PRIORITY: u32 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UnOpr {
    Minus,
    Not,
    Len,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ConstKey {
    Nil,
    Bool(bool),
    Num(u32),
    Str(Vec<u8>),
}

struct Block {
    breaklist: i32,
    nactvar: usize,
    upval: bool,
    isbreakable: bool,
}

struct FuncState {
    f: Prototype,
    h: HashMap<ConstKey, usize>,
    blocks: Vec<Block>,
    lasttarget: i32,
    jpc: i32,
    freereg: usize,
    nactvar: usize,
    upvalues: Vec<(Kind, i32)>,
    actvar: Vec<usize>,
}

impl FuncState {
    fn new(source: Option<Vec<u8>>) -> Self {
        Self {
            f: Prototype {
                source,
                line_defined: 0,
                last_line_defined: 0,
                upvalue_count: 0,
                parameter_count: 0,
                vararg: 0,
                max_stack_size: 2,
                code: Vec::new(),
                constants: Vec::new(),
                prototypes: Vec::new(),
                line_info: Vec::new(),
                locals: Vec::new(),
                upvalue_names: Vec::new(),
            },
            h: HashMap::new(),
            blocks: Vec::new(),
            lasttarget: -1,
            jpc: NO_JUMP,
            freereg: 0,
            nactvar: 0,
            upvalues: Vec::new(),
            actvar: Vec::new(),
        }
    }
    #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap, reason = "code size")]
    fn pc(&self) -> i32 {
        self.f.code.len() as i32
    }
}

struct Parser<'a> {
    ls: Lexer<'a>,
    fs: Vec<FuncState>,
    depth: usize,
}

type R<T> = Result<T, String>;

fn usize_of(x: i32) -> usize {
    usize::try_from(x).expect("non-negative register/pc")
}
#[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap, reason = "registers")]
const fn i32_of(x: usize) -> i32 {
    x as i32
}

impl<'a> Parser<'a> {
    fn fs(&mut self) -> &mut FuncState {
        self.fs.last_mut().expect("function state")
    }
    fn fs_ref(&self) -> &FuncState {
        self.fs.last().expect("function state")
    }

    fn error(&self, message: &str) -> String {
        format!("{} near '{}'", self.ls.error(message), self.ls.token.describe())
    }

    // ---- lcode.c -------------------------------------------------------

    fn code(&mut self, i: u32) -> i32 {
        self.discharge_jpc();
        let line = self.ls.last_line;
        let fs = self.fs();
        fs.f.code.push(i);
        fs.f.line_info.push(line);
        fs.pc() - 1
    }
    fn code_abc(&mut self, o: u32, a: i32, b: i32, c: i32) -> i32 {
        self.code(create_abc(o, a, b, c))
    }
    #[expect(clippy::cast_sign_loss, reason = "Bx")]
    fn code_abx(&mut self, o: u32, a: i32, bx: i32) -> i32 {
        self.code(create_abx(o, a, bx as u32))
    }
    fn code_asbx(&mut self, o: u32, a: i32, sbx: i32) -> i32 {
        self.code_abx(o, a, sbx + MAXARG_SBX)
    }
    fn fix_line(&mut self, line: i32) {
        let fs = self.fs();
        if let Some(last) = fs.f.line_info.last_mut() {
            *last = line;
        }
    }
    fn instr(&mut self, pc: i32) -> &mut u32 {
        &mut self.fs().f.code[usize_of(pc)]
    }

    /// `luaK_nil`.
    fn nil(&mut self, from: i32, n: i32) {
        let fs = self.fs_ref();
        if fs.pc() > fs.lasttarget {
            if fs.pc() == 0 {
                if from >= i32_of(fs.nactvar) {
                    return;
                }
            } else {
                let previous = fs.f.code[usize_of(fs.pc() - 1)];
                if get_op(previous) == OP_LOADNIL {
                    let pfrom = get_a(previous);
                    let pto = get_b(previous);
                    if pfrom <= from && from <= pto + 1 {
                        if from + n - 1 > pto {
                            let pc = fs.pc() - 1;
                            *self.instr(pc) = set_b(previous, from + n - 1);
                        }
                        return;
                    }
                }
            }
        }
        self.code_abc(OP_LOADNIL, from, from + n - 1, 0);
    }

    fn jump(&mut self) -> i32 {
        let jpc = self.fs().jpc;
        self.fs().jpc = NO_JUMP;
        let mut j = self.code_asbx(OP_JMP, 0, NO_JUMP);
        self.concat(&mut j, jpc);
        j
    }

    fn ret(&mut self, first: i32, nret: i32) {
        self.code_abc(OP_RETURN, first, nret + 1, 0);
    }

    fn cond_jump(&mut self, op: u32, a: i32, b: i32, c: i32) -> i32 {
        self.code_abc(op, a, b, c);
        self.jump()
    }

    fn fix_jump(&mut self, pc: i32, dest: i32) -> R<()> {
        let offset = dest - (pc + 1);
        if offset.abs() > MAXARG_SBX {
            return Err(self.error("control structure too long"));
        }
        let i = *self.instr(pc);
        *self.instr(pc) = set_sbx(i, offset);
        Ok(())
    }

    fn get_label(&mut self) -> i32 {
        let pc = self.fs().pc();
        self.fs().lasttarget = pc;
        pc
    }

    fn get_jump(&self, pc: i32) -> i32 {
        let offset = get_sbx(self.fs_ref().f.code[usize_of(pc)]);
        if offset == NO_JUMP { NO_JUMP } else { pc + 1 + offset }
    }

    fn jump_control(&self, pc: i32) -> i32 {
        let code = &self.fs_ref().f.code;
        if pc >= 1 && test_t_mode(get_op(code[usize_of(pc - 1)])) { pc - 1 } else { pc }
    }

    fn need_value(&self, mut list: i32) -> bool {
        while list != NO_JUMP {
            let i = self.fs_ref().f.code[usize_of(self.jump_control(list))];
            if get_op(i) != OP_TESTSET {
                return true;
            }
            list = self.get_jump(list);
        }
        false
    }

    fn patch_test_reg(&mut self, node: i32, reg: i32) -> bool {
        let at = self.jump_control(node);
        let i = *self.instr(at);
        if get_op(i) != OP_TESTSET {
            return false;
        }
        *self.instr(at) =
            if reg != NO_REG && reg != get_b(i) { set_a(i, reg) } else { create_abc(OP_TEST, get_b(i), 0, get_c(i)) };
        true
    }

    fn remove_values(&mut self, mut list: i32) {
        while list != NO_JUMP {
            self.patch_test_reg(list, NO_REG);
            list = self.get_jump(list);
        }
    }

    fn patch_list_aux(&mut self, mut list: i32, vtarget: i32, reg: i32, dtarget: i32) -> R<()> {
        while list != NO_JUMP {
            let next = self.get_jump(list);
            if self.patch_test_reg(list, reg) {
                self.fix_jump(list, vtarget)?;
            } else {
                self.fix_jump(list, dtarget)?;
            }
            list = next;
        }
        Ok(())
    }

    fn discharge_jpc(&mut self) {
        let jpc = self.fs().jpc;
        let pc = self.fs().pc();
        // fix_jump only fails beyond MAXARG_sBx, which pc cannot reach here.
        let _ = self.patch_list_aux(jpc, pc, NO_REG, pc);
        self.fs().jpc = NO_JUMP;
    }

    fn patch_list(&mut self, list: i32, target: i32) -> R<()> {
        if target == self.fs().pc() {
            self.patch_to_here(list);
            Ok(())
        } else {
            self.patch_list_aux(list, target, NO_REG, target)
        }
    }

    fn patch_to_here(&mut self, list: i32) {
        self.get_label();
        let mut jpc = self.fs().jpc;
        self.concat(&mut jpc, list);
        self.fs().jpc = jpc;
    }

    fn concat(&mut self, l1: &mut i32, l2: i32) {
        if l2 == NO_JUMP {
            return;
        }
        if *l1 == NO_JUMP {
            *l1 = l2;
            return;
        }
        let mut list = *l1;
        loop {
            let next = self.get_jump(list);
            if next == NO_JUMP {
                break;
            }
            list = next;
        }
        let _ = self.fix_jump(list, l2);
    }

    fn check_stack(&mut self, n: usize) -> R<()> {
        let newstack = self.fs().freereg + n;
        if newstack > usize::from(self.fs().f.max_stack_size) {
            if newstack >= MAXSTACK {
                return Err(self.error("function or expression too complex"));
            }
            self.fs().f.max_stack_size = u8::try_from(newstack).expect("checked");
        }
        Ok(())
    }

    fn reserve_regs(&mut self, n: usize) -> R<()> {
        self.check_stack(n)?;
        self.fs().freereg += n;
        Ok(())
    }

    fn free_reg(&mut self, reg: i32) {
        if !isk(reg) && reg >= i32_of(self.fs().nactvar) {
            self.fs().freereg -= 1;
        }
    }

    fn free_exp(&mut self, e: &Exp) {
        if e.k == Kind::NonReloc {
            self.free_reg(e.info);
        }
    }

    fn add_k(&mut self, key: ConstKey, value: Constant) -> i32 {
        let fs = self.fs();
        if let Some(&i) = fs.h.get(&key) {
            return i32_of(i);
        }
        let i = fs.f.constants.len();
        fs.f.constants.push(value);
        fs.h.insert(key, i);
        i32_of(i)
    }
    fn string_k(&mut self, s: &[u8]) -> i32 {
        self.add_k(ConstKey::Str(s.to_vec()), Constant::String(s.to_vec()))
    }
    fn number_k(&mut self, r: f32) -> i32 {
        let bits = if r == 0.0 { 0 } else { r.to_bits() };
        self.add_k(ConstKey::Num(bits), Constant::Number(r))
    }
    fn bool_k(&mut self, b: bool) -> i32 {
        self.add_k(ConstKey::Bool(b), Constant::Boolean(b))
    }
    fn nil_k(&mut self) -> i32 {
        self.add_k(ConstKey::Nil, Constant::Nil)
    }

    fn set_returns(&mut self, e: &Exp, nresults: i32) -> R<()> {
        match e.k {
            Kind::Call => {
                let i = *self.instr(e.info);
                *self.instr(e.info) = set_c(i, nresults + 1);
            }
            Kind::Vararg => {
                let i = *self.instr(e.info);
                let free = i32_of(self.fs().freereg);
                *self.instr(e.info) = set_a(set_b(i, nresults + 1), free);
                self.reserve_regs(1)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn set_one_ret(&mut self, e: &mut Exp) {
        match e.k {
            Kind::Call => {
                e.k = Kind::NonReloc;
                e.info = get_a(*self.instr(e.info));
            }
            Kind::Vararg => {
                let i = *self.instr(e.info);
                *self.instr(e.info) = set_b(i, 2);
                e.k = Kind::Relocable;
            }
            _ => {}
        }
    }

    fn discharge_vars(&mut self, e: &mut Exp) {
        match e.k {
            Kind::Local => e.k = Kind::NonReloc,
            Kind::Upval => {
                e.info = self.code_abc(OP_GETUPVAL, 0, e.info, 0);
                e.k = Kind::Relocable;
            }
            Kind::Global => {
                e.info = self.code_abx(OP_GETGLOBAL, 0, e.info);
                e.k = Kind::Relocable;
            }
            Kind::Indexed => {
                self.free_reg(e.aux);
                self.free_reg(e.info);
                e.info = self.code_abc(OP_GETTABLE, 0, e.info, e.aux);
                e.k = Kind::Relocable;
            }
            Kind::Vararg | Kind::Call => self.set_one_ret(e),
            _ => {}
        }
    }

    fn code_label(&mut self, a: i32, b: i32, jump: i32) -> i32 {
        self.get_label();
        self.code_abc(OP_LOADBOOL, a, b, jump)
    }

    fn discharge2reg(&mut self, e: &mut Exp, reg: i32) {
        self.discharge_vars(e);
        match e.k {
            Kind::Nil => self.nil(reg, 1),
            Kind::False | Kind::True => {
                self.code_abc(OP_LOADBOOL, reg, i32::from(e.k == Kind::True), 0);
            }
            Kind::K => {
                self.code_abx(OP_LOADK, reg, e.info);
            }
            Kind::KNum => {
                let k = self.number_k(e.nval);
                self.code_abx(OP_LOADK, reg, k);
            }
            Kind::Relocable => {
                let i = *self.instr(e.info);
                *self.instr(e.info) = set_a(i, reg);
            }
            Kind::NonReloc => {
                if reg != e.info {
                    self.code_abc(OP_MOVE, reg, e.info, 0);
                }
            }
            _ => return,
        }
        e.info = reg;
        e.k = Kind::NonReloc;
    }

    fn discharge2anyreg(&mut self, e: &mut Exp) -> R<()> {
        if e.k != Kind::NonReloc {
            self.reserve_regs(1)?;
            let reg = i32_of(self.fs().freereg) - 1;
            self.discharge2reg(e, reg);
        }
        Ok(())
    }

    fn exp2reg(&mut self, e: &mut Exp, reg: i32) -> R<()> {
        self.discharge2reg(e, reg);
        if e.k == Kind::Jmp {
            let mut t = e.t;
            self.concat(&mut t, e.info);
            e.t = t;
        }
        if e.has_jumps() {
            let mut p_f = NO_JUMP;
            let mut p_t = NO_JUMP;
            if self.need_value(e.t) || self.need_value(e.f) {
                let fj = if e.k == Kind::Jmp { NO_JUMP } else { self.jump() };
                p_f = self.code_label(reg, 0, 1);
                p_t = self.code_label(reg, 1, 0);
                self.patch_to_here(fj);
            }
            let final_ = self.get_label();
            self.patch_list_aux(e.f, final_, reg, p_f)?;
            self.patch_list_aux(e.t, final_, reg, p_t)?;
        }
        e.f = NO_JUMP;
        e.t = NO_JUMP;
        e.info = reg;
        e.k = Kind::NonReloc;
        Ok(())
    }

    fn exp2nextreg(&mut self, e: &mut Exp) -> R<()> {
        self.discharge_vars(e);
        self.free_exp(e);
        self.reserve_regs(1)?;
        let reg = i32_of(self.fs().freereg) - 1;
        self.exp2reg(e, reg)
    }

    fn exp2anyreg(&mut self, e: &mut Exp) -> R<i32> {
        self.discharge_vars(e);
        if e.k == Kind::NonReloc {
            if !e.has_jumps() {
                return Ok(e.info);
            }
            if e.info >= i32_of(self.fs().nactvar) {
                let reg = e.info;
                self.exp2reg(e, reg)?;
                return Ok(e.info);
            }
        }
        self.exp2nextreg(e)?;
        Ok(e.info)
    }

    fn exp2val(&mut self, e: &mut Exp) -> R<()> {
        if e.has_jumps() {
            self.exp2anyreg(e)?;
        } else {
            self.discharge_vars(e);
        }
        Ok(())
    }

    fn exp2rk(&mut self, e: &mut Exp) -> R<i32> {
        self.exp2val(e)?;
        match e.k {
            Kind::KNum | Kind::True | Kind::False | Kind::Nil => {
                if self.fs().f.constants.len() <= usize_of(MAXINDEXRK) {
                    e.info = match e.k {
                        Kind::Nil => self.nil_k(),
                        Kind::KNum => self.number_k(e.nval),
                        _ => self.bool_k(e.k == Kind::True),
                    };
                    e.k = Kind::K;
                    return Ok(e.info | BITRK);
                }
            }
            Kind::K => {
                if e.info <= MAXINDEXRK {
                    return Ok(e.info | BITRK);
                }
            }
            _ => {}
        }
        self.exp2anyreg(e)
    }

    fn store_var(&mut self, var: &Exp, ex: &mut Exp) -> R<()> {
        match var.k {
            Kind::Local => {
                self.free_exp(ex);
                return self.exp2reg(ex, var.info);
            }
            Kind::Upval => {
                let e = self.exp2anyreg(ex)?;
                self.code_abc(OP_SETUPVAL, e, var.info, 0);
            }
            Kind::Global => {
                let e = self.exp2anyreg(ex)?;
                self.code_abx(OP_SETGLOBAL, e, var.info);
            }
            Kind::Indexed => {
                let e = self.exp2rk(ex)?;
                self.code_abc(OP_SETTABLE, var.info, var.aux, e);
            }
            _ => return Err(self.error("invalid assignment target")),
        }
        self.free_exp(ex);
        Ok(())
    }

    fn self_(&mut self, e: &mut Exp, key: &mut Exp) -> R<()> {
        self.exp2anyreg(e)?;
        self.free_exp(e);
        let func = i32_of(self.fs().freereg);
        self.reserve_regs(2)?;
        let k = self.exp2rk(key)?;
        self.code_abc(OP_SELF, func, e.info, k);
        self.free_exp(key);
        e.info = func;
        e.k = Kind::NonReloc;
        Ok(())
    }

    fn invert_jump(&mut self, e: &Exp) {
        let at = self.jump_control(e.info);
        let i = *self.instr(at);
        *self.instr(at) = set_a(i, i32::from(get_a(i) == 0));
    }

    fn jump_on_cond(&mut self, e: &mut Exp, cond: i32) -> R<i32> {
        if e.k == Kind::Relocable {
            let ie = *self.instr(e.info);
            if get_op(ie) == OP_NOT {
                self.fs().f.code.pop();
                self.fs().f.line_info.pop();
                return Ok(self.cond_jump(OP_TEST, get_b(ie), 0, i32::from(cond == 0)));
            }
        }
        self.discharge2anyreg(e)?;
        self.free_exp(e);
        Ok(self.cond_jump(OP_TESTSET, NO_REG, e.info, cond))
    }

    fn go_if_true(&mut self, e: &mut Exp) -> R<()> {
        self.discharge_vars(e);
        let pc = match e.k {
            Kind::K | Kind::KNum | Kind::True => NO_JUMP,
            Kind::False => self.jump(),
            Kind::Jmp => {
                self.invert_jump(e);
                e.info
            }
            _ => self.jump_on_cond(e, 0)?,
        };
        let mut f = e.f;
        self.concat(&mut f, pc);
        e.f = f;
        self.patch_to_here(e.t);
        e.t = NO_JUMP;
        Ok(())
    }

    fn go_if_false(&mut self, e: &mut Exp) -> R<()> {
        self.discharge_vars(e);
        let pc = match e.k {
            Kind::Nil | Kind::False => NO_JUMP,
            Kind::True => self.jump(),
            Kind::Jmp => e.info,
            _ => self.jump_on_cond(e, 1)?,
        };
        let mut t = e.t;
        self.concat(&mut t, pc);
        e.t = t;
        self.patch_to_here(e.f);
        e.f = NO_JUMP;
        Ok(())
    }

    fn code_not(&mut self, e: &mut Exp) -> R<()> {
        self.discharge_vars(e);
        match e.k {
            Kind::Nil | Kind::False => e.k = Kind::True,
            Kind::K | Kind::KNum | Kind::True => e.k = Kind::False,
            Kind::Jmp => self.invert_jump(e),
            Kind::Relocable | Kind::NonReloc => {
                self.discharge2anyreg(e)?;
                self.free_exp(e);
                e.info = self.code_abc(OP_NOT, 0, e.info, 0);
                e.k = Kind::Relocable;
            }
            _ => return Err(self.error("cannot negate expression")),
        }
        std::mem::swap(&mut e.f, &mut e.t);
        self.remove_values(e.f);
        self.remove_values(e.t);
        Ok(())
    }

    fn indexed(&mut self, t: &mut Exp, k: &mut Exp) -> R<()> {
        t.aux = self.exp2rk(k)?;
        t.k = Kind::Indexed;
        Ok(())
    }

    /// `constfolding` with `float` arithmetic.
    fn const_folding(op: u32, e1: &mut Exp, e2: &Exp) -> bool {
        if !e1.is_numeral() || !e2.is_numeral() {
            return false;
        }
        let (v1, v2) = (e1.nval, e2.nval);
        #[expect(clippy::cast_possible_truncation, reason = "C double narrowed to lua_Number")]
        let r = match op {
            OP_ADD => v1 + v2,
            OP_SUB => v1 - v2,
            OP_MUL => v1 * v2,
            OP_DIV => {
                if v2 == 0.0 {
                    return false;
                }
                v1 / v2
            }
            OP_MOD => {
                if v2 == 0.0 {
                    return false;
                }
                (f64::from(v1) - f64::from(v1 / v2).floor() * f64::from(v2)) as f32
            }
            OP_POW => f64::from(v1).powf(f64::from(v2)) as f32,
            OP_UNM => -v1,
            _ => return false,
        };
        if r.is_nan() {
            return false;
        }
        e1.nval = r;
        true
    }

    fn code_arith(&mut self, op: u32, e1: &mut Exp, e2: &mut Exp) -> R<()> {
        if Self::const_folding(op, e1, e2) {
            return Ok(());
        }
        let o2 = if op != OP_UNM && op != OP_LEN { self.exp2rk(e2)? } else { 0 };
        let o1 = self.exp2rk(e1)?;
        if o1 > o2 {
            self.free_exp(e1);
            self.free_exp(e2);
        } else {
            self.free_exp(e2);
            self.free_exp(e1);
        }
        e1.info = self.code_abc(op, 0, o1, o2);
        e1.k = Kind::Relocable;
        Ok(())
    }

    fn code_comp(&mut self, op: u32, mut cond: i32, e1: &mut Exp, e2: &mut Exp) -> R<()> {
        let mut o1 = self.exp2rk(e1)?;
        let mut o2 = self.exp2rk(e2)?;
        self.free_exp(e2);
        self.free_exp(e1);
        if cond == 0 && op != OP_EQ {
            std::mem::swap(&mut o1, &mut o2);
            cond = 1;
        }
        e1.info = self.cond_jump(op, cond, o1, o2);
        e1.k = Kind::Jmp;
        Ok(())
    }

    fn prefix(&mut self, op: UnOpr, e: &mut Exp) -> R<()> {
        let mut e2 = Exp::new(Kind::KNum, 0);
        match op {
            UnOpr::Minus => {
                if !e.is_numeral() {
                    self.exp2anyreg(e)?;
                }
                self.code_arith(OP_UNM, e, &mut e2)
            }
            UnOpr::Not => self.code_not(e),
            UnOpr::Len => {
                self.exp2anyreg(e)?;
                self.code_arith(OP_LEN, e, &mut e2)
            }
        }
    }

    fn infix(&mut self, op: BinOpr, v: &mut Exp) -> R<()> {
        match op {
            BinOpr::And => self.go_if_true(v),
            BinOpr::Or => self.go_if_false(v),
            BinOpr::Concat => self.exp2nextreg(v),
            BinOpr::Add | BinOpr::Sub | BinOpr::Mul | BinOpr::Div | BinOpr::Mod | BinOpr::Pow => {
                if !v.is_numeral() {
                    self.exp2rk(v)?;
                }
                Ok(())
            }
            _ => {
                self.exp2rk(v)?;
                Ok(())
            }
        }
    }

    fn posfix(&mut self, op: BinOpr, e1: &mut Exp, e2: &mut Exp) -> R<()> {
        match op {
            BinOpr::And => {
                self.discharge_vars(e2);
                let mut f = e2.f;
                self.concat(&mut f, e1.f);
                e2.f = f;
                *e1 = *e2;
                Ok(())
            }
            BinOpr::Or => {
                self.discharge_vars(e2);
                let mut t = e2.t;
                self.concat(&mut t, e1.t);
                e2.t = t;
                *e1 = *e2;
                Ok(())
            }
            BinOpr::Concat => {
                self.exp2val(e2)?;
                if e2.k == Kind::Relocable && get_op(*self.instr(e2.info)) == OP_CONCAT {
                    self.free_exp(e1);
                    let i = *self.instr(e2.info);
                    *self.instr(e2.info) = set_b(i, e1.info);
                    e1.k = Kind::Relocable;
                    e1.info = e2.info;
                    Ok(())
                } else {
                    self.exp2nextreg(e2)?;
                    self.code_arith(OP_CONCAT, e1, e2)
                }
            }
            BinOpr::Add => self.code_arith(OP_ADD, e1, e2),
            BinOpr::Sub => self.code_arith(OP_SUB, e1, e2),
            BinOpr::Mul => self.code_arith(OP_MUL, e1, e2),
            BinOpr::Div => self.code_arith(OP_DIV, e1, e2),
            BinOpr::Mod => self.code_arith(OP_MOD, e1, e2),
            BinOpr::Pow => self.code_arith(OP_POW, e1, e2),
            BinOpr::Eq => self.code_comp(OP_EQ, 1, e1, e2),
            BinOpr::Ne => self.code_comp(OP_EQ, 0, e1, e2),
            BinOpr::Lt => self.code_comp(OP_LT, 1, e1, e2),
            BinOpr::Le => self.code_comp(OP_LE, 1, e1, e2),
            BinOpr::Gt => self.code_comp(OP_LT, 0, e1, e2),
            BinOpr::Ge => self.code_comp(OP_LE, 0, e1, e2),
        }
    }

    fn set_list(&mut self, base: i32, nelems: i32, tostore: i32) {
        let c = (nelems - 1) / LFIELDS_PER_FLUSH + 1;
        let b = if tostore == MULTRET { 0 } else { tostore };
        if c <= 511 {
            self.code_abc(OP_SETLIST, base, b, c);
        } else {
            self.code_abc(OP_SETLIST, base, b, 0);
            #[expect(clippy::cast_sign_loss, reason = "positive count")]
            self.code(c as u32);
        }
        self.fs().freereg = usize_of(base + 1);
    }

    // ---- lparser.c -----------------------------------------------------

    fn next(&mut self) -> R<()> {
        self.ls.next()
    }
    fn test_next(&mut self, t: &Token) -> R<bool> {
        if &self.ls.token == t {
            self.next()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn check(&self, t: &Token) -> R<()> {
        if &self.ls.token == t { Ok(()) } else { Err(self.error(&format!("'{}' expected", t.describe()))) }
    }
    fn check_next(&mut self, t: &Token) -> R<()> {
        self.check(t)?;
        self.next()
    }
    fn check_match(&mut self, what: &Token, who: &Token, line: i32) -> R<()> {
        if self.test_next(what)? {
            return Ok(());
        }
        if line == self.ls.line {
            Err(self.error(&format!("'{}' expected", what.describe())))
        } else {
            Err(self.error(&format!("'{}' expected (to close '{}' at line {line})", what.describe(), who.describe())))
        }
    }
    fn check_name(&mut self) -> R<Vec<u8>> {
        let Token::Name(name) = &self.ls.token else {
            return Err(self.error("<name> expected"));
        };
        let name = name.clone();
        self.next()?;
        Ok(name)
    }
    fn code_string(&mut self, s: &[u8]) -> Exp {
        let k = self.string_k(s);
        Exp::new(Kind::K, k)
    }

    fn enter_level(&mut self) -> R<()> {
        self.depth += 1;
        if self.depth > LUAI_MAXCCALLS {
            return Err(self.ls.error("chunk has too many syntax levels"));
        }
        Ok(())
    }

    fn register_local(&mut self, name: Vec<u8>) -> usize {
        let fs = self.fs();
        fs.f.locals.push(LocalVariable { name, start_pc: 0, end_pc: 0 });
        fs.f.locals.len() - 1
    }

    fn new_local(&mut self, name: Vec<u8>, n: usize) -> R<()> {
        if self.fs().nactvar + n + 1 > LUAI_MAXVARS {
            return Err(self.error("too many local variables"));
        }
        let index = self.register_local(name);
        let fs = self.fs();
        let slot = fs.nactvar + n;
        if fs.actvar.len() <= slot {
            fs.actvar.resize(slot + 1, 0);
        }
        fs.actvar[slot] = index;
        Ok(())
    }

    fn adjust_locals(&mut self, nvars: usize) {
        let fs = self.fs();
        fs.nactvar += nvars;
        let pc = fs.pc();
        for i in (fs.nactvar - nvars)..fs.nactvar {
            let local = fs.actvar[i];
            fs.f.locals[local].start_pc = pc;
        }
    }

    fn remove_vars(&mut self, to_level: usize) {
        let fs = self.fs();
        while fs.nactvar > to_level {
            fs.nactvar -= 1;
            let pc = fs.pc();
            let local = fs.actvar[fs.nactvar];
            fs.f.locals[local].end_pc = pc;
        }
    }

    fn local_name(&self, depth: usize, i: usize) -> &[u8] {
        let fs = &self.fs[depth];
        &fs.f.locals[fs.actvar[i]].name
    }

    fn search_var(&self, depth: usize, name: &[u8]) -> Option<usize> {
        (0..self.fs[depth].nactvar).rev().find(|&i| self.local_name(depth, i) == name)
    }

    fn mark_upval(&mut self, depth: usize, level: usize) {
        let blocks = &mut self.fs[depth].blocks;
        if let Some(block) = blocks.iter_mut().rev().find(|b| b.nactvar <= level) {
            block.upval = true;
        }
    }

    fn index_upvalue(&mut self, depth: usize, name: &[u8], v: &Exp) -> R<i32> {
        let fs = &mut self.fs[depth];
        if let Some(i) = fs.upvalues.iter().position(|&(k, info)| k == v.k && info == v.info) {
            return Ok(i32_of(i));
        }
        if fs.upvalues.len() + 1 > 60 {
            return Err(self.ls.error("too many upvalues"));
        }
        let fs = &mut self.fs[depth];
        fs.f.upvalue_names.push(name.to_vec());
        fs.upvalues.push((v.k, v.info));
        fs.f.upvalue_count += 1;
        Ok(i32_of(fs.upvalues.len() - 1))
    }

    /// `singlevaraux`.
    fn single_var_aux(&mut self, depth: Option<usize>, name: &[u8], var: &mut Exp, base: bool) -> R<Kind> {
        let Some(depth) = depth else {
            *var = Exp::new(Kind::Global, NO_REG);
            return Ok(Kind::Global);
        };
        if let Some(v) = self.search_var(depth, name) {
            *var = Exp::new(Kind::Local, i32_of(v));
            if !base {
                self.mark_upval(depth, v);
            }
            return Ok(Kind::Local);
        }
        let parent = depth.checked_sub(1);
        if self.single_var_aux(parent, name, var, false)? == Kind::Global {
            return Ok(Kind::Global);
        }
        var.info = self.index_upvalue(depth, name, var)?;
        var.k = Kind::Upval;
        Ok(Kind::Upval)
    }

    fn single_var(&mut self) -> R<Exp> {
        let name = self.check_name()?;
        let mut var = Exp::new(Kind::Void, 0);
        let depth = self.fs.len() - 1;
        if self.single_var_aux(Some(depth), &name, &mut var, true)? == Kind::Global {
            var.info = self.string_k(&name);
        }
        Ok(var)
    }

    fn adjust_assign(&mut self, nvars: i32, nexps: i32, e: &mut Exp) -> R<()> {
        let mut extra = nvars - nexps;
        if e.has_multret() {
            extra += 1;
            if extra < 0 {
                extra = 0;
            }
            self.set_returns(e, extra)?;
            if extra > 1 {
                self.reserve_regs(usize_of(extra - 1))?;
            }
        } else {
            if e.k != Kind::Void {
                self.exp2nextreg(e)?;
            }
            if extra > 0 {
                let reg = i32_of(self.fs().freereg);
                self.reserve_regs(usize_of(extra))?;
                self.nil(reg, extra);
            }
        }
        Ok(())
    }

    fn enter_block(&mut self, isbreakable: bool) {
        let fs = self.fs();
        let nactvar = fs.nactvar;
        fs.blocks.push(Block { breaklist: NO_JUMP, nactvar, upval: false, isbreakable });
    }

    fn leave_block(&mut self) -> R<()> {
        let block = self.fs().blocks.pop().expect("block");
        self.remove_vars(block.nactvar);
        if block.upval {
            self.code_abc(OP_CLOSE, i32_of(block.nactvar), 0, 0);
        }
        self.fs().freereg = self.fs().nactvar;
        self.patch_to_here(block.breaklist);
        Ok(())
    }

    fn open_func(&mut self) {
        let source = self.fs.first().and_then(|fs| fs.f.source.clone());
        self.fs.push(FuncState::new(source));
    }

    fn close_func(&mut self) -> Prototype {
        self.remove_vars(0);
        self.ret(0, 0);
        let fs = self.fs.pop().expect("function state");
        fs.f
    }

    fn push_closure(&mut self, func: Prototype, upvalues: &[(Kind, i32)]) -> Exp {
        let fs = self.fs();
        fs.f.prototypes.push(func);
        let index = i32_of(fs.f.prototypes.len() - 1);
        let e = Exp::new(Kind::Relocable, self.code_abx(OP_CLOSURE, 0, index));
        for &(k, info) in upvalues {
            let o = if k == Kind::Local { OP_MOVE } else { OP_GETUPVAL };
            self.code_abc(o, 0, info, 0);
        }
        e
    }

    fn field(&mut self, v: &mut Exp) -> R<()> {
        self.exp2anyreg(v)?;
        self.next()?;
        let name = self.check_name()?;
        let mut key = self.code_string(&name);
        self.indexed(v, &mut key)
    }

    fn yindex(&mut self) -> R<Exp> {
        self.next()?;
        let mut v = self.expr()?;
        self.exp2val(&mut v)?;
        self.check_next(&Token::Char(b']'))?;
        Ok(v)
    }

    fn rec_field(&mut self, table: &Exp, nh: &mut i32) -> R<()> {
        let reg = self.fs().freereg;
        let mut key = if let Token::Name(name) = &self.ls.token {
            let name = name.clone();
            self.next()?;
            self.code_string(&name)
        } else {
            self.yindex()?
        };
        *nh += 1;
        self.check_next(&Token::Char(b'='))?;
        let rkkey = self.exp2rk(&mut key)?;
        let mut val = self.expr()?;
        let rkval = self.exp2rk(&mut val)?;
        self.code_abc(OP_SETTABLE, table.info, rkkey, rkval);
        self.fs().freereg = reg;
        Ok(())
    }

    fn constructor(&mut self) -> R<Exp> {
        let line = self.ls.line;
        let pc = self.code_abc(OP_NEWTABLE, 0, 0, 0);
        let (mut na, mut nh, mut tostore) = (0i32, 0i32, 0i32);
        let mut t = Exp::new(Kind::Relocable, pc);
        let mut v = Exp::new(Kind::Void, 0);
        self.exp2nextreg(&mut t)?;
        self.check_next(&Token::Char(b'{'))?;
        loop {
            if self.ls.token == Token::Char(b'}') {
                break;
            }
            // closelistfield
            if v.k != Kind::Void {
                self.exp2nextreg(&mut v)?;
                v.k = Kind::Void;
                if tostore == LFIELDS_PER_FLUSH {
                    self.set_list(t.info, na, tostore);
                    tostore = 0;
                }
            }
            let is_record = match &self.ls.token {
                Token::Name(_) => self.ls.peek()? == &Token::Char(b'='),
                Token::Char(b'[') => true,
                _ => false,
            };
            if is_record {
                self.rec_field(&t, &mut nh)?;
            } else {
                v = self.expr()?;
                na += 1;
                tostore += 1;
            }
            if !(self.test_next(&Token::Char(b','))? || self.test_next(&Token::Char(b';'))?) {
                break;
            }
        }
        self.check_match(&Token::Char(b'}'), &Token::Char(b'{'), line)?;
        // lastlistfield
        if tostore != 0 {
            if v.has_multret() {
                self.set_returns(&v, MULTRET)?;
                self.set_list(t.info, na, MULTRET);
                na -= 1;
            } else {
                if v.k != Kind::Void {
                    self.exp2nextreg(&mut v)?;
                }
                self.set_list(t.info, na, tostore);
            }
        }
        #[expect(clippy::cast_sign_loss, reason = "counts")]
        {
            let i = *self.instr(pc);
            *self.instr(pc) = set_c(set_b(i, int2fb(na as u32)), int2fb(nh as u32));
        }
        Ok(t)
    }

    fn par_list(&mut self) -> R<()> {
        let mut nparams = 0;
        self.fs().f.vararg = 0;
        if self.ls.token != Token::Char(b')') {
            loop {
                match &self.ls.token {
                    Token::Name(_) => {
                        let name = self.check_name()?;
                        self.new_local(name, nparams)?;
                        nparams += 1;
                    }
                    Token::Dots => {
                        self.next()?;
                        self.new_local(b"arg".to_vec(), nparams)?;
                        nparams += 1;
                        self.fs().f.vararg = VARARG_HASARG | VARARG_NEEDSARG | VARARG_ISVARARG;
                    }
                    _ => return Err(self.error("<name> or '...' expected")),
                }
                if self.fs().f.vararg != 0 || !self.test_next(&Token::Char(b','))? {
                    break;
                }
            }
        }
        self.adjust_locals(nparams);
        let fs = self.fs();
        fs.f.parameter_count = u8::try_from(fs.nactvar - usize::from(fs.f.vararg & VARARG_HASARG)).expect("params");
        let n = fs.nactvar;
        self.reserve_regs(n)
    }

    fn body(&mut self, needself: bool, line: i32) -> R<Exp> {
        self.open_func();
        self.fs().f.line_defined = line;
        self.check_next(&Token::Char(b'('))?;
        if needself {
            self.new_local(b"self".to_vec(), 0)?;
            self.adjust_locals(1);
        }
        self.par_list()?;
        self.check_next(&Token::Char(b')'))?;
        self.chunk()?;
        let last = self.ls.line;
        self.fs().f.last_line_defined = last;
        self.check_match(&Token::End, &Token::Function, line)?;
        let upvalues = self.fs().upvalues.clone();
        let func = self.close_func();
        Ok(self.push_closure(func, &upvalues))
    }

    fn exp_list1(&mut self) -> R<(i32, Exp)> {
        let mut n = 1;
        let mut v = self.expr()?;
        while self.test_next(&Token::Char(b','))? {
            self.exp2nextreg(&mut v)?;
            v = self.expr()?;
            n += 1;
        }
        Ok((n, v))
    }

    fn func_args(&mut self, f: &mut Exp) -> R<()> {
        let line = self.ls.line;
        let mut args = match &self.ls.token {
            Token::Char(b'(') => {
                if line != self.ls.last_line {
                    return Err(self.error("ambiguous syntax (function call x new statement)"));
                }
                self.next()?;
                let args = if self.ls.token == Token::Char(b')') {
                    Exp::new(Kind::Void, 0)
                } else {
                    let (_, args) = self.exp_list1()?;
                    self.set_returns(&args, MULTRET)?;
                    args
                };
                self.check_match(&Token::Char(b')'), &Token::Char(b'('), line)?;
                args
            }
            Token::Char(b'{') => self.constructor()?,
            Token::String(s) => {
                let s = s.clone();
                let e = self.code_string(&s);
                self.next()?;
                e
            }
            _ => return Err(self.error("function arguments expected")),
        };
        let base = f.info;
        let nparams = if args.has_multret() {
            MULTRET
        } else {
            if args.k != Kind::Void {
                self.exp2nextreg(&mut args)?;
            }
            i32_of(self.fs().freereg) - (base + 1)
        };
        *f = Exp::new(Kind::Call, self.code_abc(OP_CALL, base, nparams + 1, 2));
        self.fix_line(line);
        self.fs().freereg = usize_of(base + 1);
        Ok(())
    }

    fn prefix_exp(&mut self) -> R<Exp> {
        match &self.ls.token {
            Token::Char(b'(') => {
                let line = self.ls.line;
                self.next()?;
                let mut v = self.expr()?;
                self.check_match(&Token::Char(b')'), &Token::Char(b'('), line)?;
                self.discharge_vars(&mut v);
                Ok(v)
            }
            Token::Name(_) => self.single_var(),
            _ => Err(self.error("unexpected symbol")),
        }
    }

    fn primary_exp(&mut self) -> R<Exp> {
        let mut v = self.prefix_exp()?;
        loop {
            match &self.ls.token {
                Token::Char(b'.') => self.field(&mut v)?,
                Token::Char(b'[') => {
                    self.exp2anyreg(&mut v)?;
                    let mut key = self.yindex()?;
                    self.indexed(&mut v, &mut key)?;
                }
                Token::Char(b':') => {
                    self.next()?;
                    let name = self.check_name()?;
                    let mut key = self.code_string(&name);
                    self.self_(&mut v, &mut key)?;
                    self.func_args(&mut v)?;
                }
                Token::Char(b'(' | b'{') | Token::String(_) => {
                    self.exp2nextreg(&mut v)?;
                    self.func_args(&mut v)?;
                }
                _ => return Ok(v),
            }
        }
    }

    fn simple_exp(&mut self) -> R<Exp> {
        let v = match &self.ls.token {
            Token::Number(n) => {
                let mut e = Exp::new(Kind::KNum, 0);
                e.nval = *n;
                e
            }
            Token::String(s) => {
                let s = s.clone();
                self.code_string(&s)
            }
            Token::Nil => Exp::new(Kind::Nil, 0),
            Token::True => Exp::new(Kind::True, 0),
            Token::False => Exp::new(Kind::False, 0),
            Token::Dots => {
                if self.fs().f.vararg == 0 {
                    return Err(self.error("cannot use '...' outside a vararg function"));
                }
                self.fs().f.vararg &= !VARARG_NEEDSARG;
                Exp::new(Kind::Vararg, self.code_abc(OP_VARARG, 0, 1, 0))
            }
            Token::Char(b'{') => return self.constructor(),
            Token::Function => {
                self.next()?;
                let line = self.ls.line;
                return self.body(false, line);
            }
            _ => return self.primary_exp(),
        };
        self.next()?;
        Ok(v)
    }

    fn unopr(&self) -> Option<UnOpr> {
        match &self.ls.token {
            Token::Not => Some(UnOpr::Not),
            Token::Char(b'-') => Some(UnOpr::Minus),
            Token::Char(b'#') => Some(UnOpr::Len),
            _ => None,
        }
    }

    fn binopr(&self) -> Option<BinOpr> {
        Some(match &self.ls.token {
            Token::Char(b'+') => BinOpr::Add,
            Token::Char(b'-') => BinOpr::Sub,
            Token::Char(b'*') => BinOpr::Mul,
            Token::Char(b'/') => BinOpr::Div,
            Token::Char(b'%') => BinOpr::Mod,
            Token::Char(b'^') => BinOpr::Pow,
            Token::Concat => BinOpr::Concat,
            Token::Ne => BinOpr::Ne,
            Token::Eq => BinOpr::Eq,
            Token::Char(b'<') => BinOpr::Lt,
            Token::Le => BinOpr::Le,
            Token::Char(b'>') => BinOpr::Gt,
            Token::Ge => BinOpr::Ge,
            Token::And => BinOpr::And,
            Token::Or => BinOpr::Or,
            _ => return None,
        })
    }

    fn subexpr(&mut self, limit: u32) -> R<(Exp, Option<BinOpr>)> {
        self.enter_level()?;
        let mut v = if let Some(uop) = self.unopr() {
            self.next()?;
            let (mut v, _) = self.subexpr(UNARY_PRIORITY)?;
            self.prefix(uop, &mut v)?;
            v
        } else {
            self.simple_exp()?
        };
        let mut op = self.binopr();
        while let Some(o) = op {
            if o.priority().0 <= limit {
                break;
            }
            self.next()?;
            self.infix(o, &mut v)?;
            let (mut v2, nextop) = self.subexpr(o.priority().1)?;
            self.posfix(o, &mut v, &mut v2)?;
            op = nextop;
        }
        self.depth -= 1;
        Ok((v, op))
    }

    fn expr(&mut self) -> R<Exp> {
        Ok(self.subexpr(0)?.0)
    }

    fn block_follow(&self) -> bool {
        matches!(self.ls.token, Token::Else | Token::ElseIf | Token::End | Token::Until | Token::Eos)
    }

    fn block(&mut self) -> R<()> {
        self.enter_block(false);
        self.chunk()?;
        self.leave_block()
    }

    fn check_conflict(&mut self, lhs: &mut [Exp], v: &Exp) -> R<()> {
        let extra = i32_of(self.fs().freereg);
        let mut conflict = false;
        for lh in lhs.iter_mut().rev() {
            if lh.k == Kind::Indexed {
                if lh.info == v.info {
                    conflict = true;
                    lh.info = extra;
                }
                if lh.aux == v.info {
                    conflict = true;
                    lh.aux = extra;
                }
            }
        }
        if conflict {
            let free = i32_of(self.fs().freereg);
            self.code_abc(OP_MOVE, free, v.info, 0);
            self.reserve_regs(1)?;
        }
        Ok(())
    }

    /// `assignment`, with the left-hand chain kept in `lhs` (innermost last).
    fn assignment(&mut self, lhs: &mut Vec<Exp>, nvars: i32) -> R<()> {
        let last = *lhs.last().expect("target");
        if !matches!(last.k, Kind::Local | Kind::Upval | Kind::Global | Kind::Indexed) {
            return Err(self.error("syntax error"));
        }
        let mut e;
        if self.test_next(&Token::Char(b','))? {
            let nv = self.primary_exp()?;
            if nv.k == Kind::Local {
                self.check_conflict(lhs, &nv)?;
            }
            lhs.push(nv);
            self.assignment(lhs, nvars + 1)?;
            lhs.pop();
        } else {
            self.check_next(&Token::Char(b'='))?;
            let (nexps, mut ex) = self.exp_list1()?;
            if nexps == nvars {
                self.set_one_ret(&mut ex);
                let target = *lhs.last().expect("target");
                return self.store_var(&target, &mut ex);
            }
            self.adjust_assign(nvars, nexps, &mut ex)?;
            if nexps > nvars {
                let fs = self.fs();
                fs.freereg -= usize_of(nexps - nvars);
            }
        }
        e = Exp::new(Kind::NonReloc, i32_of(self.fs().freereg) - 1);
        let target = *lhs.last().expect("target");
        self.store_var(&target, &mut e)
    }

    fn cond(&mut self) -> R<i32> {
        let mut v = self.expr()?;
        if v.k == Kind::Nil {
            v.k = Kind::False;
        }
        self.go_if_true(&mut v)?;
        Ok(v.f)
    }

    fn break_stat(&mut self) -> R<()> {
        let fs = self.fs();
        let mut upval = false;
        let mut index = fs.blocks.len();
        while index > 0 && !fs.blocks[index - 1].isbreakable {
            upval |= fs.blocks[index - 1].upval;
            index -= 1;
        }
        if index == 0 {
            return Err(self.error("no loop to break"));
        }
        let nactvar = self.fs().blocks[index - 1].nactvar;
        if upval {
            self.code_abc(OP_CLOSE, i32_of(nactvar), 0, 0);
        }
        let j = self.jump();
        let mut list = self.fs().blocks[index - 1].breaklist;
        self.concat(&mut list, j);
        self.fs().blocks[index - 1].breaklist = list;
        Ok(())
    }

    fn while_stat(&mut self, line: i32) -> R<()> {
        self.next()?;
        let whileinit = self.get_label();
        let condexit = self.cond()?;
        self.enter_block(true);
        self.check_next(&Token::Do)?;
        self.block()?;
        let j = self.jump();
        self.patch_list(j, whileinit)?;
        self.check_match(&Token::End, &Token::While, line)?;
        self.leave_block()?;
        self.patch_to_here(condexit);
        Ok(())
    }

    fn repeat_stat(&mut self, line: i32) -> R<()> {
        let repeat_init = self.get_label();
        self.enter_block(true);
        self.enter_block(false);
        self.next()?;
        self.chunk()?;
        self.check_match(&Token::Until, &Token::Repeat, line)?;
        let condexit = self.cond()?;
        if self.fs().blocks.last().expect("scope").upval {
            self.break_stat()?;
            self.patch_to_here(condexit);
            self.leave_block()?;
            let j = self.jump();
            self.patch_list(j, repeat_init)?;
        } else {
            self.leave_block()?;
            self.patch_list(condexit, repeat_init)?;
        }
        self.leave_block()
    }

    fn exp1(&mut self) -> R<()> {
        let mut e = self.expr()?;
        self.exp2nextreg(&mut e)
    }

    fn for_body(&mut self, base: i32, line: i32, nvars: usize, isnum: bool) -> R<()> {
        self.adjust_locals(3);
        self.check_next(&Token::Do)?;
        let prep = if isnum { self.code_asbx(OP_FORPREP, base, NO_JUMP) } else { self.jump() };
        self.enter_block(false);
        self.adjust_locals(nvars);
        self.reserve_regs(nvars)?;
        self.block()?;
        self.leave_block()?;
        self.patch_to_here(prep);
        let endfor = if isnum {
            self.code_asbx(OP_FORLOOP, base, NO_JUMP)
        } else {
            self.code_abc(OP_TFORLOOP, base, 0, i32_of(nvars))
        };
        self.fix_line(line);
        let list = if isnum { endfor } else { self.jump() };
        self.patch_list(list, prep + 1)
    }

    fn for_num(&mut self, varname: Vec<u8>, line: i32) -> R<()> {
        let base = i32_of(self.fs().freereg);
        self.new_local(b"(for index)".to_vec(), 0)?;
        self.new_local(b"(for limit)".to_vec(), 1)?;
        self.new_local(b"(for step)".to_vec(), 2)?;
        self.new_local(varname, 3)?;
        self.check_next(&Token::Char(b'='))?;
        self.exp1()?;
        self.check_next(&Token::Char(b','))?;
        self.exp1()?;
        if self.test_next(&Token::Char(b','))? {
            self.exp1()?;
        } else {
            let free = i32_of(self.fs().freereg);
            let k = self.number_k(1.0);
            self.code_abx(OP_LOADK, free, k);
            self.reserve_regs(1)?;
        }
        self.for_body(base, line, 1, true)
    }

    fn for_list(&mut self, indexname: Vec<u8>) -> R<()> {
        let base = i32_of(self.fs().freereg);
        let mut nvars = 0;
        for name in [&b"(for generator)"[..], b"(for state)", b"(for control)"] {
            self.new_local(name.to_vec(), nvars)?;
            nvars += 1;
        }
        self.new_local(indexname, nvars)?;
        nvars += 1;
        while self.test_next(&Token::Char(b','))? {
            let name = self.check_name()?;
            self.new_local(name, nvars)?;
            nvars += 1;
        }
        self.check_next(&Token::In)?;
        let line = self.ls.line;
        let (nexps, mut e) = self.exp_list1()?;
        self.adjust_assign(3, nexps, &mut e)?;
        self.check_stack(3)?;
        self.for_body(base, line, nvars - 3, false)
    }

    fn for_stat(&mut self, line: i32) -> R<()> {
        self.enter_block(true);
        self.next()?;
        let varname = self.check_name()?;
        match &self.ls.token {
            Token::Char(b'=') => self.for_num(varname, line)?,
            Token::Char(b',') | Token::In => self.for_list(varname)?,
            _ => return Err(self.error("'=' or 'in' expected")),
        }
        self.check_match(&Token::End, &Token::For, line)?;
        self.leave_block()
    }

    fn test_then_block(&mut self) -> R<i32> {
        self.next()?;
        let condexit = self.cond()?;
        self.check_next(&Token::Then)?;
        self.block()?;
        Ok(condexit)
    }

    fn if_stat(&mut self, line: i32) -> R<()> {
        let mut escapelist = NO_JUMP;
        let mut flist = self.test_then_block()?;
        while self.ls.token == Token::ElseIf {
            let j = self.jump();
            self.concat(&mut escapelist, j);
            self.patch_to_here(flist);
            flist = self.test_then_block()?;
        }
        if self.ls.token == Token::Else {
            let j = self.jump();
            self.concat(&mut escapelist, j);
            self.patch_to_here(flist);
            self.next()?;
            self.block()?;
        } else {
            self.concat(&mut escapelist, flist);
        }
        self.patch_to_here(escapelist);
        self.check_match(&Token::End, &Token::If, line)
    }

    fn local_func(&mut self) -> R<()> {
        let name = self.check_name()?;
        self.new_local(name, 0)?;
        let v = Exp::new(Kind::Local, i32_of(self.fs().freereg));
        self.reserve_regs(1)?;
        self.adjust_locals(1);
        let line = self.ls.line;
        let mut b = self.body(false, line)?;
        self.store_var(&v, &mut b)?;
        let fs = self.fs();
        let pc = fs.pc();
        let local = fs.actvar[fs.nactvar - 1];
        fs.f.locals[local].start_pc = pc;
        Ok(())
    }

    fn local_stat(&mut self) -> R<()> {
        let mut nvars = 0;
        loop {
            let name = self.check_name()?;
            self.new_local(name, nvars)?;
            nvars += 1;
            if !self.test_next(&Token::Char(b','))? {
                break;
            }
        }
        let (nexps, mut e) =
            if self.test_next(&Token::Char(b'='))? { self.exp_list1()? } else { (0, Exp::new(Kind::Void, 0)) };
        self.adjust_assign(i32_of(nvars), nexps, &mut e)?;
        self.adjust_locals(nvars);
        Ok(())
    }

    fn func_name(&mut self) -> R<(Exp, bool)> {
        let mut v = self.single_var()?;
        while self.ls.token == Token::Char(b'.') {
            self.field(&mut v)?;
        }
        let mut needself = false;
        if self.ls.token == Token::Char(b':') {
            needself = true;
            self.field(&mut v)?;
        }
        Ok((v, needself))
    }

    fn func_stat(&mut self, line: i32) -> R<()> {
        self.next()?;
        let (v, needself) = self.func_name()?;
        let mut b = self.body(needself, line)?;
        self.store_var(&v, &mut b)?;
        self.fix_line(line);
        Ok(())
    }

    fn expr_stat(&mut self) -> R<()> {
        let v = self.primary_exp()?;
        if v.k == Kind::Call {
            let i = *self.instr(v.info);
            *self.instr(v.info) = set_c(i, 1);
            Ok(())
        } else {
            let mut lhs = vec![v];
            self.assignment(&mut lhs, 1)
        }
    }

    fn ret_stat(&mut self) -> R<()> {
        self.next()?;
        let (first, nret);
        if self.block_follow() || self.ls.token == Token::Char(b';') {
            first = 0;
            nret = 0;
        } else {
            let (n, mut e) = self.exp_list1()?;
            if e.has_multret() {
                self.set_returns(&e, MULTRET)?;
                if e.k == Kind::Call && n == 1 {
                    let i = *self.instr(e.info);
                    *self.instr(e.info) = set_op(i, OP_TAILCALL);
                }
                first = i32_of(self.fs().nactvar);
                nret = MULTRET;
            } else if n == 1 {
                first = self.exp2anyreg(&mut e)?;
                nret = 1;
            } else {
                self.exp2nextreg(&mut e)?;
                first = i32_of(self.fs().nactvar);
                nret = n;
            }
        }
        self.ret(first, nret);
        Ok(())
    }

    fn statement(&mut self) -> R<bool> {
        let line = self.ls.line;
        match &self.ls.token {
            Token::If => self.if_stat(line)?,
            Token::While => self.while_stat(line)?,
            Token::Do => {
                self.next()?;
                self.block()?;
                self.check_match(&Token::End, &Token::Do, line)?;
            }
            Token::For => self.for_stat(line)?,
            Token::Repeat => self.repeat_stat(line)?,
            Token::Function => self.func_stat(line)?,
            Token::Local => {
                self.next()?;
                if self.test_next(&Token::Function)? {
                    self.local_func()?;
                } else {
                    self.local_stat()?;
                }
            }
            Token::Return => {
                self.ret_stat()?;
                return Ok(true);
            }
            Token::Break => {
                self.next()?;
                self.break_stat()?;
                return Ok(true);
            }
            _ => self.expr_stat()?,
        }
        Ok(false)
    }

    fn chunk(&mut self) -> R<()> {
        self.enter_level()?;
        let mut islast = false;
        while !islast && !self.block_follow() {
            islast = self.statement()?;
            self.test_next(&Token::Char(b';'))?;
            let fs = self.fs();
            fs.freereg = fs.nactvar;
        }
        self.depth -= 1;
        Ok(())
    }
}

/// `luaY_parser` via `luaL_loadstring`: the chunk's main function.
pub fn compile(source: &[u8], chunk_name: &str) -> Result<Prototype, String> {
    let mut parser = Parser { ls: Lexer::new(source), fs: Vec::new(), depth: 0 };
    parser.fs.push(FuncState::new(Some(chunk_name.as_bytes().to_vec())));
    parser.fs().f.vararg = VARARG_ISVARARG;
    parser.next()?;
    parser.chunk()?;
    parser.check(&Token::Eos)?;
    Ok(parser.close_func())
}
