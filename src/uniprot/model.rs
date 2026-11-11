//! Minimal UniProtKB entry model: only the parts of the UniProt XML this
//! pipeline reads. Filled by `xml::parse_uniprot_xml`.

use std::collections::HashMap;

use crate::shared_types::{EnsemblId, UniProtCanonId, UniProtId};

#[derive(Debug, Default)]
pub(crate) struct UniProtEntry {
    /// `<accession>` elements in document order; the first one is primary.
    pub(crate) accessions: Vec<String>,
    /// `<protein><recommendedName><fullName>`
    pub(crate) protein_name: Option<String>,
    /// Primary `<name>` of the first `<gene>`.
    pub(crate) gene_name: Option<String>,
    /// Canonical (displayed) sequence.
    pub(crate) sequence: String,
    /// Splice variant and sequence variant features only.
    pub(crate) features: Vec<Feature>,
    /// The (first) `<comment type="alternative products">`.
    pub(crate) alternative_products: Option<AlternativeProducts>,
    /// `<evidence>` elements, by `key`.
    pub(crate) evidences: HashMap<String, Evidence>,
    /// Top-level `<dbReference type="Ensembl">` elements.
    pub(crate) ensembl_refs: Vec<EnsemblRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeatureKind {
    /// `type="splice variant"` (flat file: VAR_SEQ)
    SpliceVariant,
    /// `type="sequence variant"` (flat file: VARIANT)
    SequenceVariant,
}

#[derive(Debug, Clone)]
pub(crate) struct Feature {
    pub(crate) kind: FeatureKind,
    /// e.g. `VSP_006532`, `VAR_012345`
    pub(crate) id: Option<String>,
    pub(crate) description: Option<String>,
    /// `<original>`; absent for deletions ("Missing").
    pub(crate) original: Option<String>,
    /// `<variation>` elements; empty for deletions ("Missing").
    pub(crate) variations: Vec<String>,
    /// 1-based, `None` if the position is unknown.
    pub(crate) begin: Option<usize>,
    /// 1-based inclusive, `None` if the position is unknown.
    pub(crate) end: Option<usize>,
    /// `<location sequence="P12345-2">`: the isoform the location refers to,
    /// `None` for the canonical sequence.
    pub(crate) location_sequence: Option<String>,
    /// Keys into `UniProtEntry::evidences` (`evidence="1 2"`).
    pub(crate) evidence_keys: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct AlternativeProducts {
    /// e.g. `alternative splicing`
    pub(crate) events: Vec<String>,
    pub(crate) isoforms: Vec<Isoform>,
    /// Free-text `<text>` notes.
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Isoform {
    /// e.g. `P04637-2`; the first one is used.
    pub(crate) ids: Vec<String>,
    /// The first name is the isoform name, any further ones are synonyms.
    pub(crate) names: Vec<String>,
    pub(crate) sequence_type: IsoformSequenceType,
    /// VSP ids from `<sequence type="described" ref="VSP_1 VSP_2"/>`.
    pub(crate) vsp_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum IsoformSequenceType {
    Displayed,
    Described,
    NotDescribed,
    External,
    #[default]
    Missing,
    Other(String),
}

impl IsoformSequenceType {
    pub(crate) fn parse(s: &str) -> Self {
        match s {
            "displayed" => Self::Displayed,
            "described" => Self::Described,
            "not described" => Self::NotDescribed,
            "external" => Self::External,
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Evidence {
    /// ECO code, e.g. `ECO:0000303`
    pub(crate) code: String,
    pub(crate) source: Option<EvidenceSource>,
}

#[derive(Debug, Clone)]
pub(crate) enum EvidenceSource {
    /// `<source><dbReference type="PubMed" id="12345"/></source>`
    DbRef { db: String, id: String },
    /// `<source ref="3"/>`: a reference cited within the entry.
    Ref(String),
}

#[derive(Debug, Clone)]
pub(crate) struct EnsemblRef {
    /// Transcript id as written, possibly versioned (`ENST00000269305.9`).
    pub(crate) transcript_id: String,
    /// `<molecule id="P04637-1"/>`: the isoform the transcript encodes,
    /// `None` for the canonical sequence.
    pub(crate) molecule: Option<String>,
}

impl UniProtEntry {
    /// Primary accession. Entries without accession are rejected by the parser.
    pub(crate) fn primary_accession(&self) -> &str {
        &self.accessions[0]
    }

    pub(crate) fn sequence(&self) -> &str {
        &self.sequence
    }

    pub(crate) fn protein_name(&self) -> &str {
        self.protein_name.as_deref().unwrap_or("Unknown")
    }

    pub(crate) fn gene_name(&self) -> &str {
        self.gene_name.as_deref().unwrap_or("Unknown")
    }

    pub(crate) fn var_seq_features(&self) -> impl Iterator<Item = &Feature> {
        self.features
            .iter()
            .filter(|f| f.kind == FeatureKind::SpliceVariant)
    }

    pub(crate) fn natural_variant_features(&self) -> impl Iterator<Item = &Feature> {
        self.features
            .iter()
            .filter(|f| f.kind == FeatureKind::SequenceVariant)
    }

    /// Ensembl transcript cross-references of this entry, each paired with the
    /// UniProt id it points to. A cross-reference without a `<molecule>` points
    /// to the canonical accession.
    pub(crate) fn ensembl_mapping(&self) -> Vec<(UniProtId, EnsemblId)> {
        self.ensembl_refs
            .iter()
            .map(|r| {
                let uniprot_id = match &r.molecule {
                    Some(accession) => UniProtId::from_accession(accession),
                    None => UniProtId::Canonical(UniProtCanonId::new(self.primary_accession())),
                };
                (uniprot_id, EnsemblId::from_versioned(&r.transcript_id))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/test_model.rs"]
mod tests;
