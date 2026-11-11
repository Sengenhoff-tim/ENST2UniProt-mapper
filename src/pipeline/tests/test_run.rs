use super::*;
use crate::test_support::{minimal_entry_xml, rich_entry_xml, uniprot_xml, write_file};

/// Runs the whole pipeline offline (no API fallbacks) on:
/// - XML: P99999 (rich: consumes ENST..01-03), P88888, P77777 (omitted)
/// - include list: P00000 (not in the XML)
/// - id mapping: ENST..30 -> P77777 (omitted), ENST..40 -> P66666 (nowhere)
/// - variants: ENST..02 (on P99999-2), ENST..09 (unmapped), ENST..30,
///   ENST..40, ENST..50 (unmapped, not in the FASTA)
/// - FASTA: ENST..09, ENST..30, ENST..40
///
/// Returns the output's ID lines.
async fn run_offline() -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let path = |name: &str, content: &str| write_file(&dir, name, content);

    let xml = uniprot_xml(&[
        rich_entry_xml("P99999"),
        minimal_entry_xml("P88888", ""),
        minimal_entry_xml("P77777", ""),
    ]);
    let opts = RunOptions {
        accessions_path: path("acc.txt", "P00000\n").display().to_string(),
        variants_path: Some(
            path(
                "variants.txt",
                "ENST00000000002.1\t3E>3K\n\
                 ENST00000000009.1\t2C>2Y\n\
                 ENST00000000030.1\t2A>2C\n\
                 ENST00000000040.1\t1M>1V\n\
                 ENST00000000050.1\t1M>1V\n",
            )
            .display()
            .to_string(),
        ),
        idmapping_path: path(
            "idmapping.tsv",
            "P99999-2\tENST00000000002.1\nP77777\tENST00000000030.1\nP66666\tENST00000000040.1\n",
        )
        .display()
        .to_string(),
        omit_accessions: Some(path("omit.txt", "P77777\n")),
        output_path: Some(dir.path().join("out.txt").display().to_string()),
        uniprot_xml: Some(path("uniprot.xml", &xml)),
        uniprot_api_fallback: false,
        ensembl_fasta: Some(path(
            "pep.fa",
            ">ENST00000000009.1\nMACDEFGH\n>ENST00000000030.1\nMAAAA\n>ENST00000000040.1\nMWWWW\n",
        )),
        ensembl_api_fallback: false,
        retry_config: RetryConfig::default(),
        uniprot_variants_enabled: true,
    };
    let output_path = opts.output_path.clone().unwrap();

    run(opts).await.unwrap();

    std::fs::read_to_string(output_path)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("ID   "))
        .map(|l| l.split_whitespace().next().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn builds_xml_entries_then_ensembl_entries_and_honours_the_omit_list() {
    assert_eq!(
        run_offline().await,
        [
            // every XML entry except the omitted P77777, in file order
            "P99999",
            "P88888",
            // unconsumed sample transcripts with a FASTA sequence, sorted;
            // ENST..30 maps to the omitted P77777, ENST..50 has no sequence
            "ENST00000000009",
            "ENST00000000040",
        ]
    );
}
