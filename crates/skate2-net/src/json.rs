//! The little JSON the dedicated server's own files hold (`server-config.json` and the ban
//! lists): one object of strings and numbers, read and written without a library.

use std::fmt::Write as _;

/// A value of the object: a string, a number as written, or anything else (passed over).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Text(String),
    Number(String),
    Other,
}

/// The top-level object's fields in order; `None` for text that is not one JSON object. A byte
/// order mark before it (Notepad writes one) is passed over.
pub(crate) fn object(text: &str) -> Option<Vec<(String, Value)>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
    let mut fields = Vec::new();
    reader.expect(b'{')?;
    if reader.peek()? == b'}' {
        reader.at += 1;
    } else {
        loop {
            let key = reader.string()?;
            reader.expect(b':')?;
            let value = match reader.peek()? {
                b'"' => Value::Text(reader.string()?),
                b'-' | b'0'..=b'9' => Value::Number(reader.number()?.to_owned()),
                _ => {
                    reader.skip_value()?;
                    Value::Other
                }
            };
            fields.push((key, value));
            match reader.next()? {
                b',' => {}
                b'}' => break,
                _ => return None,
            }
        }
    }
    // Nothing but white space may follow the object.
    reader.peek().is_none().then_some(fields)
}

/// `text` as a JSON string.
pub(crate) fn quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            character if u32::from(character) < 0x20 => {
                let _ = write!(quoted, "\\u{:04x}", u32::from(character));
            }
            character => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// The next byte past white space, not taken.
    fn peek(&mut self) -> Option<u8> {
        while self.bytes.get(self.at)?.is_ascii_whitespace() {
            self.at += 1;
        }
        self.bytes.get(self.at).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        (self.next()? == byte).then_some(())
    }

    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut text = String::new();
        loop {
            let start = self.at;
            while self.bytes.get(self.at).is_some_and(|&byte| byte != b'"' && byte != b'\\' && byte >= 0x20) {
                self.at += 1;
            }
            // The run ends at an ASCII byte, so it is whole UTF-8.
            text.push_str(std::str::from_utf8(&self.bytes[start..self.at]).ok()?);
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return Some(text),
                b'\\' => {
                    let escape = *self.bytes.get(self.at)?;
                    self.at += 1;
                    text.push(match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let unit = self.unit()?;
                            if (0xd800..0xdc00).contains(&unit) {
                                // A high surrogate: its low half follows as another escape.
                                if self.bytes.get(self.at..self.at + 2)? != b"\\u" {
                                    return None;
                                }
                                self.at += 2;
                                let low = self.unit()?;
                                char::decode_utf16([unit, low]).next()?.ok()?
                            } else {
                                char::from_u32(u32::from(unit))?
                            }
                        }
                        _ => return None,
                    });
                }
                // A JSON string holds no control character as it is.
                _ => return None,
            }
        }
    }

    /// The four hexadecimal digits of a `\u` escape.
    fn unit(&mut self) -> Option<u16> {
        let digits = self.bytes.get(self.at..self.at + 4)?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        self.at += 4;
        u16::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()
    }

    /// A number's text, as written.
    fn number(&mut self) -> Option<&'a str> {
        self.peek()?;
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E')) {
            self.at += 1;
        }
        let bytes: &'a [u8] = self.bytes;
        std::str::from_utf8(&bytes[start..self.at]).ok().filter(|number| number.parse::<f64>().is_ok())
    }

    fn skip_value(&mut self) -> Option<()> {
        match self.peek()? {
            b'"' => self.string().map(drop),
            b'-' | b'0'..=b'9' => self.number().map(drop),
            open @ (b'{' | b'[') => {
                self.at += 1;
                let close = if open == b'{' { b'}' } else { b']' };
                if self.peek()? == close {
                    self.at += 1;
                    return Some(());
                }
                loop {
                    if open == b'{' {
                        self.string()?;
                        self.expect(b':')?;
                    }
                    self.skip_value()?;
                    match self.next()? {
                        b',' => {}
                        byte if byte == close => return Some(()),
                        _ => return None,
                    }
                }
            }
            _ => {
                for word in [&b"true"[..], b"false", b"null"] {
                    if self.bytes[self.at..].starts_with(word) {
                        self.at += word.len();
                        return Some(());
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_object_reads_back_what_quote_writes() {
        let name = "Sk8 \"Q\" \\ \u{e9} \u{1f6f9}\ttab\u{1}";
        let text = format!("{{\n  \"a\": {},\n  \"port\": 8080\n}}\n", quote(name));
        assert_eq!(object(&text), Some(vec![("a".into(), Value::Text(name.into())), ("port".into(), Value::Number("8080".into()))]));
    }

    #[test]
    fn hand_written_objects_read_as_json_does() {
        assert_eq!(object("\u{feff}{}"), Some(Vec::new()), "a byte order mark is passed over");
        assert_eq!(object(r#"{"x": "🛹 é"}"#), Some(vec![("x".into(), Value::Text("\u{1f6f9} \u{e9}".into()))]));
        assert_eq!(
            object(r#"{"a": [1, {"b": null}], "c": true, "d": -1.5e3}"#),
            Some(vec![("a".into(), Value::Other), ("c".into(), Value::Other), ("d".into(), Value::Number("-1.5e3".into()))])
        );
        for broken in ["", "[]", "{", r#"{"a" 1}"#, r#"{"a": 1,}"#, r#"{"a": "b"} x"#, "{\"a\": \"line\nbreak\"}", r#"{"a": "\ud83d"}"#, r#"{"a": tru}"#] {
            assert_eq!(object(broken), None, "{broken:?}");
        }
    }
}
