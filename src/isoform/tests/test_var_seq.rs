use super::*;
use crate::test_support::{SEQ, feature};
use crate::uniprot::FeatureKind::SpliceVariant;

fn apply(features: &[Feature]) -> Result<String> {
    apply_var_seq_features(SEQ, &features.iter().collect::<Vec<_>>())
}

#[test]
fn substitutes_a_range() {
    let seq = apply(&[feature(SpliceVariant, "VSP_1", 2, 3, Some("CD"), &["W"])]).unwrap();
    assert_eq!(seq, "AWEFGHIKLMNPQRSTVWY");
}

#[test]
fn deletes_a_range_without_variation() {
    let seq = apply(&[feature(SpliceVariant, "VSP_1", 18, 20, None, &[])]).unwrap();
    assert_eq!(seq, "ACDEFGHIKLMNPQRST");
}

#[test]
fn applies_several_features_regardless_of_input_order() {
    // An earlier feature changing the length must not shift a later one.
    let seq = apply(&[
        feature(SpliceVariant, "VSP_1", 2, 3, Some("CD"), &["WWWW"]),
        feature(SpliceVariant, "VSP_2", 18, 20, None, &[]),
    ])
    .unwrap();
    assert_eq!(seq, "AWWWWEFGHIKLMNPQRST");
}

#[test]
fn rejects_an_original_that_does_not_match() {
    let err = apply(&[feature(SpliceVariant, "VSP_1", 2, 3, Some("XX"), &["W"])]).unwrap_err();
    assert!(err.to_string().contains("mismatch"), "{err}");
}

#[test]
fn rejects_out_of_range_and_unknown_positions() {
    assert!(apply(&[feature(SpliceVariant, "VSP_1", 0, 1, None, &[])]).is_err());
    assert!(apply(&[feature(SpliceVariant, "VSP_1", 19, 21, None, &[])]).is_err());
    assert!(apply(&[feature(SpliceVariant, "VSP_1", 5, 4, None, &[])]).is_err());

    let mut unknown = feature(SpliceVariant, "VSP_1", 1, 1, None, &[]);
    unknown.begin = None;
    assert!(apply(&[unknown]).is_err());
}
