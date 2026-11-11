//! Variant and feature builders.

use crate::shared_types::UniProtIsoId;
use crate::uniprot::{Feature, FeatureKind};
use crate::variant::Variant;

/// A canonical-sequence variant.
pub(crate) fn variant(
    id: &str,
    begin: usize,
    end: usize,
    aa_ref: Option<&str>,
    aa_new: Option<&str>,
) -> Variant {
    Variant {
        isoform: None,
        id: id.to_string(),
        begin,
        end,
        aa_ref: aa_ref.map(str::to_string),
        aa_new: aa_new.map(str::to_string),
    }
}

/// A variant located on isoform `iso`.
pub(crate) fn iso_variant(
    iso: &str,
    id: &str,
    begin: usize,
    end: usize,
    aa_ref: &str,
    aa_new: &str,
) -> Variant {
    Variant {
        isoform: Some(UniProtIsoId::new(iso)),
        ..variant(id, begin, end, Some(aa_ref), Some(aa_new))
    }
}

/// A feature of `kind` at `begin..=end`; `original` / `variation` as in the XML.
pub(crate) fn feature(
    kind: FeatureKind,
    id: &str,
    begin: usize,
    end: usize,
    original: Option<&str>,
    variations: &[&str],
) -> Feature {
    Feature {
        kind,
        id: Some(id.to_string()),
        description: None,
        original: original.map(str::to_string),
        variations: variations.iter().map(|v| v.to_string()).collect(),
        begin: Some(begin),
        end: Some(end),
        location_sequence: None,
        evidence_keys: Vec::new(),
    }
}
