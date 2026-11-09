use anyhow::{bail, ensure};
use tracing::trace;

use super::{
    Variant,
    log::{Stage, log_variant_event},
};

/// What `resolve_stop` should do. Computed from shared borrows, then applied.
enum Plan {
    Unchanged,
    MapDel,
    Edit(Resolution),
}

/// The new span and replacement. `aa_ref` is not part of it: `apply` always
/// takes it from the sequence over `begin..=end`, so the reference length
/// matches the span by construction.
struct Resolution {
    /// `None` encodes missing sequence.
    aa_new: Option<String>,
    begin: usize,
    end: usize,
    case: &'static str,
}

impl Variant {
    /// Rewrites stop-codon markers (`*`) and EBI's `del` into the target encoding.
    /// Errors carry the full variant description, so callers can log them as-is.
    pub(crate) fn resolve_stop(&mut self, seq: &str) -> anyhow::Result<()> {
        self.check_position(seq)?;

        let plan = match (self.aa_ref.as_deref(), self.aa_new.as_deref()) {
            (None, None) => {
                trace!(variant = %self, "no sequence fields, nothing to resolve");
                Plan::Unchanged
            }
            (None, Some(_)) => bail!("variant {}: aa_new is set but aa_ref is missing", self),
            (Some(aa_ref), None) => self.plan_rewrite(seq, aa_ref, ""),
            (Some(aa_ref), Some(aa_new)) => self.plan_rewrite(seq, aa_ref, aa_new),
        };

        match plan {
            Plan::Unchanged => {}
            Plan::MapDel => {
                self.aa_new = None;
                log_variant_event(
                    Stage::EbiNormalization,
                    self,
                    "mapped EBI 'del' replacement to missing sequence",
                );
            }
            Plan::Edit(res) => {
                let outcome = format!("{} (seq_len {})", res.case, seq.len());
                self.apply(res, seq);
                log_variant_event(Stage::StopCodonResolution, self, &outcome);
            }
        }
        Ok(())
    }

    fn check_position(&self, seq: &str) -> anyhow::Result<()> {
        ensure!(
            !seq.is_empty(),
            "variant {}: isoform sequence is empty, variant omitted",
            self
        );
        ensure!(
            self.begin >= 1 && self.begin <= seq.len(),
            "variant {}: begin {} is outside the sequence (length {}, positions are 1-based)",
            self,
            self.begin,
            seq.len()
        );
        Ok(())
    }

    /// Works out the rewrite for the sequence fields without mutating `self`.
    /// Covers EBI `del` mapping and stop-codon truncation.
    fn plan_rewrite(&self, seq: &str, aa_ref: &str, aa_new: &str) -> Plan {
        if aa_new == "del" {
            return Plan::MapDel;
        }

        let (ref_star, new_star) = find_stops(aa_ref, aa_new);
        if ref_star.is_none() && new_star.is_none() {
            trace!(variant = %self, "no stop-codon markers, nothing to resolve");
            return Plan::Unchanged;
        }

        self.log_unusual_marker(aa_ref, aa_new, ref_star, new_star);
        Plan::Edit(self.build_resolution(seq, aa_new, ref_star, new_star))
    }

    fn log_unusual_marker(
        &self,
        aa_ref: &str,
        aa_new: &str,
        ref_star: Option<usize>,
        new_star: Option<usize>,
    ) {
        if aa_ref.ends_with('*') && aa_new.ends_with('*') {
            return;
        }
        log_variant_event(
            Stage::StopCodonValidation,
            self,
            &format!(
                "flagged stop marker at aa_ref {ref_star:?}, aa_new {new_star:?}; truncating at first '*'"
            ),
        );
    }

    /// Chooses the rewrite case and computes the new field values, without mutating `self`.
    /// Collapsing to an empty replacement encodes missing sequence on both fields.
    fn build_resolution(
        &self,
        seq: &str,
        aa_new: &str,
        ref_star: Option<usize>,
        new_star: Option<usize>,
    ) -> Resolution {
        let new_trunc = truncate_at(aa_new, new_star);
        let mut res = match (ref_star, new_star) {
            (Some(0), _) => self.anchor_on_last_residue(seq, new_trunc),
            (Some(_), _) => self.truncate_reference(seq, new_trunc),
            (None, Some(_)) => self.truncate_at_gained_stop(seq, new_trunc),
            (None, None) => unreachable!("plan_rewrite returns early when no stop is present"),
        };

        if res.aa_new.as_deref() == Some("") {
            res.aa_new = None;
        }
        res
    }

    /// `aa_ref` is only "*": no real reference residue exists here, so synthesize an anchor
    /// from the isoform's last residue. E.g. seq="AAA", ref="*", new="BB*" becomes
    /// ref="A", new="ABB", begin=end=len(seq).
    fn anchor_on_last_residue(&self, seq: &str, new_trunc: &str) -> Resolution {
        let last = seq
            .chars()
            .last()
            .expect("resolve_stop guarantees a non-empty sequence");
        Resolution {
            aa_new: Some(format!("{last}{new_trunc}")),
            begin: seq.len(),
            end: seq.len(),
            case: "extension anchored on last residue",
        }
    }

    /// Residues precede the '*' in `aa_ref`, so the replaced span runs to the isoform's end.
    /// E.g. seq="ABCDE", "2B*>2X" becomes ref="BCDE", new="X", span 2..5.
    fn truncate_reference(&self, seq: &str, new_trunc: &str) -> Resolution {
        Resolution {
            aa_new: Some(new_trunc.to_owned()),
            begin: self.begin,
            end: seq.len(),
            case: "reference stop truncated, span extended to sequence end",
        }
    }

    /// `aa_new` gains a stop, so everything downstream of this position is truncated.
    /// E.g. seq="ABCDE", "1A>1BB*" becomes ref="ABCDE", new="BB", span 1..5.
    fn truncate_at_gained_stop(&self, seq: &str, new_trunc: &str) -> Resolution {
        Resolution {
            aa_new: Some(new_trunc.to_owned()),
            begin: self.begin,
            end: seq.len(),
            case: "stop gained, span extended to sequence end",
        }
    }

    /// Applies `res`; the reference becomes the residues `seq` has over the new span.
    fn apply(&mut self, res: Resolution, seq: &str) {
        // `check_position` guarantees 1 <= begin <= len, and every case ends
        // at begin or at len.
        self.aa_ref = Some(seq[res.begin - 1..res.end].to_string());
        self.aa_new = res.aa_new;
        self.begin = res.begin;
        self.end = res.end;
    }
}

/// Byte indices of the first `*` in each field, if present.
fn find_stops(aa_ref: &str, aa_new: &str) -> (Option<usize>, Option<usize>) {
    (aa_ref.find('*'), aa_new.find('*'))
}

fn truncate_at(s: &str, star: Option<usize>) -> &str {
    match star {
        Some(idx) => &s[..idx],
        None => s,
    }
}
#[cfg(test)]
mod tests {
    use crate::test_support::variant;
    use crate::variant::Variant;

    /// `ABCDE`, as in the cases below.
    const SEQ: &str = "ABCDE";

    fn resolved(mut v: Variant) -> Variant {
        v.resolve_stop(SEQ).expect("resolves");
        v
    }

    fn fields(v: &Variant) -> (usize, usize, Option<&str>, Option<&str>) {
        (v.begin, v.end, v.aa_ref.as_deref(), v.aa_new.as_deref())
    }

    #[test]
    fn plain_substitution_is_unchanged() {
        let v = resolved(variant("v", 2, 2, Some("B"), Some("X")));
        assert_eq!(fields(&v), (2, 2, Some("B"), Some("X")));
    }

    #[test]
    fn missing_sequence_is_unchanged() {
        let v = resolved(variant("v", 2, 3, None, None));
        assert_eq!(fields(&v), (2, 3, None, None));
    }

    #[test]
    fn ebi_del_becomes_missing_sequence() {
        let v = resolved(variant("v", 2, 2, Some("B"), Some("del")));
        assert_eq!(fields(&v), (2, 2, Some("B"), None));
    }

    #[test]
    fn gained_stop_truncates_to_the_sequence_end() {
        // "3C>3*": everything from position 3 on is lost
        let v = resolved(variant("v", 3, 3, Some("C"), Some("*")));
        assert_eq!(fields(&v), (3, 5, Some("CDE"), None));
    }

    #[test]
    fn gained_stop_keeps_residues_before_the_marker() {
        // "1A>1BB*": BB replaces the rest of the sequence
        let v = resolved(variant("v", 1, 1, Some("A"), Some("BB*")));
        assert_eq!(fields(&v), (1, 5, Some("ABCDE"), Some("BB")));
    }

    #[test]
    fn lost_stop_extends_from_the_last_residue() {
        // "1*>1BB*": C-terminal extension anchored on the last residue E
        let v = resolved(variant("v", 1, 1, Some("*"), Some("BB*")));
        assert_eq!(fields(&v), (5, 5, Some("E"), Some("EBB")));
    }

    #[test]
    fn stop_inside_the_reference_extends_the_span_to_the_end() {
        // "2B*>2X": the reference ends at the stop, the span runs to the end
        let v = resolved(variant("v", 2, 2, Some("B*"), Some("X")));
        assert_eq!(fields(&v), (2, 5, Some("BCDE"), Some("X")));
    }

    #[test]
    fn resolved_reference_always_spans_begin_to_end() {
        for (aa_ref, aa_new) in [
            ("A", "*"),
            ("A", "BB*"),
            ("*", "BB*"),
            ("B*", "X"),
            ("BC", "X*Y"),
        ] {
            let v = resolved(variant("v", 2, 2, Some(aa_ref), Some(aa_new)));
            let span = v.end - v.begin + 1;
            assert_eq!(
                v.aa_ref.as_deref().map(str::len),
                Some(span),
                "{aa_ref}>{aa_new}: {v}"
            );
        }
    }

    #[test]
    fn rejects_new_residues_without_reference() {
        assert!(
            variant("v", 2, 2, None, Some("X"))
                .resolve_stop(SEQ)
                .is_err()
        );
    }

    #[test]
    fn rejects_positions_outside_the_sequence() {
        assert!(
            variant("v", 0, 0, Some("A"), Some("X"))
                .resolve_stop(SEQ)
                .is_err()
        );
        assert!(
            variant("v", 6, 6, Some("A"), Some("X"))
                .resolve_stop(SEQ)
                .is_err()
        );
        assert!(
            variant("v", 1, 1, Some("A"), Some("X"))
                .resolve_stop("")
                .is_err()
        );
    }
}
