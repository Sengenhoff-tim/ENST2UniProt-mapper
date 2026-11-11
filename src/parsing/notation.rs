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
#[path = "tests/test_notation.rs"]
mod tests;
