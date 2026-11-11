use std::collections::HashMap;

use crate::shared_types::{Sequence, UniProtIsoId};
use crate::uniprot::{IsoformSequenceType, UniProtEntry};
use crate::variant::Variant;

use super::log::{Stage, log_entry_rejection};

/// Isoform information and natural variants of one UniProt entry.
pub(crate) struct EntryIsoforms {
    /// The isoform id UniProt displays as the canonical sequence (e.g.
    /// `P04637-1`), an alias for the canonical accession.
    pub(crate) displayed_id: Option<UniProtIsoId>,
    /// Sequences of the other isoforms that could be reconstructed.
    pub(crate) sequences: HashMap<UniProtIsoId, Sequence>,
    /// The entry's natural variant (VARIANT) features.
    pub(crate) natural_variants: Vec<Variant>,
}

/// Collect isoform IDs and reconstruct their sequences from the
/// ALTERNATIVE PRODUCTS comment, and convert the entry's natural variant
/// features into `Variant`s. Any isoform or variant that can't be handled is
/// skipped and logged rather than failing the whole entry.
pub(crate) fn collect_and_reconstruct_isoforms(entry: &UniProtEntry) -> EntryIsoforms {
    let accession = entry.primary_accession();

    let natural_variants: Vec<Variant> = entry
        .natural_variant_features()
        .filter_map(|f| f.to_natural_variant(accession))
        .collect();

    let mut displayed_id: Option<UniProtIsoId> = None;
    let mut isoforms = HashMap::new();

    let Some(alt_products) = &entry.alternative_products else {
        return EntryIsoforms {
            displayed_id,
            sequences: isoforms,
            natural_variants,
        };
    };

    for iso in &alt_products.isoforms {
        let isoform_id: Option<UniProtIsoId> = iso.ids.first().map(UniProtIsoId::new);

        if iso.sequence_type == IsoformSequenceType::Displayed {
            // alias for canonical id is not pushed to isoforms
            if let Some(id) = isoform_id {
                displayed_id = Some(id);
            }
            continue;
        }

        let Some(isoform_id) = isoform_id else {
            log_entry_rejection(
                Stage::IsoformReconstruction,
                accession,
                "-",
                "skipped, isoform entry has no isoform id",
            );
            continue;
        };

        if let Some(seq) = entry.reconstruct_isoform(&isoform_id, iso) {
            isoforms.insert(isoform_id, seq);
        }
    }

    EntryIsoforms {
        displayed_id,
        sequences: isoforms,
        natural_variants,
    }
}

#[cfg(test)]
#[path = "tests/test_collect.rs"]
mod tests;
