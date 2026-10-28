//! Ordered original `.pat` records. Duplicate names retain distinct owners.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "retained gameplay recognizer integration pending")
)]
use std::sync::Arc;
#[path = "input_pattern_parser.rs"]
mod parser;
pub(crate) use parser::InputPatternParseError;

#[derive(Debug)]
pub(crate) struct InputPattern {
    name: String,
    // Native push_front: index zero is the LAST authored point.
    points: Vec<[f32; 2]>,
    squared_tolerance: f32,
}
impl InputPattern {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn points(&self) -> &[[f32; 2]] {
        &self.points
    }
    pub(crate) const fn squared_tolerance(&self) -> f32 {
        self.squared_tolerance
    }
}
#[derive(Debug)]
pub(crate) struct InputPatterns {
    patterns: Vec<Arc<InputPattern>>,
}
impl InputPatterns {
    /// `KnownPatterns::Load826e2d30`, bounded original coordinate grammar.
    /// Safe-host refusal for unsupported CRT syntax/clock/capacity. No native
    /// reload is claimed: failed parse returns no newly published collection.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Arc<Self>, InputPatternParseError> {
        parser::parse(bytes).map(Arc::new)
    }
    pub(crate) fn patterns(&self) -> &[Arc<InputPattern>] {
        &self.patterns
    }
}
#[cfg(test)]
#[path = "input_pattern_tests.rs"]
pub(crate) mod tests;
