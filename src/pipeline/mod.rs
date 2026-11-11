//! Workflow:
//! 1) Parse the ensured accessions and the ENST sample variant file; the
//!    ensured accessions (extended via the id mapping, minus the excluded
//!    accessions) are the entries to ensure in the output
//! 2) Load all entries of the UniProt XML file (minus the excluded accessions);
//!    ensured accessions missing from it are fetched from the UniProt REST API if
//!    enabled, and logged otherwise — provides canonical sequence,
//!    isoform VAR_SEQ features, CC ALTERNATIVE PRODUCTS, and ENST→isoform
//!    cross-references
//! 3) Build ENST→isoform mapping from cross-references per entry
//! 4) Reconstruct isoform sequences by applying VAR_SEQ features to canonical
//! 5) Optionally (disabled by default) fetch EBI variation data for all
//!    canonical + isoform accessions
//! 6) Join EBI variants with ENST-derived variants per isoform; write output
//! 7) ENSTs unmapped to any UniProt entry → look up the sequence in the Ensembl
//!    FASTA (optionally falling back to the Ensembl REST API); emit synthetic
//!    flat-file entries

mod entry;
mod fallback;
mod input;
mod log;
mod run;
mod variants;

pub(crate) use run::{RunOptions, run};
