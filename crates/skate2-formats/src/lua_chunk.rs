//! Lua 5.1 binary chunks embedded as `Attrib::Blob` values in the
//! `ChallengeCore`/`progressioncore` vaults.
//!
//! `cMessageHandler::Attach` (`0x82ce38c8`) hands each blob to
//! `luaL_loadbuffer`, so the layout is `luaU_undump`'s: the twelve-byte
//! `luaU_header`, then one function record. Skate 2's embedded VM uses a
//! four-byte `float` `lua_Number`, four-byte `int`/`size_t`/`Instruction`,
//! and little-endian chunks.

use std::fmt;

pub const HEADER: [u8; 12] = [0x1b, b'L', b'u', b'a', 0x51, 0, 1, 4, 4, 4, 4, 0];

/// `luaP_opnames` order (Lua 5.1 `lopcodes.h`).
pub const OPCODE_COUNT: u32 = 38;

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    Nil,
    Boolean(bool),
    Number(f32),
    String(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalVariable {
    pub name: Vec<u8>,
    pub start_pc: i32,
    pub end_pc: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prototype {
    /// `LoadFunction` inherits the parent source when a nested record has none.
    pub source: Option<Vec<u8>>,
    pub line_defined: i32,
    pub last_line_defined: i32,
    pub upvalue_count: u8,
    pub parameter_count: u8,
    pub vararg: u8,
    pub max_stack_size: u8,
    pub code: Vec<u32>,
    pub constants: Vec<Constant>,
    pub prototypes: Vec<Prototype>,
    pub line_info: Vec<i32>,
    pub locals: Vec<LocalVariable>,
    pub upvalue_names: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkError {
    pub offset: usize,
    pub reason: &'static str,
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lua chunk at byte {}: {}", self.offset, self.reason)
    }
}

impl std::error::Error for ChunkError {}

type Result<T> = std::result::Result<T, ChunkError>;

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn error(&self, reason: &'static str) -> ChunkError {
        ChunkError { offset: self.pos, reason }
    }
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.pos.checked_add(n).ok_or_else(|| self.error("size overflow"))?;
        let slice = self.bytes.get(self.pos..end).ok_or_else(|| self.error("truncated"))?;
        self.pos = end;
        Ok(slice)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("length checked")))
    }
    fn int(&mut self) -> Result<i32> {
        Ok(self.word()?.cast_signed())
    }
    fn count(&mut self) -> Result<usize> {
        let n = self.int()?;
        usize::try_from(n).map_err(|_| self.error("negative count"))
    }
    /// `LoadString`: a `size_t` length including the trailing NUL; zero is NULL.
    fn string(&mut self) -> Result<Option<Vec<u8>>> {
        let n = self.word()? as usize;
        if n == 0 {
            return Ok(None);
        }
        let bytes = self.take(n)?;
        Ok(Some(bytes[..n - 1].to_vec()))
    }

    fn function(&mut self, parent_source: Option<&[u8]>) -> Result<Prototype> {
        let source = self.string()?.or_else(|| parent_source.map(<[u8]>::to_vec));
        let line_defined = self.int()?;
        let last_line_defined = self.int()?;
        let upvalue_count = self.byte()?;
        let parameter_count = self.byte()?;
        let vararg = self.byte()?;
        let max_stack_size = self.byte()?;
        let code_len = self.count()?;
        let mut code = Vec::with_capacity(code_len.min(1 << 16));
        for _ in 0..code_len {
            let instruction = self.word()?;
            if instruction & 0x3f >= OPCODE_COUNT {
                return Err(self.error("invalid opcode"));
            }
            code.push(instruction);
        }
        let constant_len = self.count()?;
        let mut constants = Vec::with_capacity(constant_len.min(1 << 16));
        for _ in 0..constant_len {
            constants.push(match self.byte()? {
                0 => Constant::Nil,
                1 => Constant::Boolean(self.byte()? != 0),
                3 => Constant::Number(f32::from_bits(self.word()?)),
                4 => Constant::String(self.string()?.ok_or_else(|| self.error("NULL string constant"))?),
                _ => return Err(self.error("invalid constant type")),
            });
        }
        let prototype_len = self.count()?;
        let mut prototypes = Vec::with_capacity(prototype_len.min(1 << 12));
        for _ in 0..prototype_len {
            prototypes.push(self.function(source.as_deref())?);
        }
        let line_len = self.count()?;
        let mut line_info = Vec::with_capacity(line_len.min(1 << 16));
        for _ in 0..line_len {
            line_info.push(self.int()?);
        }
        let local_len = self.count()?;
        let mut locals = Vec::with_capacity(local_len.min(1 << 12));
        for _ in 0..local_len {
            let name = self.string()?.unwrap_or_default();
            let start_pc = self.int()?;
            let end_pc = self.int()?;
            locals.push(LocalVariable { name, start_pc, end_pc });
        }
        let upvalue_len = self.count()?;
        let mut upvalue_names = Vec::with_capacity(upvalue_len.min(256));
        for _ in 0..upvalue_len {
            upvalue_names.push(self.string()?.unwrap_or_default());
        }
        Ok(Prototype {
            source,
            line_defined,
            last_line_defined,
            upvalue_count,
            parameter_count,
            vararg,
            max_stack_size,
            code,
            constants,
            prototypes,
            line_info,
            locals,
            upvalue_names,
        })
    }
}

/// Parses one chunk and returns the main function and the bytes consumed.
pub fn parse(bytes: &[u8]) -> Result<(Prototype, usize)> {
    if bytes.len() < HEADER.len() || bytes[..HEADER.len()] != HEADER {
        return Err(ChunkError { offset: 0, reason: "header is not Skate 2's Lua 5.1 float format" });
    }
    let mut reader = Reader { bytes, pos: HEADER.len() };
    let main = reader.function(None)?;
    Ok((main, reader.pos))
}

/// Instruction field accessors (`lopcodes.h` `GET_OPCODE`/`GETARG_*`).
#[must_use]
pub const fn opcode(instruction: u32) -> u32 {
    instruction & 0x3f
}
#[must_use]
pub const fn arg_a(instruction: u32) -> usize {
    ((instruction >> 6) & 0xff) as usize
}
#[must_use]
pub const fn arg_b(instruction: u32) -> usize {
    ((instruction >> 23) & 0x1ff) as usize
}
#[must_use]
pub const fn arg_c(instruction: u32) -> usize {
    ((instruction >> 14) & 0x1ff) as usize
}
#[must_use]
pub const fn arg_bx(instruction: u32) -> usize {
    (instruction >> 14) as usize
}
#[must_use]
pub const fn arg_sbx(instruction: u32) -> i32 {
    (instruction >> 14) as i32 - 131_071
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(out: &mut Vec<u8>, s: &[u8]) {
        out.extend_from_slice(&(s.len() as u32 + 1).to_le_bytes());
        out.extend_from_slice(s);
        out.push(0);
    }

    #[test]
    fn parses_nested_function_and_inherits_source() {
        let mut b = HEADER.to_vec();
        string(&mut b, b"=handler");
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&[0, 0, 2, 2]);
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&0x0080_001eu32.to_le_bytes());
        b.extend_from_slice(&3i32.to_le_bytes());
        b.push(1);
        b.push(1);
        b.push(3);
        b.extend_from_slice(&2.5f32.to_bits().to_le_bytes());
        b.push(4);
        string(&mut b, b"h_0123");
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&3i32.to_le_bytes());
        b.extend_from_slice(&[0, 3, 0, 4]);
        for _ in 0..6 {
            b.extend_from_slice(&0i32.to_le_bytes());
        }
        for _ in 0..3 {
            b.extend_from_slice(&0i32.to_le_bytes());
        }
        let (main, used) = parse(&b).unwrap();
        assert_eq!(used, b.len());
        assert_eq!(main.constants, vec![
            Constant::Boolean(true),
            Constant::Number(2.5),
            Constant::String(b"h_0123".to_vec()),
        ]);
        assert_eq!(main.prototypes[0].source.as_deref(), Some(&b"=handler"[..]));
        assert_eq!(main.prototypes[0].parameter_count, 3);
        assert_eq!(opcode(main.code[0]), 30);
        assert_eq!(arg_b(main.code[0]), 1);
    }

    #[test]
    fn rejects_double_number_chunks() {
        let mut b = HEADER.to_vec();
        b[10] = 8;
        assert!(parse(&b).is_err());
    }
}
