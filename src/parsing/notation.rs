use crate::variant::Variant;

/// Parses `<pos><ref_aa>` style position/residue notation.
/// e.g. "726G" -> (726, "G")
fn split_position_and_residues(s: &str) -> Option<(usize, &str)> {
    let digit_count = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }
    let (pos_str, residues) = s.split_at(digit_count);
    let pos: usize = pos_str.parse().ok()?;
    Some((pos, residues))
}

/// Checks that the position on the left of `>` matches the position on the right.
/// e.g. "726G>726S" -> Some(true) (positions match: 726 == 726)
/// e.g. "726G>731S" -> Some(false) (positions differ: 726 != 731)
/// e.g. "garbage!!" -> None (couldn't parse either side)
pub(super) fn positions_match(variant_str: &str) -> Option<bool> {
    let (left, right) = variant_str.split_once('>')?;
    let (begin, _) = split_position_and_residues(left)?;
    let (rpos, _) = split_position_and_residues(right)?;
    Some(begin == rpos)
}

/// Parses a protein variant string of the form `<pos><ref_aa>><pos><alt_aa>`.
/// e.g. "726G>726S" -> Some(Variant { begin: 726, end: 726, aa_ref: "G", aa_new: "S" })
pub(super) fn to_variant(id: &str, variant_str: &str) -> Option<Variant> {
    let (left, right) = variant_str.split_once('>')?;
    let (begin, raw_wt) = split_position_and_residues(left)?;
    let (_rpos, raw_mt) = split_position_and_residues(right)?;
    let end = begin + raw_wt.len().checked_sub(1)?;
    Some(Variant {
        isoform: None,
        id: id.to_string(),
        begin,
        end,
        aa_ref: Some(raw_wt.to_string()),
        aa_new: Some(raw_mt.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_position_from_residues() {
        assert_eq!(split_position_and_residues("726G"), Some((726, "G")));
        assert_eq!(split_position_and_residues("12AB*"), Some((12, "AB*")));
        assert_eq!(split_position_and_residues("G726"), None);
    }

    #[test]
    fn positions_match_compares_both_sides() {
        assert_eq!(positions_match("726G>726S"), Some(true));
        assert_eq!(positions_match("726G>731S"), Some(false));
        assert_eq!(positions_match("garbage!!"), None);
    }

    #[test]
    fn single_residue_substitution() {
        let v = to_variant("ENST1_0", "726G>726S").unwrap();
        assert_eq!((v.id.as_str(), v.begin, v.end), ("ENST1_0", 726, 726));
        assert_eq!((v.aa_ref.as_deref(), v.aa_new.as_deref()), (Some("G"), Some("S")));
        assert_eq!(v.isoform, None);
    }

    #[test]
    fn multi_residue_reference_spans_its_length() {
        let v = to_variant("x", "2AA>2B").unwrap();
        assert_eq!((v.begin, v.end), (2, 3));
        assert_eq!((v.aa_ref.as_deref(), v.aa_new.as_deref()), (Some("AA"), Some("B")));
    }

    #[test]
    fn stop_markers_are_kept_verbatim() {
        let v = to_variant("x", "3C>3*").unwrap();
        assert_eq!((v.aa_ref.as_deref(), v.aa_new.as_deref()), (Some("C"), Some("*")));
    }

    #[test]
    fn rejects_malformed_notation() {
        assert!(to_variant("x", "726G").is_none(), "no '>'");
        assert!(to_variant("x", "G>S").is_none(), "no position");
        assert!(to_variant("x", "726>726S").is_none(), "empty reference");
    }
}
