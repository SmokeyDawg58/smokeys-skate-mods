//! Bounded source grammar: identifiers, decimal f32 constants, subtraction,
//! `&&`, unary minus, parentheses and `.pressed`/`.held`/`.released`/`.repeat` button modifiers.
//! Native `ReadToken` sscanf("%f") is 829882ec;
//! identifier lookup is case-insensitive at 82986d24. Unsupported selected
//! syntax and ambiguous duplicate definitions refuse explicitly.
use super::{EVENT_NAMES, Expression, NAMES, SESSION_MARKER_NAMES, SelectedInputMap};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InputMapParseError {
    Syntax {
        offset: usize,
        message: &'static str,
    },
    Missing(String),
    Duplicate(String),
    Cycle(String),
    Unsupported {
        name: String,
        offset: usize,
    },
    NestingLimit,
}
impl fmt::Display for InputMapParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { offset, message } => write!(f, "input map byte {offset}: {message}"),
            Self::Missing(name) => write!(f, "input map definition missing: {name}"),
            Self::Duplicate(name) => write!(f, "ambiguous selected input definition: {name}"),
            Self::Cycle(name) => write!(f, "input alias cycle: {name}"),
            Self::Unsupported { name, offset } => {
                write!(f, "unsupported selected expression {name} at byte {offset}")
            }
            Self::NestingLimit => f.write_str("input expression exceeds bounded host nesting"),
        }
    }
}
#[derive(Clone, Copy)]
struct Token<'a> {
    text: &'a str,
    offset: usize,
}
struct Definition<'a> {
    name: &'a str,
    body: Vec<Token<'a>>,
}

pub(super) fn parse(
    text: &str,
    include_input_events: bool,
    include_session_markers: bool,
) -> Result<SelectedInputMap, InputMapParseError> {
    let tokens = tokenize(text)?;
    let definitions = definitions(&tokens)?;
    let mut expressions = Vec::with_capacity(NAMES.len());
    for name in NAMES {
        let expression = resolve(name, &definitions, &mut Vec::new())?;
        // Native bytecode evaluates computed children before direct register
        // operands. This bounded evaluator accepts only a single operation on
        // direct operands, for which recursive reads preserve that ordering.
        if !direct_expression(&expression) {
            return Err(InputMapParseError::Unsupported {
                name: name.to_owned(),
                offset: 0,
            });
        }
        expressions.push(expression);
    }
    let input_events = if include_input_events {
        let mut values = Vec::with_capacity(EVENT_NAMES.len());
        for name in EVENT_NAMES {
            let expression = resolve(name, &definitions, &mut Vec::new())?;
            if !direct_expression(&expression) {
                return Err(InputMapParseError::Unsupported {
                    name: name.to_owned(),
                    offset: 0,
                });
            }
            values.push(expression);
        }
        Some(values.try_into().expect("five selected event names"))
    } else {
        None
    };
    let session_markers = if include_session_markers {
        let mut values = Vec::with_capacity(SESSION_MARKER_NAMES.len());
        for name in SESSION_MARKER_NAMES {
            let expression = resolve(name, &definitions, &mut Vec::new())?;
            if !direct_expression(&expression) {
                return Err(InputMapParseError::Unsupported {
                    name: name.to_owned(),
                    offset: 0,
                });
            }
            values.push(expression);
        }
        Some(values.try_into().expect("three marker/HUD input names"))
    } else {
        None
    };
    Ok(SelectedInputMap {
        expressions: expressions.try_into().expect("selected name count"),
        input_events,
        session_markers,
        frontend: None,
    })
}
fn direct_expression(expression: &Expression) -> bool {
    let leaf = |value: &Expression| {
        matches!(
            value,
            Expression::Literal(_)
                | Expression::Button(_)
                | Expression::Held(_)
                | Expression::Pressed(_)
                | Expression::Released(_)
                | Expression::Repeat(_)
        )
    };
    match expression {
        Expression::Literal(_)
        | Expression::Button(_)
        | Expression::Held(_)
        | Expression::Pressed(_)
        | Expression::Released(_)
        | Expression::Repeat(_) => true,
        Expression::Subtract(left, right) | Expression::And(left, right) | Expression::Or(left, right) => {
            leaf(left) && leaf(right)
        }
        Expression::Negate(value) => leaf(value),
    }
}
fn syntax(offset: usize, message: &'static str) -> InputMapParseError {
    InputMapParseError::Syntax { offset, message }
}
fn identifier(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && text.len() <= 63
}
fn tokenize(text: &str) -> Result<Vec<Token<'_>>, InputMapParseError> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset].is_ascii_whitespace() {
            offset += 1;
            continue;
        }
        if bytes[offset..].starts_with(b"//") {
            while offset < bytes.len() && bytes[offset] != b'\n' {
                offset += 1;
            }
            continue;
        }
        if !bytes[offset].is_ascii() {
            return Err(syntax(offset, "non-ASCII input syntax"));
        }
        let start = offset;
        offset += 1;
        if bytes[start].is_ascii_alphabetic() {
            while offset < bytes.len()
                && (bytes[offset].is_ascii_alphanumeric() || bytes[offset] == b'_')
            {
                offset += 1;
            }
        } else if bytes[start].is_ascii_digit() {
            while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                offset += 1;
            }
            if bytes.get(offset) == Some(&b'.') {
                offset += 1;
                while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                    offset += 1;
                }
            }
            if matches!(bytes.get(offset), Some(b'e' | b'E')) {
                offset += 1;
                if matches!(bytes.get(offset), Some(b'+' | b'-')) {
                    offset += 1;
                }
                let exponent = offset;
                while offset < bytes.len() && bytes[offset].is_ascii_digit() {
                    offset += 1;
                }
                if exponent == offset {
                    return Err(syntax(start, "malformed decimal exponent"));
                }
            }
            if bytes.get(offset) == Some(&b'f') {
                offset += 1;
            }
        }
        result.push(Token {
            text: &text[start..offset],
            offset: start,
        });
    }
    Ok(result)
}
fn definitions<'a>(tokens: &[Token<'a>]) -> Result<Vec<Definition<'a>>, InputMapParseError> {
    let mut result = Vec::new();
    let mut position = 0;
    while let Some(name) = tokens.get(position) {
        if !identifier(name.text) {
            return Err(syntax(name.offset, "expected definition name"));
        }
        if tokens.get(position + 1).map(|token| token.text) != Some("=") {
            return Err(syntax(name.offset, "expected assignment"));
        }
        position += 2;
        let start = position;
        let mut depth = 0_usize;
        while let Some(token) = tokens.get(position) {
            match token.text {
                "(" => depth += 1,
                ")" => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| syntax(token.offset, "unmatched closing parenthesis"))?;
                }
                ";" if depth == 0 => break,
                _ => {}
            }
            position += 1;
        }
        if position == tokens.len() {
            return Err(syntax(name.offset, "unterminated definition"));
        }
        result.push(Definition {
            name: name.text,
            body: tokens[start..position].to_vec(),
        });
        position += 1;
    }
    Ok(result)
}
fn resolve(
    name: &str,
    definitions: &[Definition<'_>],
    path: &mut Vec<usize>,
) -> Result<Expression, InputMapParseError> {
    if path.len() >= 128 {
        return Err(InputMapParseError::NestingLimit);
    }
    let mut matches = definitions
        .iter()
        .enumerate()
        .filter(|(_, definition)| definition.name.eq_ignore_ascii_case(name));
    let (index, definition) = matches
        .next()
        .ok_or_else(|| InputMapParseError::Missing(name.to_owned()))?;
    if matches.next().is_some() {
        return Err(InputMapParseError::Duplicate(name.to_owned()));
    }
    if path.contains(&index) {
        return Err(InputMapParseError::Cycle(name.to_owned()));
    }
    path.push(index);
    let mut parser = Parser {
        definition,
        definitions: &definitions[..index],
        position: 0,
        path,
        depth: 0,
    };
    let expression = parser.or_expression()?;
    if parser.position != definition.body.len() {
        return Err(parser.unsupported());
    }
    path.pop();
    Ok(expression)
}
struct Parser<'a, 'b> {
    definition: &'a Definition<'b>,
    definitions: &'a [Definition<'b>],
    position: usize,
    path: &'a mut Vec<usize>,
    depth: usize,
}
impl Parser<'_, '_> {
    fn unsupported(&self) -> InputMapParseError {
        InputMapParseError::Unsupported {
            name: self.definition.name.to_owned(),
            offset: self
                .definition
                .body
                .get(self.position)
                .map_or(0, |token| token.offset),
        }
    }
    fn token(&self, offset: usize) -> Option<&str> {
        self.definition
            .body
            .get(self.position + offset)
            .map(|token| token.text)
    }
    /// ParseAnd8298f250 below ParseOr and above ParseCompare/ParseSum: the
    /// tokenizer yields `&&` as two `&` tokens; left-associative nodes.
    fn or_expression(&mut self) -> Result<Expression, InputMapParseError> {
        let mut left = self.and_expression()?;
        while self.token(0) == Some("|") && self.token(1) == Some("|") {
            self.position += 2;
            left = Expression::Or(Box::new(left), Box::new(self.and_expression()?));
        }
        Ok(left)
    }
    fn and_expression(&mut self) -> Result<Expression, InputMapParseError> {
        let mut left = self.expression()?;
        while self.token(0) == Some("&") && self.token(1) == Some("&") {
            self.position += 2;
            left = Expression::And(Box::new(left), Box::new(self.expression()?));
        }
        Ok(left)
    }
    fn expression(&mut self) -> Result<Expression, InputMapParseError> {
        let mut left = self.atom()?;
        while self
            .definition
            .body
            .get(self.position)
            .map(|token| token.text)
            == Some("-")
        {
            self.position += 1;
            left = Expression::Subtract(Box::new(left), Box::new(self.atom()?));
        }
        Ok(left)
    }
    fn atom(&mut self) -> Result<Expression, InputMapParseError> {
        if self.depth + self.path.len() >= 128 {
            return Err(InputMapParseError::NestingLimit);
        }
        self.depth += 1;
        let token = *self
            .definition
            .body
            .get(self.position)
            .ok_or_else(|| self.unsupported())?;
        self.position += 1;
        let mut result = match token.text {
            "-" => Expression::Negate(Box::new(self.atom()?)),
            "(" => {
                let expression = self.or_expression()?;
                if self
                    .definition
                    .body
                    .get(self.position)
                    .map(|token| token.text)
                    != Some(")")
                {
                    return Err(self.unsupported());
                }
                self.position += 1;
                expression
            }
            text if text.as_bytes()[0].is_ascii_digit() => {
                if text.len() > 63 {
                    return Err(self.unsupported());
                }
                Expression::Literal(
                    text.strip_suffix('f')
                        .unwrap_or(text)
                        .parse::<f32>()
                        .map_err(|_| self.unsupported())?,
                )
            }
            text if identifier(text) => {
                if text.len() > 6
                    && text[..6].eq_ignore_ascii_case("button")
                    && text[6..].bytes().all(|byte| byte.is_ascii_digit())
                {
                    let index = text[6..].parse::<usize>().map_err(|_| self.unsupported())?;
                    // GetRegister exposes 154 button identities (96..1019).
                    if index >= 154 {
                        return Err(self.unsupported());
                    }
                    Expression::Button(index)
                } else {
                    resolve(text, self.definitions, self.path)?
                }
            }
            _ => return Err(self.unsupported()),
        };
        if self
            .definition
            .body
            .get(self.position)
            .map(|token| token.text)
            == Some(".")
        {
            // LookupModifierName829868f8 compares case-insensitively.
            let modifier = self
                .definition
                .body
                .get(self.position + 1)
                .map(|token| token.text);
            let Expression::Button(index) = result else {
                return Err(self.unsupported());
            };
            if modifier.is_some_and(|text| text.eq_ignore_ascii_case("pressed")) {
                result = Expression::Pressed(index);
            } else if modifier.is_some_and(|text| text.eq_ignore_ascii_case("released")) {
                result = Expression::Released(index);
            } else if modifier.is_some_and(|text| text.eq_ignore_ascii_case("repeat")) {
                result = Expression::Repeat(index);
            } else if modifier.is_some_and(|text| text.eq_ignore_ascii_case("held")) {
                result = Expression::Held(index);
            } else {
                return Err(self.unsupported());
            }
            self.position += 2;
        }
        self.depth -= 1;
        Ok(result)
    }
}

/// The original frontend definitions the selected menus read.
pub(super) fn parse_frontend(text: &str) -> Result<[Expression; 34], InputMapParseError> {
    let tokens = tokenize(text)?;
    let definitions = definitions(&tokens)?;
    let mut values = Vec::with_capacity(32);
    for name in [
        "AptStart", "AptNext", "AptPrevious", "AptUp", "AptDown", "AptLeft", "AptRight", "ManualLStickX", "DPadLR",
        "AptMisc", "AptLShift2", "AptRShift2", "AptOption", "AptLeftReleased", "AptRightReleased",
        "AptDUp", "AptDDown", "AptDLeft", "AptDRight", "AptDUpReleased",
        "ManualRStickX", "ManualRStickY", "ManualLStickY",
        "AptRShift1", "AptLShift1", "AptLShift3", "AptLShift1Released",
        "AptDDownReleased", "AptDLeftReleased", "AptDRightReleased", "AptOptionReleased", "AptSelect",
        "AptRShift2Released", "AptLShift2Released",
    ] {
        let expression = resolve(name, &definitions, &mut Vec::new())?;
        if !direct_expression(&expression) {
            return Err(InputMapParseError::Unsupported { name: name.to_owned(), offset: 0 });
        }
        values.push(expression);
    }
    Ok(values.try_into().expect("thirty-four frontend names"))
}
