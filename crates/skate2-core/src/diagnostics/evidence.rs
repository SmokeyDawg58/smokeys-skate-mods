/// Confidence attached to a reconstructed behavior or layout.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Confidence {
    Hypothesis,
    Medium,
    High,
    Exact,
}

/// Traceability metadata for a reconstructed symbol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceSymbol {
    pub address: u32,
    pub decorated_name: &'static str,
    pub object_file: &'static str,
    pub confidence: Confidence,
}
