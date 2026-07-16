/// A single detected instance of sensitive content within the input text.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Byte offset where the match begins (inclusive).
    pub start: usize,
    /// Byte offset where the match ends (exclusive).
    pub end: usize,
    /// The id of the `Rule` that produced this finding, e.g. "email".
    pub rule_id: String,
    /// The exact text that matched.
    pub matched: String,
}
