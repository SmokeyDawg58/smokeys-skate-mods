//! Compiled `BehaviourGraph` descriptor elements.
//!
//! This is the recursive wire grammar consumed by
//! `BehaviourGraph::BehaviourParser::ReadElement` at `0x82cf6070`. It decodes
//! serialized elements only; runtime factory registrations and node behavior
//! belong to the core layer.

use std::{error::Error, fmt, str};

pub const BEHAVIOUR_BINARY_ATTRIBUTE_CAPACITY: usize = 16;
pub const BEHAVIOUR_BINARY_SAFE_DEPTH: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviourBinaryAttribute {
    pub name: String,
    pub text: String,
    pub float_bits: u32,
    pub boolean_raw: u8,
}

impl BehaviourBinaryAttribute {
    #[must_use]
    pub const fn float_value(&self) -> f32 {
        f32::from_bits(self.float_bits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviourBinaryElement {
    pub name: String,
    pub attributes: Vec<BehaviourBinaryAttribute>,
    pub children: Vec<Self>,
}

impl BehaviourBinaryElement {
    /// Parses one complete compiled `BehaviourGraph` descriptor.
    ///
    /// Native C-string scans and recursion are unsafe on malformed input. This
    /// checked host parser requires UTF-8, bounds every string/field, limits
    /// recursion, and rejects trailing bytes; those are safety policies beyond
    /// the valid-corpus native path.
    ///
    /// # Errors
    ///
    /// Returns a typed truncation, string, count, depth, allocation, or
    /// trailing-data failure.
    pub fn parse(bytes: &[u8]) -> Result<Self, BehaviourBinaryError> {
        let mut reader = Reader { bytes, position: 0 };
        let root = reader.element(0)?;
        if reader.position != bytes.len() {
            return Err(BehaviourBinaryError::TrailingBytes {
                count: bytes.len() - reader.position,
            });
        }
        Ok(root)
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(Self::node_count).sum::<usize>()
    }

    #[must_use]
    pub fn preorder(&self) -> BehaviourBinaryPreorder<'_> {
        BehaviourBinaryPreorder { stack: vec![self] }
    }

    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&BehaviourBinaryAttribute> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
    }
}

pub struct BehaviourBinaryPreorder<'a> {
    stack: Vec<&'a BehaviourBinaryElement>,
}

impl<'a> Iterator for BehaviourBinaryPreorder<'a> {
    type Item = &'a BehaviourBinaryElement;

    fn next(&mut self) -> Option<Self::Item> {
        let element = self.stack.pop()?;
        self.stack.extend(element.children.iter().rev());
        Some(element)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BehaviourBinaryError {
    UnexpectedEnd {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    UnterminatedString {
        offset: usize,
    },
    InvalidUtf8 {
        offset: usize,
    },
    TooManyAttributes {
        count: u32,
        maximum: usize,
    },
    ImpossibleChildCount {
        count: u32,
        remaining: usize,
    },
    Depth {
        depth: usize,
        maximum: usize,
    },
    Allocation,
    TrailingBytes {
        count: usize,
    },
}

impl fmt::Display for BehaviourBinaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "compiled BehaviourGraph needs {needed} bytes at {offset}, only {remaining} remain"
            ),
            Self::UnterminatedString { offset } => write!(
                formatter,
                "compiled BehaviourGraph string at {offset} has no NUL terminator"
            ),
            Self::InvalidUtf8 { offset } => write!(
                formatter,
                "compiled BehaviourGraph string at {offset} is not UTF-8"
            ),
            Self::TooManyAttributes { count, maximum } => write!(
                formatter,
                "compiled BehaviourGraph has {count} attributes; maximum is {maximum}"
            ),
            Self::ImpossibleChildCount { count, remaining } => write!(
                formatter,
                "compiled BehaviourGraph declares {count} children in {remaining} remaining bytes"
            ),
            Self::Depth { depth, maximum } => write!(
                formatter,
                "compiled BehaviourGraph depth {depth} exceeds safe maximum {maximum}"
            ),
            Self::Allocation => formatter.write_str("compiled BehaviourGraph allocation failed"),
            Self::TrailingBytes { count } => {
                write!(
                    formatter,
                    "compiled BehaviourGraph has {count} trailing bytes"
                )
            }
        }
    }
}

impl Error for BehaviourBinaryError {}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn element(&mut self, depth: usize) -> Result<BehaviourBinaryElement, BehaviourBinaryError> {
        if depth > BEHAVIOUR_BINARY_SAFE_DEPTH {
            return Err(BehaviourBinaryError::Depth {
                depth,
                maximum: BEHAVIOUR_BINARY_SAFE_DEPTH,
            });
        }
        let name = self.string()?;
        let attribute_count = self.u32()?;
        if usize::try_from(attribute_count)
            .ok()
            .is_none_or(|count| count > BEHAVIOUR_BINARY_ATTRIBUTE_CAPACITY)
        {
            return Err(BehaviourBinaryError::TooManyAttributes {
                count: attribute_count,
                maximum: BEHAVIOUR_BINARY_ATTRIBUTE_CAPACITY,
            });
        }
        let attribute_count = usize::try_from(attribute_count)
            .expect("validated BehaviourGraph attribute count fits usize");
        let mut attributes = Vec::new();
        attributes
            .try_reserve_exact(attribute_count)
            .map_err(|_| BehaviourBinaryError::Allocation)?;
        for _ in 0..attribute_count {
            attributes.push(BehaviourBinaryAttribute {
                name: self.string()?,
                text: self.string()?,
                float_bits: self.u32()?,
                boolean_raw: self.u8()?,
            });
        }
        let child_count = self.u32()?;
        let child_count_usize = usize::try_from(child_count).map_err(|_| {
            BehaviourBinaryError::ImpossibleChildCount {
                count: child_count,
                remaining: self.remaining(),
            }
        })?;
        if child_count_usize > self.remaining() / 9 {
            return Err(BehaviourBinaryError::ImpossibleChildCount {
                count: child_count,
                remaining: self.remaining(),
            });
        }
        let mut children = Vec::new();
        children
            .try_reserve_exact(child_count_usize)
            .map_err(|_| BehaviourBinaryError::Allocation)?;
        for _ in 0..child_count_usize {
            children.push(self.element(depth + 1)?);
        }
        Ok(BehaviourBinaryElement {
            name,
            attributes,
            children,
        })
    }

    fn string(&mut self) -> Result<String, BehaviourBinaryError> {
        let offset = self.position;
        let remaining = &self.bytes[offset..];
        let length = remaining
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(BehaviourBinaryError::UnterminatedString { offset })?;
        let value = str::from_utf8(&remaining[..length])
            .map_err(|_| BehaviourBinaryError::InvalidUtf8 { offset })?
            .to_owned();
        self.position += length + 1;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, BehaviourBinaryError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, BehaviourBinaryError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .expect("checked BehaviourGraph u32 width"),
        ))
    }

    fn take(&mut self, count: usize) -> Result<&[u8], BehaviourBinaryError> {
        let remaining = self.remaining();
        if remaining < count {
            return Err(BehaviourBinaryError::UnexpectedEnd {
                offset: self.position,
                needed: count,
                remaining,
            });
        }
        let start = self.position;
        self.position += count;
        Ok(&self.bytes[start..self.position])
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32_bytes(value: u32) -> [u8; 4] {
        value.to_be_bytes()
    }

    #[test]
    fn parses_recursive_elements_and_preserves_raw_attribute_fields() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BehaviourSource\0");
        bytes.extend_from_slice(&u32_bytes(0));
        bytes.extend_from_slice(&u32_bytes(1));
        bytes.extend_from_slice(b"BehaviourPlugin\0");
        bytes.extend_from_slice(&u32_bytes(2));
        bytes.extend_from_slice(b"file\0state/plugin.xml\0");
        bytes.extend_from_slice(&u32_bytes(0x3f80_0000));
        bytes.push(0);
        bytes.extend_from_slice(b"enabled\0true\0");
        bytes.extend_from_slice(&u32_bytes(0x4040_0000));
        bytes.push(0xa5);
        bytes.extend_from_slice(&u32_bytes(0));

        let root = BehaviourBinaryElement::parse(&bytes).unwrap();
        assert_eq!(root.name, "BehaviourSource");
        assert_eq!(root.node_count(), 2);
        let plugin = &root.children[0];
        assert_eq!(plugin.name, "BehaviourPlugin");
        assert_eq!(plugin.attribute("file").unwrap().float_bits, 0x3f80_0000);
        assert_eq!(plugin.attribute("enabled").unwrap().float_bits, 0x4040_0000);
        assert_eq!(plugin.attribute("enabled").unwrap().boolean_raw, 0xa5);
        assert_eq!(
            root.preorder().map(|node| &node.name).collect::<Vec<_>>(),
            ["BehaviourSource", "BehaviourPlugin"]
        );
    }

    #[test]
    fn rejects_attribute_overflow_truncation_and_trailing_bytes() {
        let mut too_many = b"node\0".to_vec();
        too_many.extend_from_slice(&u32_bytes(17));
        assert_eq!(
            BehaviourBinaryElement::parse(&too_many),
            Err(BehaviourBinaryError::TooManyAttributes {
                count: 17,
                maximum: BEHAVIOUR_BINARY_ATTRIBUTE_CAPACITY,
            })
        );
        assert!(matches!(
            BehaviourBinaryElement::parse(b"unterminated"),
            Err(BehaviourBinaryError::UnterminatedString { offset: 0 })
        ));
        let mut trailing = b"root\0\0\0\0\0\0\0\0\0".to_vec();
        trailing.push(1);
        assert_eq!(
            BehaviourBinaryElement::parse(&trailing),
            Err(BehaviourBinaryError::TrailingBytes { count: 1 })
        );
    }
}
