//! Lua 5.1 lexer (`llex.c`), for chunks the game compiles from source with
//! `luaL_loadstring` (objective and signpost scripts).

use super::number::string_to_number;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    And,
    Break,
    Do,
    Else,
    ElseIf,
    End,
    False,
    For,
    Function,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    Return,
    Then,
    True,
    Until,
    While,
    Concat,
    Dots,
    Eq,
    Ge,
    Le,
    Ne,
    Number(f32),
    Name(Vec<u8>),
    String(Vec<u8>),
    Eos,
    Char(u8),
}

impl Token {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Number(n) => format!("{n}"),
            Self::Name(s) | Self::String(s) => String::from_utf8_lossy(s).into_owned(),
            Self::Char(c) => char::from(*c).to_string(),
            Self::Eos => "<eof>".into(),
            other => format!("{other:?}").to_lowercase(),
        }
    }
}

fn reserved(word: &[u8]) -> Option<Token> {
    Some(match word {
        b"and" => Token::And,
        b"break" => Token::Break,
        b"do" => Token::Do,
        b"else" => Token::Else,
        b"elseif" => Token::ElseIf,
        b"end" => Token::End,
        b"false" => Token::False,
        b"for" => Token::For,
        b"function" => Token::Function,
        b"if" => Token::If,
        b"in" => Token::In,
        b"local" => Token::Local,
        b"nil" => Token::Nil,
        b"not" => Token::Not,
        b"or" => Token::Or,
        b"repeat" => Token::Repeat,
        b"return" => Token::Return,
        b"then" => Token::Then,
        b"true" => Token::True,
        b"until" => Token::Until,
        b"while" => Token::While,
        _ => return None,
    })
}

pub(crate) struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    pub(crate) line: i32,
    pub(crate) last_line: i32,
    pub(crate) token: Token,
    lookahead: Option<Token>,
    buffer: Vec<u8>,
}

const EOZ: i32 = -1;

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a [u8]) -> Self {
        Self { source, pos: 0, line: 1, last_line: 1, token: Token::Eos, lookahead: None, buffer: Vec::new() }
    }

    fn current(&self) -> i32 {
        self.source.get(self.pos).map_or(EOZ, |&b| i32::from(b))
    }
    fn advance(&mut self) {
        self.pos += 1;
    }
    fn save_and_next(&mut self) {
        if let Some(&b) = self.source.get(self.pos) {
            self.buffer.push(b);
        }
        self.advance();
    }
    fn is_newline(&self) -> bool {
        matches!(self.current(), 10 | 13)
    }
    fn current_is(&self, set: &[u8]) -> bool {
        u8::try_from(self.current()).is_ok_and(|c| set.contains(&c))
    }
    fn check_next(&mut self, set: &[u8]) -> bool {
        if self.current_is(set) {
            self.save_and_next();
            true
        } else {
            false
        }
    }

    pub(crate) fn error(&self, message: &str) -> String {
        format!("[string]:{}: {message}", self.line)
    }

    /// `inclinenumber`.
    fn new_line(&mut self) {
        let old = self.current();
        self.advance();
        if self.is_newline() && self.current() != old {
            self.advance();
        }
        self.line += 1;
    }

    /// `luaX_next`.
    pub(crate) fn next(&mut self) -> Result<(), String> {
        self.last_line = self.line;
        self.token = match self.lookahead.take() {
            Some(t) => t,
            None => self.lex()?,
        };
        Ok(())
    }

    /// `luaX_lookahead`.
    pub(crate) fn peek(&mut self) -> Result<&Token, String> {
        if self.lookahead.is_none() {
            let t = self.lex()?;
            self.lookahead = Some(t);
        }
        Ok(self.lookahead.as_ref().expect("set"))
    }

    /// `skip_sep`: `[=*[` level, or `-1 - count` when not a long bracket.
    fn skip_sep(&mut self) -> i32 {
        let s = self.current();
        self.save_and_next();
        let mut count = 0;
        while self.current() == i32::from(b'=') {
            self.save_and_next();
            count += 1;
        }
        if self.current() == s { count } else { -count - 1 }
    }

    fn read_long_string(&mut self, keep: bool, sep: i32) -> Result<Vec<u8>, String> {
        self.save_and_next();
        if self.is_newline() {
            self.new_line();
        }
        loop {
            match self.current() {
                EOZ => return Err(self.error(if keep { "unfinished long string" } else { "unfinished long comment" })),
                93 => {
                    // ']'
                    if self.skip_sep() == sep {
                        self.save_and_next();
                        break;
                    }
                }
                10 | 13 => {
                    self.buffer.push(b'\n');
                    self.new_line();
                    if !keep {
                        self.buffer.clear();
                    }
                }
                _ => {
                    if keep {
                        self.save_and_next();
                    } else {
                        self.buffer.clear();
                        self.advance();
                    }
                }
            }
        }
        let sep = usize::try_from(sep).unwrap_or(0);
        let start = 2 + sep;
        let end = self.buffer.len().saturating_sub(2 + sep);
        Ok(if keep && start <= end { self.buffer[start..end].to_vec() } else { Vec::new() })
    }

    fn read_string(&mut self, delimiter: i32) -> Result<Vec<u8>, String> {
        self.save_and_next();
        while self.current() != delimiter {
            match self.current() {
                EOZ => return Err(self.error("unfinished string")),
                10 | 13 => return Err(self.error("unfinished string")),
                92 => {
                    // '\\'
                    self.advance();
                    let c = match self.current() {
                        97 => 7,
                        98 => 8,
                        102 => 12,
                        110 => 10,
                        114 => 13,
                        116 => 9,
                        118 => 11,
                        10 | 13 => {
                            self.buffer.push(b'\n');
                            self.new_line();
                            continue;
                        }
                        EOZ => continue,
                        d if (48..=57).contains(&d) => {
                            let mut value = 0;
                            let mut i = 0;
                            while i < 3 && (48..=57).contains(&self.current()) {
                                value = 10 * value + (self.current() - 48);
                                self.advance();
                                i += 1;
                            }
                            if value > 255 {
                                return Err(self.error("escape sequence too large"));
                            }
                            self.buffer.push(u8::try_from(value).expect("checked"));
                            continue;
                        }
                        other => {
                            self.save_and_next();
                            let _ = other;
                            continue;
                        }
                    };
                    self.buffer.push(c);
                    self.advance();
                }
                _ => self.save_and_next(),
            }
        }
        self.save_and_next();
        Ok(self.buffer[1..self.buffer.len() - 1].to_vec())
    }

    /// `read_numeral`: `luaO_str2d` on the collected text.
    fn read_numeral(&mut self) -> Result<f32, String> {
        while self.current_is(b"0123456789.") {
            self.save_and_next();
        }
        if self.check_next(b"Ee") {
            self.check_next(b"+-");
        }
        while u8::try_from(self.current()).is_ok_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
            self.save_and_next();
        }
        string_to_number(&self.buffer).ok_or_else(|| self.error("malformed number"))
    }

    /// `llex`.
    fn lex(&mut self) -> Result<Token, String> {
        self.buffer.clear();
        loop {
            match self.current() {
                10 | 13 => {
                    self.new_line();
                }
                45 => {
                    // '-'
                    self.advance();
                    if self.current() != 45 {
                        return Ok(Token::Char(b'-'));
                    }
                    self.advance();
                    if self.current() == 91 {
                        let sep = self.skip_sep();
                        self.buffer.clear();
                        if sep >= 0 {
                            self.read_long_string(false, sep)?;
                            self.buffer.clear();
                            continue;
                        }
                    }
                    while !self.is_newline() && self.current() != EOZ {
                        self.advance();
                    }
                }
                91 => {
                    // '['
                    let sep = self.skip_sep();
                    if sep >= 0 {
                        return Ok(Token::String(self.read_long_string(true, sep)?));
                    }
                    if sep == -1 {
                        return Ok(Token::Char(b'['));
                    }
                    return Err(self.error("invalid long string delimiter"));
                }
                61 => {
                    self.advance();
                    if self.current() != 61 {
                        return Ok(Token::Char(b'='));
                    }
                    self.advance();
                    return Ok(Token::Eq);
                }
                60 => {
                    self.advance();
                    if self.current() != 61 {
                        return Ok(Token::Char(b'<'));
                    }
                    self.advance();
                    return Ok(Token::Le);
                }
                62 => {
                    self.advance();
                    if self.current() != 61 {
                        return Ok(Token::Char(b'>'));
                    }
                    self.advance();
                    return Ok(Token::Ge);
                }
                126 => {
                    self.advance();
                    if self.current() != 61 {
                        return Ok(Token::Char(b'~'));
                    }
                    self.advance();
                    return Ok(Token::Ne);
                }
                34 | 39 => {
                    let delimiter = self.current();
                    return Ok(Token::String(self.read_string(delimiter)?));
                }
                46 => {
                    // '.'
                    self.save_and_next();
                    if self.check_next(b".") {
                        if self.check_next(b".") {
                            return Ok(Token::Dots);
                        }
                        return Ok(Token::Concat);
                    }
                    if !self.current_is(b"0123456789") {
                        return Ok(Token::Char(b'.'));
                    }
                    return Ok(Token::Number(self.read_numeral()?));
                }
                EOZ => return Ok(Token::Eos),
                c => {
                    let c = u8::try_from(c).expect("byte");
                    if matches!(c, b' ' | b'\t' | 0x0b | 0x0c) {
                        self.advance();
                    } else if c.is_ascii_digit() {
                        return Ok(Token::Number(self.read_numeral()?));
                    } else if c.is_ascii_alphabetic() || c == b'_' {
                        while u8::try_from(self.current()).is_ok_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
                            self.save_and_next();
                        }
                        let word = std::mem::take(&mut self.buffer);
                        return Ok(reserved(&word).unwrap_or(Token::Name(word)));
                    } else {
                        self.advance();
                        return Ok(Token::Char(c));
                    }
                }
            }
        }
    }
}
