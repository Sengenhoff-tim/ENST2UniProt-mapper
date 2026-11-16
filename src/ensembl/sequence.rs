/// Ensembl occasionally returns a translated protein sequence with containing '*' stop-codon marker (e.g. for select readthrough transcripts).
/// Sequence is assumed to stop at the first stop codon
pub(super) fn strip_stop_codon(enst: &str, seq: String) -> String {
    match seq.find('*') {
        Some(pos) => {
            tracing::info!(
                enst = enst,
                "{}, sequence contained '*' stop codon at position {}; amino acids after are discarded",
                enst,
                pos
            );
            seq[..pos].to_string()
        }
        None => seq,
    }
}

#[cfg(test)]
#[path = "tests/test_sequence.rs"]
mod tests;
