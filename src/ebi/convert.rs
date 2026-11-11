use crate::shared_types::UniProtIsoId;
use crate::variant::Variant;

use super::{
    log::{log_feature_event, warn_feature_event, Stage},
    types::EbiFeature,
};

impl EbiFeature {
    /// Builds a variant on isoform `accession`.
    pub(crate) fn to_iso_variant(&self, accession: &UniProtIsoId) -> Option<Variant> {
        self.to_variant(accession.as_str(), Some(accession.clone()), |begin, replaced| {
            format!("EBI{}_{}_{}", self.ft_id.as_deref().unwrap_or(""), begin, replaced)
        })
    }

    /// Builds a variant on the canonical sequence of `accession`.
    pub(crate) fn to_canon_variant(&self, accession: &str) -> Option<Variant> {
        self.to_variant(accession, None, |begin, replaced| format!("EBI:{}:{}", begin, replaced))
    }

    /// Returns `None` (and logs why) when the feature can't be converted.
    fn to_variant(
        &self,
        accession: &str,
        isoform: Option<UniProtIsoId>,
        make_id: impl FnOnce(usize, &str) -> String,
    ) -> Option<Variant> {
        let Some(replaced) = &self.wild_type else {
            log_feature_event(
                Stage::EbiConversion,
                accession,
                self,
                "skipped, no wild-type residue given",
            );
            return None;
        };

        let Ok(begin) = self.begin.parse::<usize>() else {
            warn_feature_event(
                Stage::EbiConversion,
                accession,
                self,
                "skipped, begin is not a valid position",
            );
            return None;
        };

        let Some(len_minus_one) = replaced.len().checked_sub(1) else {
            warn_feature_event(
                Stage::EbiConversion,
                accession,
                self,
                "skipped, wild-type sequence is empty",
            );
            return None;
        };

        Some(Variant {
            isoform,
            id: make_id(begin, replaced),
            begin,
            end: begin + len_minus_one,
            aa_ref: Some(replaced.clone()),
            aa_new: self.mutated_type.clone(),
        })
    }
}
