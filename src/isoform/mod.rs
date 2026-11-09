//! Isoform sequence reconstruction (used to compute isoform-local variant
//! positions; the isoforms themselves are not written as separate synthetic
//! entries) and collection of UniProt "Natural variant" features.

mod collect;
mod log;
mod natural_variant;
mod reconstruct;
mod var_seq;

pub(crate) use collect::{collect_and_reconstruct_isoforms, EntryIsoforms};
