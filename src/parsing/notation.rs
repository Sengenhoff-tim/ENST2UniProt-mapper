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

/// Why a variant string could not be parsed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum NotationError {
    /// Not of the form `<pos><ref_aa>><pos><alt_aa>`.
    Malformed,
    /// The positions left and right of `>` differ, e.g. "726G>731S".
    PositionMismatch,
}

/// Parses a protein variant string of the form `<pos><ref_aa>><pos><alt_aa>`.
/// e.g. "726G>726S" -> Ok(Variant { begin: 726, end: 726, aa_ref: "G", aa_new: "S" })
/// e.g. "726G>731S" -> Err(PositionMismatch); "garbage!!" -> Err(Malformed)
pub(super) fn parse_notation(id: &str, variant_str: &str) -> Result<Variant, NotationError> {
    let (left, right) = variant_str
        .split_once('>')
        .ok_or(NotationError::Malformed)?;
    let (begin, raw_wt) = split_position_and_residues(left).ok_or(NotationError::Malformed)?;
    let (rpos, raw_mt) = split_position_and_residues(right).ok_or(NotationError::Malformed)?;
    if begin != rpos {
        return Err(NotationError::PositionMismatch);
    }
    let end = begin
        + raw_wt
            .len()
            .checked_sub(1)
            .ok_or(NotationError::Malformed)?;
    Ok(Variant {
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
