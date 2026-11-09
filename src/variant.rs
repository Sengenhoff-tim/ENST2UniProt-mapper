use std::fmt;

use anyhow::{bail, ensure};
use tracing::{debug, info, trace};

use crate::types::UniProtIsoId;

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    pub(crate) isoform: Option<UniProtIsoId>,
    pub(crate) id: String,
    /// 1-based, inclusive.
    pub(crate) begin: usize,
    /// 1-based, inclusive.
    pub(crate) end: usize,
    pub(crate) aa_ref: Option<String>,
    /// `None` encodes missing sequence.
    pub(crate) aa_new: Option<String>,
}

// Skips the id field.
impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        self.isoform == other.isoform
            && self.begin == other.begin
            && self.end == other.end
            && self.aa_ref == other.aa_ref
            && self.aa_new == other.aa_new
    }
}

impl Eq for Variant {}

// Skips the id field, consistent with PartialEq.
impl std::hash::Hash for Variant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.isoform.hash(state);
        self.begin.hash(state);
        self.end.hash(state);
        self.aa_ref.hash(state);
        self.aa_new.hash(state);
    }
}

/// Compact one-line form used in logs and error messages, e.g.
/// `EBI:1929:G (canonical:1929-1929 G>W)`. A `-` stands for a `None` field.
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}:{}-{} {}>{})",
            self.id,
            self.isoform_label(),
            self.begin,
            self.end,
            self.aa_ref.as_deref().unwrap_or("-"),
            self.aa_new.as_deref().unwrap_or("-"),
        )
    }
}

impl Variant {
    fn isoform_label(&self) -> &str {
        self.isoform.as_ref().map_or("canonical", |iso| iso.as_str())
    }

    /// Merge two identical variants by joining their IDs with `_`.
    pub(crate) fn merge(&mut self, other: &Variant) {
        debug_assert!(self == other, "merge called on non-identical variants");
        trace!(variant = %self, other_id = %other.id, "merging duplicate variant");
        if self.id.is_empty() {
            self.id = other.id.clone();
        } else {
            self.id.push('_');
            self.id.push_str(&other.id);
        }
    }
}

fn is_valid_aa(c: char) -> bool {
    matches!(
        c,
        'G' | 'A' | 'S' | 'P' | 'V' | 'T' | 'C' | 'L' | 'I' | 'N' | 'D' | 'Q' | 'K'
            | 'E' | 'M' | 'H' | 'F' | 'U' | 'R' | 'Y' | 'W' | 'O' | 'J' | 'X' | 'Z' | 'B'
    )
}

impl Variant {
    pub(crate) fn is_valid(&self) -> bool {
        for (field, value) in [("aa_ref", &self.aa_ref), ("aa_new", &self.aa_new)] {
            let Some(value) = value else { continue };
            if let Some(invalid) = value.chars().find(|c| !is_valid_aa(*c)) {
                info!(
                    variant = %self,
                    field,
                    value = %value,
                    invalid = %invalid,
                    "variant contains an invalid amino-acid code"
                );
                return false;
            }
        }
        true
    }
}

impl Variant {
    /// Rewrites stop-codon markers (`*`) and EBI's `del` into the target encoding.
    /// Errors carry the full variant description, so callers can log them as-is.
    pub(crate) fn resolve_stop(&mut self, seq: &str) -> anyhow::Result<()> {
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

        match (self.aa_ref.clone(), self.aa_new.clone()) {
            // Neither field present: nothing to resolve.
            (None, None) => {
                trace!(variant = %self, "no sequence fields, nothing to resolve");
                Ok(())
            }

            // A replacement without a reference cannot be placed.
            (None, Some(_)) => bail!("variant {}: aa_new is set but aa_ref is missing", self),

            // Only aa_ref present: replacement is missing sequence.
            (Some(aa_ref), None) => self.resolve_star(seq, &aa_ref, ""),

            (Some(aa_ref), Some(aa_new)) => self.resolve_star(seq, &aa_ref, &aa_new),
        }
    }

    fn resolve_star(&mut self, seq: &str, aa_ref: &str, aa_new: &str) -> anyhow::Result<()> {
        // "del": alternative missing-sequence encoding in EBI. Positions are assumed to stay correct.
        if aa_new == "del" {
            debug!(variant = %self, "EBI 'del' replacement mapped to missing sequence");
            self.aa_new = None;
            return Ok(());
        }

        let ref_star = aa_ref.find('*');
        let new_star = aa_new.find('*');

        if ref_star.is_none() && new_star.is_none() {
            return Ok(());
        }

        // Expected shape: both fields end in a single trailing '*'.
        // Anything else (mid-string '*', lone '*' ref, asymmetric '*') is
        // unusual enough to call out explicitly.
        let is_common_case = aa_ref.ends_with('*') && aa_new.ends_with('*');
        if !is_common_case {
            info!(
                variant = %self,
                aa_ref,
                aa_new,
                ref_star = ?ref_star,
                new_star = ?new_star,
                "unusual stop-codon marker position; truncating at first '*'"
            );
        }

        let before = self.to_string();
        let case = self.apply_stop_truncation(seq, aa_ref, aa_new, ref_star, new_star);

        // If the replacement collapsed to nothing, encode that as missing sequence on both fields.
        let collapsed = self.aa_new.as_deref() == Some("");
        if collapsed {
            self.aa_ref = None;
            self.aa_new = None;
        }

        debug!(
            before = %before,
            after = %self,
            case,
            collapsed,
            seq_len = seq.len(),
            "resolved stop-codon marker"
        );
        Ok(())
    }

    /// Given the positions of the first '*' in `aa_ref`/`aa_new` (if any),
    /// truncate the fields and adjust begin/end to fit the target encoding.
    /// Returns a short name of the case applied, for logging.
    /// Precondition: at least one of `ref_star`, `new_star` is `Some`, and `seq` is non-empty.
    fn apply_stop_truncation(
        &mut self,
        seq: &str,
        aa_ref: &str,
        aa_new: &str,
        ref_star: Option<usize>,
        new_star: Option<usize>,
    ) -> &'static str {
        let new_trunc = match new_star {
            Some(idx) => &aa_new[..idx],
            None => aa_new,
        };

        match (ref_star, new_star) {
            // aa_ref is only "*": no real reference residue exists at this
            // position (downstream can't append), so synthesize an anchor
            // from the isoform's last residue and fold the insertion into
            // aa_new: e.g. seq="AAA", ref="*", new="BB*" -> ref="A", new="ABB",
            // begin=end=len(seq), i.e. "replace the last A with ABB".
            (Some(0), _) => {
                let last = seq
                    .chars()
                    .last()
                    .expect("resolve_stop guarantees a non-empty sequence");
                self.aa_ref = Some(last.to_string());
                self.aa_new = Some(format!("{last}{new_trunc}"));
                self.begin = seq.len();
                self.end = seq.len();
                "extension anchored on last residue"
            }

            // Residues precede the '*' in aa_ref (idx > 0 guaranteed by match
            // order): the replaced span runs to the true end of the isoform.
            (Some(idx), _) => {
                self.aa_ref = Some(aa_ref[..idx].to_string());
                self.aa_new = Some(new_trunc.to_string());
                self.end = seq.len();
                "reference stop truncated, span extended to sequence end"
            }

            // No '*' in aa_ref, but aa_new gains a stop: everything downstream
            // of this position in the isoform is truncated.
            (None, Some(_)) => {
                self.aa_new = Some(new_trunc.to_string());
                self.end = seq.len();
                "stop gained, span extended to sequence end"
            }

            (None, None) => unreachable!("caller guarantees at least one of ref_star/new_star is Some"),
        }
    }
}

pub(crate) trait JoinVariants {
    fn join(&mut self, variants_b: Vec<Variant>);
}

impl JoinVariants for Vec<Variant> {
    fn join(&mut self, variants_b: Vec<Variant>) {
        let incoming = variants_b.len();
        let mut merged = 0usize;
        for variant_b in variants_b {
            if let Some(variant_a) = self.iter_mut().find(|v| **v == variant_b) {
                variant_a.merge(&variant_b);
                merged += 1;
            } else {
                self.push(variant_b);
            }
        }
        debug!(
            incoming,
            merged,
            added = incoming - merged,
            total = self.len(),
            "joined variant lists"
        );
    }
}