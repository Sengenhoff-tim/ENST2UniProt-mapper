use crate::shared_types::{Sequence, UniProtIsoId};
use crate::uniprot::{Feature, Isoform, IsoformSequenceType, UniProtEntry};

use super::{
    log::{Stage, log_entry_event, log_entry_rejection},
    var_seq::apply_var_seq_features,
};

impl UniProtEntry {
    /// Reconstruct a single isoform's sequence from its VAR_SEQ features.
    /// Returns `None` (and logs why) when the isoform can't be reconstructed.
    pub(super) fn reconstruct_isoform(
        &self,
        isoform_id: &UniProtIsoId,
        iso: &Isoform,
    ) -> Option<Sequence> {
        let accession = self.primary_accession();
        // An isoform that should have been reconstructable but wasn't.
        let skip = |outcome: &str| {
            log_entry_rejection(
                Stage::IsoformReconstruction,
                accession,
                isoform_id.as_str(),
                outcome,
            );
            None
        };
        // An isoform whose sequence UniProt does not describe here, by design.
        let not_applicable = |outcome: &str| {
            log_entry_event(
                Stage::IsoformReconstruction,
                accession,
                isoform_id.as_str(),
                outcome,
            );
            None
        };

        match &iso.sequence_type {
            IsoformSequenceType::Described => {
                let features: Vec<&Feature> = iso
                    .vsp_ids
                    .iter()
                    .filter_map(|id| {
                        self.var_seq_features()
                            .find(|f| f.id.as_deref() == Some(id.as_str()))
                    })
                    .collect();

                if iso.vsp_ids.is_empty() || features.len() != iso.vsp_ids.len() {
                    return skip(&format!(
                        "skipped, isoform references {} VSP id(s) but only {} were found among this entry's splice variant features",
                        iso.vsp_ids.len(),
                        features.len()
                    ));
                }

                match apply_var_seq_features(self.sequence(), &features) {
                    Ok(seq) => Some(Sequence::new(seq)),
                    Err(e) => skip(&format!("skipped, failed to reconstruct isoform: {:?}", e)),
                }
            }
            IsoformSequenceType::NotDescribed => not_applicable(
                "skipped, isoform sequence status is 'not described' (no VAR_SEQ features to apply)",
            ),
            IsoformSequenceType::External => not_applicable(
                "skipped, isoform sequence status is 'external' (sequence lives in another entry)",
            ),
            IsoformSequenceType::Displayed => None,
            IsoformSequenceType::Missing => skip("skipped, isoform has no sequence status"),
            IsoformSequenceType::Other(other) => skip(&format!(
                "skipped, unknown/unsupported isoform sequence status '{}'",
                other
            )),
        }
    }
}

#[cfg(test)]
#[path = "tests/test_reconstruct.rs"]
mod tests;
