use super::*;
use crate::test_support::{variant, write_file};

async fn run_fallback(ensts: &[&str], fasta: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let fasta = write_file(&dir, "pep.fa", fasta);
    let sample_variants: HashMap<EnsemblId, Vec<Variant>> = [(
        EnsemblId::new("ENST1"),
        vec![
            variant("ENST1.1_0", 2, 2, Some("A"), Some("C")),
            variant("ENST1.1_1", 9, 9, Some("K"), Some("E")),
        ],
    )]
    .into();
    let mut out = Vec::new();
    handle_unmapped_transcripts(
        &mut out,
        ensts.iter().map(|e| EnsemblId::new(*e)).collect(),
        &sample_variants,
        Some(&fasta),
        false,
        &RetryConfig::default(),
    )
    .await
    .unwrap();
    String::from_utf8(out).unwrap()
}

#[tokio::test]
async fn writes_an_entry_per_transcript_found_in_the_fasta() {
    let out = run_fallback(&["ENST1"], ">ENST1.1\nMACDE\n").await;
    assert!(out.contains("ID   ENST1                   Reviewed;         5 AA."));
}

#[tokio::test]
async fn transcripts_without_fitting_variants_get_no_entry() {
    // ENST2 has no sample variants; ENST1's variants (at 2 and 9) lie outside "M".
    let out = run_fallback(&["ENST1", "ENST2"], ">ENST1.1\nM\n>ENST2.1\nMKK\n").await;
    assert!(out.is_empty(), "{out}");
}

#[tokio::test]
async fn keeps_variants_that_fit_the_ensembl_sequence() {
    let out = run_fallback(&["ENST1"], ">ENST1.1\nMACDE\n").await;
    assert!(out.contains("/id=\"ENST1.1_0\""));
    // position 9 is outside the 5-residue sequence
    assert!(!out.contains("/id=\"ENST1.1_1\""));
}

#[tokio::test]
async fn transcripts_without_sequence_are_left_out() {
    let out = run_fallback(&["ENST1", "ENST404"], ">ENST1.1\nMACDE\n").await;
    assert!(!out.contains("ENST404"));
}

#[tokio::test]
async fn nothing_to_do_without_transcripts() {
    assert!(run_fallback(&[], ">ENST1.1\nMACDE\n").await.is_empty());
}
