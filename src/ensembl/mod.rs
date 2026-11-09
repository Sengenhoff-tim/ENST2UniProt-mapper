//! Ensembl protein sequences, read from a local FASTA with an optional REST
//! API fallback.

mod api;
mod fasta;

pub(crate) use api::fetch_ensembl_sequences;
pub(crate) use fasta::load_ensembl_fasta;
