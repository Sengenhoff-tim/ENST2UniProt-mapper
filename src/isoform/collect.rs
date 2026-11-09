use std::collections::HashMap;

use crate::types::{Sequence, UniProtIsoId};
use crate::uniprot::{IsoformSequenceType, UniProtEntry};
use crate::variant::Variant;

use super::log::{log_entry_rejection, Stage};

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
        return EntryIsoforms { displayed_id, sequences: isoforms, natural_variants };
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

    EntryIsoforms { displayed_id, sequences: isoforms, natural_variants }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{minimal_entry_xml, parse_entry, rich_entry};

    #[test]
    fn collects_displayed_alias_isoforms_and_natural_variants() {
        let EntryIsoforms { displayed_id, sequences, natural_variants } =
            collect_and_reconstruct_isoforms(&rich_entry());

        assert_eq!(displayed_id, Some(UniProtIsoId::new("P99999-1")));
        // -1 is the canonical alias and -3 is not described: only -2 remains.
        assert_eq!(sequences.len(), 1);
        assert_eq!(sequences[&UniProtIsoId::new("P99999-2")].as_str(), "AWEFGHIKLMNPQRST");

        let ids: Vec<&str> = natural_variants.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["VAR_000001", "VAR_000002"]);
    }

    #[test]
    fn entry_without_alternative_products_has_no_isoforms() {
        let entry = parse_entry(&minimal_entry_xml(
            "P1",
            r#"<feature type="sequence variant" id="VAR_1"><original>K</original><variation>E</variation><location><position position="9"/></location></feature>"#,
        ));
        let collected = collect_and_reconstruct_isoforms(&entry);
        assert_eq!(collected.displayed_id, None);
        assert!(collected.sequences.is_empty());
        assert_eq!(collected.natural_variants.len(), 1);
    }
}
