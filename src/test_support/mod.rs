//! Shared fixtures for unit tests.

mod uniprot_xml;
mod util;
mod variants;

pub(crate) use uniprot_xml::{
    SEQ, minimal_entry_xml, parse_entries, parse_entry, rich_entry, rich_entry_xml, uniprot_xml,
};
pub(crate) use util::{strs, write_file};
pub(crate) use variants::{feature, iso_variant, variant};
