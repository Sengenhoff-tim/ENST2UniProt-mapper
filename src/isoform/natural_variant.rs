use crate::types::UniProtIsoId;
use crate::uniprot::Feature;
use crate::variant::Variant;

use super::log::{Stage, log_entry_rejection};

impl Feature {
    /// Converts a UniProt natural variant (VARIANT) feature into a `Variant`.
    /// Returns `None` (and logs why) when the feature can't be mapped.
    ///
    /// `aa_ref == None && aa_new == None` encodes missing sequence.
    pub(super) fn to_natural_variant(&self, accession: &str) -> Option<Variant> {
        let Some(begin) = self.begin else {
            self.log_unmapped(accession, "cannot be mapped: No position");
            return None;
        };

        let mut aa_ref = None;
        let mut aa_new = None;

        // original and variation must both be set, otherwise the variant
        // encodes missing sequence
        if let Some(original) = &self.original
            && !self.variations.is_empty()
        {
            if self.variations.len() > 1 {
                self.log_unmapped(
                    accession,
                    "cannot be mapped: Alternative sequence is ambiguous",
                );
                return None;
            }
            aa_new = self.variations.first().cloned();
            aa_ref = Some(original.clone());
        }

        Some(Variant {
            // e.g. `<location sequence="P12345-2">` for isoform-specific variants
            isoform: self.location_sequence.as_ref().map(UniProtIsoId::new),
            id: self.id.clone().unwrap_or_default(),
            begin,
            // end defaults to begin if unknown
            end: self.end.unwrap_or(begin),
            aa_ref,
            aa_new,
        })
    }

    fn log_unmapped(&self, accession: &str, outcome: &str) {
        // Feature id, falling back to the description.
        let label = self
            .id
            .as_deref()
            .or(self.description.as_deref())
            .unwrap_or("No Identifier or Description");
        log_entry_rejection(Stage::NaturalVariantMapping, accession, label, outcome);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::feature;
    use crate::types::UniProtIsoId;
    use crate::uniprot::FeatureKind::SequenceVariant;

    #[test]
    fn maps_a_substitution() {
        let v = feature(SequenceVariant, "VAR_1", 9, 9, Some("K"), &["E"])
            .to_natural_variant("P1")
            .unwrap();
        assert_eq!((v.id.as_str(), v.begin, v.end), ("VAR_1", 9, 9));
        assert_eq!(
            (v.aa_ref.as_deref(), v.aa_new.as_deref()),
            (Some("K"), Some("E"))
        );
        assert_eq!(v.isoform, None);
    }

    #[test]
    fn without_variation_it_encodes_missing_sequence() {
        let v = feature(SequenceVariant, "VAR_1", 3, 5, Some("DEF"), &[])
            .to_natural_variant("P1")
            .unwrap();
        assert_eq!((v.begin, v.end, v.aa_ref, v.aa_new), (3, 5, None, None));
    }

    #[test]
    fn takes_the_isoform_from_the_location() {
        let mut f = feature(SequenceVariant, "VAR_1", 5, 5, Some("G"), &["A"]);
        f.location_sequence = Some("P1-2".to_string());
        assert_eq!(
            f.to_natural_variant("P1").unwrap().isoform,
            Some(UniProtIsoId::new("P1-2"))
        );
    }

    #[test]
    fn end_defaults_to_begin() {
        let mut f = feature(SequenceVariant, "VAR_1", 5, 5, Some("F"), &["L"]);
        f.end = None;
        assert_eq!(f.to_natural_variant("P1").unwrap().end, 5);
    }

    #[test]
    fn rejects_unknown_position_and_ambiguous_variation() {
        let mut no_position = feature(SequenceVariant, "VAR_1", 5, 5, Some("F"), &["L"]);
        no_position.begin = None;
        assert!(no_position.to_natural_variant("P1").is_none());

        let ambiguous = feature(SequenceVariant, "VAR_2", 5, 5, Some("F"), &["L", "W"]);
        assert!(ambiguous.to_natural_variant("P1").is_none());
    }
}
