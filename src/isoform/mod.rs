//! Isoform sequence reconstruction (used to compute isoform-local variant
//! positions; the isoforms themselves are not written as separate synthetic
//! entries).

mod collect;
mod log;
mod reconstruct;
mod var_seq;

pub(crate) use collect::{EntryIsoforms, collect_and_reconstruct_isoforms};
