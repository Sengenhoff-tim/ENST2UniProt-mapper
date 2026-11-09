use super::{
    Variant,
    log::{Stage, log_variant_event},
};

fn is_valid_aa(c: char) -> bool {
    matches!(
        c,
        'G' | 'A'
            | 'S'
            | 'P'
            | 'V'
            | 'T'
            | 'C'
            | 'L'
            | 'I'
            | 'N'
            | 'D'
            | 'Q'
            | 'K'
            | 'E'
            | 'M'
            | 'H'
            | 'F'
            | 'U'
            | 'R'
            | 'Y'
            | 'W'
            | 'O'
            | 'J'
            | 'X'
            | 'Z'
            | 'B'
    )
}

impl Variant {
    pub(crate) fn is_valid(&self) -> bool {
        [("aa_ref", &self.aa_ref), ("aa_new", &self.aa_new)]
            .into_iter()
            .filter_map(|(field, value)| value.as_deref().map(|v| (field, v)))
            .all(|(field, value)| self.check_residues(field, value))
    }

    fn check_residues(&self, field: &str, value: &str) -> bool {
        match value.chars().find(|c| !is_valid_aa(*c)) {
            None => true,
            Some(invalid) => {
                log_variant_event(
                    Stage::VariantValidation,
                    self,
                    &format!("rejected {field} {value:?}: invalid residue {invalid:?}"),
                );
                false
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::test_support::variant;

    #[test]
    fn accepts_amino_acid_codes_and_missing_fields() {
        assert!(variant("v", 1, 1, Some("ACDEFGHIKLMNPQRSTVWY"), Some("UOXBZJ")).is_valid());
        assert!(variant("v", 1, 1, Some("A"), None).is_valid());
        assert!(variant("v", 1, 1, None, None).is_valid());
    }

    #[test]
    fn rejects_non_amino_acid_characters() {
        assert!(!variant("v", 1, 1, Some("A"), Some("*")).is_valid());
        assert!(!variant("v", 1, 1, Some("a"), Some("C")).is_valid());
        assert!(!variant("v", 1, 1, Some("A"), Some("del")).is_valid());
    }
}
