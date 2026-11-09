use crate::types::{Sequence, UniProtIsoId};
use crate::uniprot::{Feature, Isoform, IsoformSequenceType, UniProtEntry};

use super::{
    log::{log_entry_event, log_entry_rejection, Stage},
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
            log_entry_rejection(Stage::IsoformReconstruction, accession, isoform_id.as_str(), outcome);
            None
        };
        // An isoform whose sequence UniProt does not describe here, by design.
        let not_applicable = |outcome: &str| {
            log_entry_event(Stage::IsoformReconstruction, accession, isoform_id.as_str(), outcome);
            None
        };

        match &iso.sequence_type {
            IsoformSequenceType::Described => {
                let features: Vec<&Feature> = iso
                    .vsp_ids
                    .iter()
                    .filter_map(|id| self.var_seq_features().find(|f| f.id.as_deref() == Some(id.as_str())))
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
            IsoformSequenceType::External => {
                not_applicable("skipped, isoform sequence status is 'external' (sequence lives in another entry)")
            }
            IsoformSequenceType::Displayed => None,
            IsoformSequenceType::Missing => skip("skipped, isoform has no sequence status"),
            IsoformSequenceType::Other(other) => {
                skip(&format!("skipped, unknown/unsupported isoform sequence status '{}'", other))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rich_entry;

    fn isoform(sequence_type: IsoformSequenceType, vsp_ids: &[&str]) -> Isoform {
        Isoform {
            ids: vec!["P99999-9".to_string()],
            names: Vec::new(),
            sequence_type,
            vsp_ids: vsp_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn reconstruct(iso: &Isoform) -> Option<Sequence> {
        rich_entry().reconstruct_isoform(&UniProtIsoId::new("P99999-9"), iso)
    }

    #[test]
    fn described_isoform_applies_its_vsp_features() {
        let seq = reconstruct(&isoform(IsoformSequenceType::Described, &["VSP_000001", "VSP_000002"]));
        assert_eq!(seq.unwrap().as_str(), "AWEFGHIKLMNPQRST");
    }

    #[test]
    fn described_isoform_with_unknown_or_no_vsp_is_skipped() {
        assert!(reconstruct(&isoform(IsoformSequenceType::Described, &["VSP_000001", "VSP_404"])).is_none());
        assert!(reconstruct(&isoform(IsoformSequenceType::Described, &[])).is_none());
    }

    #[test]
    fn other_sequence_types_are_not_reconstructed() {
        for ty in [
            IsoformSequenceType::Displayed,
            IsoformSequenceType::NotDescribed,
            IsoformSequenceType::External,
            IsoformSequenceType::Missing,
            IsoformSequenceType::Other("new".into()),
        ] {
            assert!(reconstruct(&isoform(ty.clone(), &["VSP_000001"])).is_none(), "{ty:?}");
        }
    }
}
