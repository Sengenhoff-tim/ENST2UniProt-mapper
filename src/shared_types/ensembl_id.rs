//! Ensembl identifiers.

use super::newtype::string_newtype;

string_newtype!(
    /// Unversioned Ensembl transcript id, e.g. `ENST00000269305`.
    EnsemblId
);

impl EnsemblId {
    /// Drops a trailing `.version`, e.g. `ENST00000269305.9` -> `ENST00000269305`.
    pub(crate) fn from_versioned(id: &str) -> Self {
        Self::new(id.split('.').next().unwrap_or(id))
    }

    /// Whether this looks like a transcript id (`ENST...`).
    pub(crate) fn is_transcript(&self) -> bool {
        self.0.starts_with("ENST")
    }
}

#[cfg(test)]
#[path = "tests/test_ensembl_id.rs"]
mod tests;
