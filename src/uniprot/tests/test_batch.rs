use super::*;

fn accessions(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("P{:05}", i)).collect()
}

#[test]
fn stream_url_ors_accession_terms_and_requests_xml() {
    let url = build_uniprot_stream_url(&["P1".to_string(), "P2".to_string()]);
    assert_eq!(
        url,
        "https://rest.uniprot.org/uniprotkb/stream?query=accession:P1+OR+accession:P2&format=xml"
    );
}

#[test]
fn batches_respect_the_or_condition_limit() {
    let batches = batch_accessions(&accessions(250));
    let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
    assert!(
        sizes.iter().all(|&n| n <= MAX_UNIPROT_BATCH_SIZE),
        "{sizes:?}"
    );
    assert_eq!(sizes.iter().sum::<usize>(), 250);
}

#[test]
fn batches_respect_the_url_length_limit() {
    let long: Vec<String> = (0..100).map(|i| format!("A{:0>60}", i)).collect();
    for batch in batch_accessions(&long) {
        assert!(build_uniprot_stream_url(&batch).len() <= MAX_UNIPROT_URL_LEN);
    }
}

#[test]
fn batches_preserve_order_and_handle_empty_input() {
    let input = accessions(150);
    let flattened: Vec<String> = batch_accessions(&input).concat();
    assert_eq!(flattened, input);
    assert!(batch_accessions(&[]).is_empty());
}
