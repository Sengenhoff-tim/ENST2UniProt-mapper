//! Core domain types shared across modules.
//!
//! Identifiers and sequences are thin wrappers around `String`, so that e.g.
//! a transcript id cannot be passed where a UniProt accession is expected.
//! They compare and hash like the wrapped string and implement `Borrow<str>`,
//! so maps and sets keyed by them can be queried with a plain `&str`.

mod ensembl_id;
mod newtype;
mod sequence;
mod uniprot_id;

pub(crate) use ensembl_id::EnsemblId;
pub(crate) use sequence::Sequence;
pub(crate) use uniprot_id::{UniProtCanonId, UniProtId, UniProtIsoId};
