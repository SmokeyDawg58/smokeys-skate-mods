//! Compiled `StateGraph` preorder payloads.
//!
//! This module reconstructs the wire reader used by
//! `StateGraph::Parser::ReadElement` at `0x82cf8740`. It intentionally stops at
//! the lossless element boundary: concrete condition, behaviour, and hook
//! factories belong to the runtime layer and are not inferred here.

use std::{error::Error, fmt};

pub const STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY: usize = 16;
pub const STATE_GRAPH_PARSER_STACK_CAPACITY: usize = 24;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphBinaryAttribute {
    /// Diagnostic copy of the serialized key. Native lookup retains only its
    /// 32-bit hash.
    pub key: Vec<u8>,
    pub key_hash: u32,
    pub text: Vec<u8>,
    pub float_bits: u32,
    pub boolean_raw: u8,
}

impl StateGraphBinaryAttribute {
    #[must_use]
    pub const fn float_value(&self) -> f32 {
        f32::from_bits(self.float_bits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphBinaryElement {
    pub name: Vec<u8>,
    pub attributes: Vec<StateGraphBinaryAttribute>,
    pub children: Vec<Self>,
}

impl StateGraphBinaryElement {
    /// Looks up an attribute through the native hash-only, first-insertion-wins
    /// identity rule. A colliding serialized key therefore aliases the first
    /// attribute with that hash.
    #[must_use]
    pub fn attribute(&self, key: &[u8]) -> Option<&StateGraphBinaryAttribute> {
        let hash = state_graph_string_hash32(key);
        self.attributes
            .iter()
            .find(|attribute| attribute.key_hash == hash)
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(Self::node_count).sum::<usize>()
    }

    #[must_use]
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
            + self
                .children
                .iter()
                .map(Self::attribute_count)
                .sum::<usize>()
    }

    #[must_use]
    pub fn maximum_depth(&self) -> usize {
        self.children
            .iter()
            .map(Self::maximum_depth)
            .max()
            .map_or(0, |child_depth| child_depth + 1)
    }

    #[must_use]
    pub fn preorder(&self) -> StateGraphBinaryPreorder<'_> {
        StateGraphBinaryPreorder { stack: vec![self] }
    }
}

pub struct StateGraphBinaryPreorder<'a> {
    stack: Vec<&'a StateGraphBinaryElement>,
}

impl<'a> Iterator for StateGraphBinaryPreorder<'a> {
    type Item = &'a StateGraphBinaryElement;

    fn next(&mut self) -> Option<Self::Item> {
        let element = self.stack.pop()?;
        self.stack.extend(element.children.iter().rev());
        Some(element)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphBinaryDocument {
    pub root: StateGraphBinaryElement,
    pub consumed: usize,
}

impl StateGraphBinaryDocument {
    /// Decodes the first root exactly as native `ParseBinary`: bytes following
    /// that recursive record are not inspected.
    ///
    /// Native malformed-input behavior can assert and continue into invalid
    /// memory. This checked host reader instead bounds strings, fields, counts,
    /// allocation, and the 24-entry parser stack.
    ///
    /// # Errors
    ///
    /// Returns a typed malformed-input or host-allocation failure.
    pub fn parse_native_prefix(bytes: &[u8]) -> Result<Self, StateGraphBinaryError> {
        let mut reader = Reader { bytes, position: 0 };
        let root = reader.element(0)?;
        Ok(Self {
            root,
            consumed: reader.position,
        })
    }

    /// Decodes one root and additionally requires the corpus/test exact-
    /// consumption policy. Native `ParseBinary` itself does not perform this
    /// final check.
    ///
    /// # Errors
    ///
    /// Returns a typed parser failure or the number of trailing bytes.
    pub fn parse_exact(bytes: &[u8]) -> Result<Self, StateGraphBinaryError> {
        let document = Self::parse_native_prefix(bytes)?;
        if document.consumed != bytes.len() {
            return Err(StateGraphBinaryError::TrailingBytes {
                count: bytes.len() - document.consumed,
            });
        }
        Ok(document)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateGraphBinaryError {
    UnexpectedEnd {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    UnterminatedString {
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
    StackCapacity {
        nesting: usize,
        maximum: usize,
    },
    Allocation,
    TrailingBytes {
        count: usize,
    },
}

impl fmt::Display for StateGraphBinaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "compiled StateGraph needs {needed} bytes at {offset}, only {remaining} remain"
            ),
            Self::UnterminatedString { offset } => write!(
                formatter,
                "compiled StateGraph string at {offset} has no NUL terminator"
            ),
            Self::TooManyAttributes { count, maximum } => write!(
                formatter,
                "compiled StateGraph has {count} attributes; native maximum is {maximum}"
            ),
            Self::ImpossibleChildCount { count, remaining } => write!(
                formatter,
                "compiled StateGraph declares {count} children in {remaining} remaining bytes"
            ),
            Self::StackCapacity { nesting, maximum } => write!(
                formatter,
                "compiled StateGraph nesting {nesting} exceeds native parser-stack capacity {maximum}"
            ),
            Self::Allocation => formatter.write_str("compiled StateGraph allocation failed"),
            Self::TrailingBytes { count } => {
                write!(formatter, "compiled StateGraph has {count} trailing bytes")
            }
        }
    }
}

impl Error for StateGraphBinaryError {}

/// Reconstructs `stringhash32` (`0x82985a40`) with the `StateGraph` attribute
/// seed `0xabcdef00`. Empty input maps to zero exactly as the wrapper does.
///
/// # Panics
///
/// Panics when the byte slice is larger than the native 32-bit length domain.
#[must_use]
pub fn state_graph_string_hash32(bytes: &[u8]) -> u32 {
    if bytes.is_empty() {
        return 0;
    }
    let mut a = 0x9e37_79b9_u32;
    let mut b = 0x9e37_79b9_u32;
    let mut c = 0xabcd_ef00_u32;
    let mut chunks = bytes.chunks_exact(12);
    for chunk in &mut chunks {
        a = a.wrapping_add(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        b = b.wrapping_add(u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]));
        c = c.wrapping_add(u32::from_le_bytes([
            chunk[8], chunk[9], chunk[10], chunk[11],
        ]));
        (a, b, c) = lookup2_mix(a, b, c);
    }
    c = c.wrapping_add(
        u32::try_from(bytes.len()).expect("StateGraph hash input exceeds native u32 length"),
    );
    let tail = chunks.remainder();
    if tail.len() >= 11 {
        c = c.wrapping_add(u32::from(tail[10]) << 24);
    }
    if tail.len() >= 10 {
        c = c.wrapping_add(u32::from(tail[9]) << 16);
    }
    if tail.len() >= 9 {
        c = c.wrapping_add(u32::from(tail[8]) << 8);
    }
    if tail.len() >= 8 {
        b = b.wrapping_add(u32::from(tail[7]) << 24);
    }
    if tail.len() >= 7 {
        b = b.wrapping_add(u32::from(tail[6]) << 16);
    }
    if tail.len() >= 6 {
        b = b.wrapping_add(u32::from(tail[5]) << 8);
    }
    if tail.len() >= 5 {
        b = b.wrapping_add(u32::from(tail[4]));
    }
    if tail.len() >= 4 {
        a = a.wrapping_add(u32::from(tail[3]) << 24);
    }
    if tail.len() >= 3 {
        a = a.wrapping_add(u32::from(tail[2]) << 16);
    }
    if tail.len() >= 2 {
        a = a.wrapping_add(u32::from(tail[1]) << 8);
    }
    if !tail.is_empty() {
        a = a.wrapping_add(u32::from(tail[0]));
    }
    lookup2_mix(a, b, c).2
}

fn lookup2_mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 13);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ a.wrapping_shl(8);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 13);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 12);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ a.wrapping_shl(16);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 5);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 3);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ a.wrapping_shl(10);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 15);
    (a, b, c)
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn element(&mut self, depth: usize) -> Result<StateGraphBinaryElement, StateGraphBinaryError> {
        if depth >= STATE_GRAPH_PARSER_STACK_CAPACITY {
            return Err(StateGraphBinaryError::StackCapacity {
                nesting: depth + 1,
                maximum: STATE_GRAPH_PARSER_STACK_CAPACITY,
            });
        }
        let name = self.string()?;
        let attribute_count = self.u32()?;
        if usize::try_from(attribute_count)
            .ok()
            .is_none_or(|count| count > STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY)
        {
            return Err(StateGraphBinaryError::TooManyAttributes {
                count: attribute_count,
                maximum: STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY,
            });
        }
        let attribute_count = usize::try_from(attribute_count)
            .expect("validated StateGraph attribute count fits usize");
        let mut attributes = Vec::new();
        attributes
            .try_reserve_exact(attribute_count)
            .map_err(|_| StateGraphBinaryError::Allocation)?;
        for _ in 0..attribute_count {
            let key = self.string()?;
            attributes.push(StateGraphBinaryAttribute {
                key_hash: state_graph_string_hash32(&key),
                key,
                text: self.string()?,
                float_bits: self.u32()?,
                boolean_raw: self.u8()?,
            });
        }
        let child_count = self.u32()?;
        let child_count_usize = usize::try_from(child_count).map_err(|_| {
            StateGraphBinaryError::ImpossibleChildCount {
                count: child_count,
                remaining: self.remaining(),
            }
        })?;
        if child_count_usize > self.remaining() / 9 {
            return Err(StateGraphBinaryError::ImpossibleChildCount {
                count: child_count,
                remaining: self.remaining(),
            });
        }
        let mut children = Vec::new();
        children
            .try_reserve_exact(child_count_usize)
            .map_err(|_| StateGraphBinaryError::Allocation)?;
        for _ in 0..child_count_usize {
            children.push(self.element(depth + 1)?);
        }
        Ok(StateGraphBinaryElement {
            name,
            attributes,
            children,
        })
    }

    fn string(&mut self) -> Result<Vec<u8>, StateGraphBinaryError> {
        let offset = self.position;
        let remaining = &self.bytes[offset..];
        let length = remaining
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(StateGraphBinaryError::UnterminatedString { offset })?;
        let value = remaining[..length].to_vec();
        self.position += length + 1;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, StateGraphBinaryError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, StateGraphBinaryError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .expect("checked StateGraph u32 width"),
        ))
    }

    fn take(&mut self, count: usize) -> Result<&[u8], StateGraphBinaryError> {
        let remaining = self.remaining();
        if remaining < count {
            return Err(StateGraphBinaryError::UnexpectedEnd {
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

    fn push_element(
        bytes: &mut Vec<u8>,
        name: &str,
        attributes: &[(&str, &str, u32, u8)],
        children: u32,
    ) {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&u32::try_from(attributes.len()).unwrap().to_be_bytes());
        for (key, text, bits, boolean) in attributes {
            bytes.extend_from_slice(key.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(text.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&bits.to_be_bytes());
            bytes.push(*boolean);
        }
        bytes.extend_from_slice(&children.to_be_bytes());
    }

    #[test]
    fn exact_hash_matches_recovered_dispatch_constants() {
        assert_eq!(state_graph_string_hash32(b""), 0);
        assert_eq!(state_graph_string_hash32(b"stategraph"), 0x7526_fd2f);
        assert_eq!(state_graph_string_hash32(b"state"), 0x5f91_f83d);
        assert_eq!(state_graph_string_hash32(b"transition"), 0x8c0e_fa95);
        assert_eq!(state_graph_string_hash32(b"expression"), 0xf617_65b1);
        assert_eq!(state_graph_string_hash32(b"condition"), 0xf868_f111);
        assert_eq!(state_graph_string_hash32(b"behaviour"), 0xae36_6e1e);
        assert_eq!(state_graph_string_hash32(b"param"), 0xe54f_b484);
        assert_eq!(state_graph_string_hash32(b"hook"), 0x18c2_92cc);
        assert_eq!(state_graph_string_hash32(b"include"), 0x902a_b1f7);
        assert_eq!(state_graph_string_hash32(b"abcdefghijkl"), 0x0e43_3798);
        assert_eq!(state_graph_string_hash32(b"abcdefghijklm"), 0xee05_aae8);
        assert_eq!(
            state_graph_string_hash32(b"abcdefghijklmnopqrstuvwx"),
            0x85e4_1a17
        );
        assert_eq!(
            state_graph_string_hash32(b"abcdefghijklmnopqrstuvwxy"),
            0x1fe7_0cbb
        );
    }

    #[test]
    fn parses_ordered_tree_and_preserves_raw_typed_attributes() {
        let mut bytes = Vec::new();
        push_element(
            &mut bytes,
            "state",
            &[
                ("negative", "-1", 0xbf80_0000, 0),
                ("nan", "nan", 0x7fc0_1234, 1),
            ],
            2,
        );
        push_element(&mut bytes, "param", &[], 0);
        push_element(&mut bytes, "unknown", &[], 0);

        let document = StateGraphBinaryDocument::parse_exact(&bytes).unwrap();
        assert_eq!(document.consumed, bytes.len());
        assert_eq!(document.root.node_count(), 3);
        assert_eq!(document.root.attribute_count(), 2);
        assert_eq!(document.root.maximum_depth(), 1);
        assert_eq!(
            document.root.attribute(b"negative").unwrap().float_bits,
            0xbf80_0000
        );
        assert_eq!(
            document.root.attribute(b"nan").unwrap().float_bits,
            0x7fc0_1234
        );
        assert_eq!(document.root.attribute(b"nan").unwrap().boolean_raw, 1);
        assert_eq!(
            document
                .root
                .preorder()
                .map(|node| node.name.as_slice())
                .collect::<Vec<_>>(),
            [
                b"state".as_slice(),
                b"param".as_slice(),
                b"unknown".as_slice()
            ]
        );
    }

    #[test]
    fn native_prefix_accepts_trailing_bytes_but_exact_policy_rejects_them() {
        let mut bytes = Vec::new();
        push_element(&mut bytes, "state", &[], 0);
        let consumed = bytes.len();
        bytes.extend_from_slice(&[0xaa, 0xbb]);
        assert_eq!(
            StateGraphBinaryDocument::parse_native_prefix(&bytes)
                .unwrap()
                .consumed,
            consumed
        );
        assert_eq!(
            StateGraphBinaryDocument::parse_exact(&bytes),
            Err(StateGraphBinaryError::TrailingBytes { count: 2 })
        );
    }

    #[test]
    fn hash_only_lookup_keeps_the_first_duplicate() {
        let mut bytes = Vec::new();
        push_element(
            &mut bytes,
            "state",
            &[("name", "first", 1, 0), ("name", "second", 2, 1)],
            0,
        );
        let document = StateGraphBinaryDocument::parse_exact(&bytes).unwrap();
        assert_eq!(document.root.attribute(b"name").unwrap().text, b"first");
    }

    #[test]
    fn hash_only_lookup_aliases_distinct_colliding_keys_to_the_first_value() {
        assert_eq!(state_graph_string_hash32(b"a05763"), 0xfe11_cd43);
        assert_eq!(state_graph_string_hash32(b"a063d3"), 0xfe11_cd43);
        let mut bytes = Vec::new();
        push_element(
            &mut bytes,
            "state",
            &[("a05763", "first", 1, 0), ("a063d3", "second", 2, 1)],
            0,
        );
        let document = StateGraphBinaryDocument::parse_exact(&bytes).unwrap();
        assert_eq!(document.root.attribute(b"a05763").unwrap().text, b"first");
        assert_eq!(document.root.attribute(b"a063d3").unwrap().text, b"first");
    }

    #[test]
    fn preserves_non_utf8_c_string_bytes_losslessly() {
        let mut bytes = vec![0xff, 0];
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&[0xfe, 0, 0xfd, 0]);
        bytes.extend_from_slice(&0x7fc0_1234_u32.to_be_bytes());
        bytes.push(0xa5);
        bytes.extend_from_slice(&0_u32.to_be_bytes());

        let document = StateGraphBinaryDocument::parse_exact(&bytes).unwrap();
        assert_eq!(document.root.name, [0xff]);
        let attribute = document.root.attribute(&[0xfe]).unwrap();
        assert_eq!(attribute.key, [0xfe]);
        assert_eq!(attribute.text, [0xfd]);
        assert_eq!(attribute.float_bits, 0x7fc0_1234);
        assert_eq!(attribute.boolean_raw, 0xa5);
    }

    #[test]
    fn rejects_unterminated_short_attribute_overflow_and_twenty_fifth_level() {
        assert_eq!(
            StateGraphBinaryDocument::parse_exact(b"state"),
            Err(StateGraphBinaryError::UnterminatedString { offset: 0 })
        );
        assert!(matches!(
            StateGraphBinaryDocument::parse_exact(b"state\0\0\0"),
            Err(StateGraphBinaryError::UnexpectedEnd { .. })
        ));
        let mut too_many = b"state\0".to_vec();
        too_many.extend_from_slice(&17_u32.to_be_bytes());
        assert_eq!(
            StateGraphBinaryDocument::parse_exact(&too_many),
            Err(StateGraphBinaryError::TooManyAttributes {
                count: 17,
                maximum: STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY,
            })
        );

        let mut maximum_nesting = Vec::new();
        for _ in 0..23 {
            push_element(&mut maximum_nesting, "state", &[], 1);
        }
        push_element(&mut maximum_nesting, "state", &[], 0);
        assert_eq!(
            StateGraphBinaryDocument::parse_exact(&maximum_nesting)
                .unwrap()
                .root
                .maximum_depth(),
            23
        );

        let mut nested = Vec::new();
        for _ in 0..24 {
            push_element(&mut nested, "state", &[], 1);
        }
        push_element(&mut nested, "state", &[], 0);
        assert_eq!(
            StateGraphBinaryDocument::parse_exact(&nested),
            Err(StateGraphBinaryError::StackCapacity {
                nesting: 25,
                maximum: STATE_GRAPH_PARSER_STACK_CAPACITY,
            })
        );
    }
}
