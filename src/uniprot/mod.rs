//! UniProtKB entries, read from a local XML dump with an optional REST API
//! fallback. Both sources are UniProt XML and go through the same reader.

mod batch;
mod entries;
mod fetch;
mod model;
mod xml;

pub(crate) use entries::{log_unresolved_accessions, UniProtEntries};
pub(crate) use fetch::fetch_uniprot_entries;
pub(crate) use model::{
    AlternativeProducts, EvidenceSource, Feature, Isoform, IsoformSequenceType, UniProtEntry,
};
pub(crate) use xml::load_uniprot_xml;

#[cfg(test)]
pub(crate) use model::{Evidence, FeatureKind};
#[cfg(test)]
pub(crate) use xml::parse_uniprot_xml;
