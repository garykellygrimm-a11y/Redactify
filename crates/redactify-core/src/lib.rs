/// A single detected instance of sensitive content.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub start: usize,
    pub end: usize,
    pub rule_id: String,
}

/// Scan `text` for sensitive content. (Stub — detection engine lands in Milestone 1.)
pub fn detect(text: &str) -> Vec<Finding> {
    let _ = text;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_empty_for_now() {
        assert!(detect("nothing sensitive here").is_empty());
    }
}