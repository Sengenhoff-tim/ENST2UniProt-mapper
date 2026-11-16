use tracing::info;

use crate::shared_types::UniProtIsoId;
use crate::variant::Variant;

use super::{Feature, UniProtEntry};

const STAGE: &str = "NATURAL VARIANT MAPPING";

impl UniProtEntry {
    /// The entry's natural variant (VARIANT) features as `Variant`s.
    /// Features that can't be mapped are skipped and logged.
    pub(crate) fn natural_variants(&self) -> Vec<Variant> {
        let accession = self.primary_accession();
        self.natural_variant_features()
            .filter_map(|f| f.to_natural_variant(accession))
            .collect()
    }
}

impl Feature {
    /// Converts a UniProt natural variant (VARIANT) feature into a `Variant`.
    /// Returns `None` (and logs why) when the feature can't be mapped.
    ///
    /// `aa_ref == None && aa_new == None` encodes missing sequence.
    fn to_natural_variant(&self, accession: &str) -> Option<Variant> {
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
        info!(
            stage = STAGE,
            accession,
            subject = label,
            outcome,
            rejected = true,
            "{}: {}",
            STAGE,
            outcome,
        );
    }
}

#[cfg(test)]
#[path = "tests/test_natural_variant.rs"]
mod tests;
