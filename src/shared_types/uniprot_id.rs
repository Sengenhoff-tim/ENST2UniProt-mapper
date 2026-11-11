//! UniProt accessions: canonical, isoform, or either.

use super::newtype::string_newtype;

string_newtype!(
    /// Canonical UniProt accession, e.g. `P04637`.
    UniProtCanonId
);

string_newtype!(
    /// UniProt isoform accession, e.g. `P04637-2`.
    UniProtIsoId
);

impl UniProtIsoId {
    /// The canonical accession this isoform belongs to, e.g. `P04637-2` -> `P04637`.
    pub(crate) fn canonical(&self) -> UniProtCanonId {
        UniProtCanonId::new(self.0.split('-').next().unwrap_or(&self.0))
    }
}

/// A UniProt accession that may name either the canonical sequence or an isoform.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum UniProtId {
    Canonical(UniProtCanonId),
    Isoform(UniProtIsoId),
}

impl UniProtId {
    /// Classifies an accession: `P04637-2` is an isoform, `P04637` canonical.
    pub(crate) fn from_accession(accession: &str) -> Self {
        if accession.contains('-') {
            Self::Isoform(UniProtIsoId::new(accession))
        } else {
            Self::Canonical(UniProtCanonId::new(accession))
        }
    }

    /// The canonical accession, e.g. `P04637` for both `P04637` and `P04637-2`.
    pub(crate) fn canonical(&self) -> UniProtCanonId {
        match self {
            Self::Canonical(id) => id.clone(),
            Self::Isoform(id) => id.canonical(),
        }
    }
}

#[cfg(test)]
#[path = "tests/test_uniprot_id.rs"]
mod tests;
