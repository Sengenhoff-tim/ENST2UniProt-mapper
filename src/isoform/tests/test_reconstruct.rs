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
    let seq = reconstruct(&isoform(
        IsoformSequenceType::Described,
        &["VSP_000001", "VSP_000002"],
    ));
    assert_eq!(seq.unwrap().as_str(), "AWEFGHIKLMNPQRST");
}

#[test]
fn described_isoform_with_unknown_or_no_vsp_is_skipped() {
    assert!(
        reconstruct(&isoform(
            IsoformSequenceType::Described,
            &["VSP_000001", "VSP_404"]
        ))
        .is_none()
    );
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
        assert!(
            reconstruct(&isoform(ty.clone(), &["VSP_000001"])).is_none(),
            "{ty:?}"
        );
    }
}
