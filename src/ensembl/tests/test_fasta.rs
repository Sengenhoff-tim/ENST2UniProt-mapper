use super::*;
use crate::test_support::write_file;

fn load(content: &str, wanted: &[&str]) -> HashMap<EnsemblId, Sequence> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "pep.fa", content);
    let wanted: HashSet<EnsemblId> = wanted.iter().map(|id| EnsemblId::new(*id)).collect();
    load_ensembl_fasta(&path, &wanted).unwrap()
}

#[test]
fn transcript_id_from_ensembl_pep_header() {
    let header = "ENSP00000493376.2 pep chromosome:GRCh38:1:65565:71585:1 gene:ENSG00000186092.7 transcript:ENST00000641515.2 gene_biotype:protein_coding";
    assert_eq!(
        transcript_id(header),
        Some(EnsemblId::new("ENST00000641515"))
    );
}

#[test]
fn transcript_id_from_bare_header() {
    assert_eq!(
        transcript_id("ENST00000641515.2"),
        Some(EnsemblId::new("ENST00000641515"))
    );
    assert_eq!(transcript_id("sp|P12345|FOO"), None);
}

#[test]
fn joins_multi_line_records_and_keeps_only_wanted_ones() {
    let sequences = load(
        ">ENSP1.1 pep transcript:ENST1.1\nMACD\nEFGH\n>ENSP2.1 pep transcript:ENST2.1\nMKKK\n",
        &["ENST1"],
    );
    assert_eq!(sequences.len(), 1);
    assert_eq!(sequences["ENST1"].as_str(), "MACDEFGH");
}

#[test]
fn truncates_at_the_first_stop_codon() {
    let sequences = load(">ENST1.1\nMAC*DE\n", &["ENST1"]);
    assert_eq!(sequences["ENST1"].as_str(), "MAC");
}

#[test]
fn keeps_the_first_of_duplicate_records() {
    let sequences = load(">ENST1.1\nMAAA\n>ENST1.2\nMBBB\n", &["ENST1"]);
    assert_eq!(sequences["ENST1"].as_str(), "MAAA");
}

#[test]
fn ignores_headers_without_transcript_id() {
    let sequences = load(">sp|P12345|FOO\nMAAA\n>ENST1\nMCCC\n", &["ENST1"]);
    assert_eq!(sequences.len(), 1);
    assert_eq!(sequences["ENST1"].as_str(), "MCCC");
}
