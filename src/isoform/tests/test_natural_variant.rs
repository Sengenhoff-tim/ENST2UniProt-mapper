use crate::shared_types::UniProtIsoId;
use crate::test_support::feature;
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
