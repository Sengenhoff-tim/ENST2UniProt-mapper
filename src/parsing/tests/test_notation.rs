use super::*;

#[test]
fn splits_position_from_residues() {
    assert_eq!(split_position_and_residues("726G"), Some((726, "G")));
    assert_eq!(split_position_and_residues("12AB*"), Some((12, "AB*")));
    assert_eq!(split_position_and_residues("G726"), None);
}

#[test]
fn single_residue_substitution() {
    let v = parse_notation("ENST1_0", "726G>726S").unwrap();
    assert_eq!((v.id.as_str(), v.begin, v.end), ("ENST1_0", 726, 726));
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("G"), Some("S"))
    );
    assert_eq!(v.isoform, None);
}

#[test]
fn multi_residue_reference_spans_its_length() {
    let v = parse_notation("x", "2AA>2B").unwrap();
    assert_eq!((v.begin, v.end), (2, 3));
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("AA"), Some("B"))
    );
}

#[test]
fn stop_markers_are_kept_verbatim() {
    let v = parse_notation("x", "3C>3*").unwrap();
    assert_eq!(
        (v.aa_ref.as_deref(), v.aa_new.as_deref()),
        (Some("C"), Some("*"))
    );
}

#[test]
fn rejects_malformed_notation() {
    let err = |s| parse_notation("x", s).unwrap_err();
    assert_eq!(err("726G"), NotationError::Malformed, "no '>'");
    assert_eq!(err("G>S"), NotationError::Malformed, "no position");
    assert_eq!(err("726>726S"), NotationError::Malformed, "empty reference");
    assert_eq!(err("garbage!!"), NotationError::Malformed);
}

#[test]
fn rejects_differing_positions() {
    assert_eq!(
        parse_notation("x", "726G>731S").unwrap_err(),
        NotationError::PositionMismatch
    );
}
