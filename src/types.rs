//! Core domain types shared across modules.
//!
//! Identifiers and sequences are thin wrappers around `String`, so that e.g.
//! a transcript id cannot be passed where a UniProt accession is expected.
//! They compare and hash like the wrapped string and implement `Borrow<str>`,
//! so maps and sets keyed by them can be queried with a plain `&str`.

use std::borrow::Borrow;
use std::fmt;

/// Defines a `String` newtype with `new`, `as_str`, `Display`, `AsRef<str>`
/// and `Borrow<str>`.
macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }
    };
}

string_newtype!(
    /// Unversioned Ensembl transcript id, e.g. `ENST00000269305`.
    EnsemblId
);

string_newtype!(
    /// Canonical UniProt accession, e.g. `P04637`.
    UniProtCanonId
);

string_newtype!(
    /// UniProt isoform accession, e.g. `P04637-2`.
    UniProtIsoId
);

string_newtype!(
    /// Amino acid sequence (one-letter codes).
    Sequence
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
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ensembl_id_from_versioned_strips_the_version() {
        assert_eq!(
            EnsemblId::from_versioned("ENST00000269305.9").as_str(),
            "ENST00000269305"
        );
        assert_eq!(
            EnsemblId::from_versioned("ENST00000269305").as_str(),
            "ENST00000269305"
        );
    }

    #[test]
    fn ensembl_id_recognizes_transcripts() {
        assert!(EnsemblId::new("ENST00000269305").is_transcript());
        assert!(!EnsemblId::new("ENSP00000269305").is_transcript());
        assert!(!EnsemblId::new("NM_000546").is_transcript());
    }

    #[test]
    fn isoform_id_knows_its_canonical_accession() {
        assert_eq!(
            UniProtIsoId::new("P04637-2").canonical(),
            UniProtCanonId::new("P04637")
        );
    }

    #[test]
    fn uniprot_id_classifies_accessions() {
        assert_eq!(
            UniProtId::from_accession("P04637"),
            UniProtId::Canonical(UniProtCanonId::new("P04637"))
        );
        assert_eq!(
            UniProtId::from_accession("P04637-2"),
            UniProtId::Isoform(UniProtIsoId::new("P04637-2"))
        );
    }

    #[test]
    fn uniprot_id_canonical_for_both_kinds() {
        let expected = UniProtCanonId::new("P04637");
        assert_eq!(UniProtId::from_accession("P04637").canonical(), expected);
        assert_eq!(UniProtId::from_accession("P04637-2").canonical(), expected);
    }

    #[test]
    fn ids_can_be_looked_up_by_str() {
        let set: HashSet<UniProtCanonId> = [UniProtCanonId::new("P04637")].into();
        assert!(set.contains("P04637"));
        assert!(!set.contains("P38398"));
    }

    #[test]
    fn ids_display_and_sort_like_their_string() {
        assert_eq!(EnsemblId::new("ENST1").to_string(), "ENST1");
        let mut ids = [
            EnsemblId::new("ENST2"),
            EnsemblId::new("ENST10"),
            EnsemblId::new("ENST1"),
        ];
        ids.sort();
        let sorted: Vec<&str> = ids.iter().map(EnsemblId::as_str).collect();
        assert_eq!(sorted, ["ENST1", "ENST10", "ENST2"]);
    }
}
