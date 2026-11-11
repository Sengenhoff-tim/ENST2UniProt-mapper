use std::fmt;

use crate::shared_types::UniProtIsoId;

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    pub(crate) isoform: Option<UniProtIsoId>,
    pub(crate) id: String,
    /// 1-based, inclusive.
    pub(crate) begin: usize,
    /// 1-based, inclusive.
    pub(crate) end: usize,
    pub(crate) aa_ref: Option<String>,
    /// `None` encodes missing sequence.
    pub(crate) aa_new: Option<String>,
}

// Skips the id field.
impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.isoform == other.isoform
            && self.begin == other.begin
            && self.end == other.end
            && self.aa_ref == other.aa_ref
            && self.aa_new == other.aa_new
    }
}

impl Eq for Variant {}

// Skips the id field, consistent with PartialEq.
impl std::hash::Hash for Variant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.isoform.hash(state);
        self.begin.hash(state);
        self.end.hash(state);
        self.aa_ref.hash(state);
        self.aa_new.hash(state);
    }
}

/// Compact one-line form used in logs and error messages, e.g.
/// `EBI:1929:G (canonical:1929-1929 G>W)`. A `-` stands for a `None` field.
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let isoform = self
            .isoform
            .as_ref()
            .map_or("canonical", |iso| iso.as_str());
        write!(
            f,
            "{} ({}:{}-{} {}>{})",
            self.id,
            isoform,
            self.begin,
            self.end,
            self.aa_ref.as_deref().unwrap_or("-"),
            self.aa_new.as_deref().unwrap_or("-"),
        )
    }
}
#[cfg(test)]
#[path = "tests/test_types.rs"]
mod tests;
