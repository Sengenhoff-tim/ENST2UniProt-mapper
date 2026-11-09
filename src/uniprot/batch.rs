// Backstop on the number of accessions per batch. UniProt's query parser
// enforces a hard maximum of 100 `OR` conditions per query (confirmed via
// their error message: "Too many OR conditions in query. Maximum allowed
// is 100."), so this must never exceed 100.
const MAX_UNIPROT_BATCH_SIZE: usize = 100;

// Conservative cap on the *length* of the generated stream URL, kept as
// a secondary safeguard against pathologically long accession lists
// (many proxies/load balancers reject URLs above ~8KB).
const MAX_UNIPROT_URL_LEN: usize = 3000;

const UNIPROT_STREAM_BASE_URL: &str = "https://rest.uniprot.org/uniprotkb/stream?query=";
const UNIPROT_STREAM_SUFFIX: &str = "&format=xml";

pub(super) fn build_uniprot_stream_url(accessions: &[String]) -> String {
    let query = accessions
        .iter()
        .map(|a| format!("accession:{}", a))
        .collect::<Vec<_>>()
        .join("+OR+");
    format!(
        "{}{}{}",
        UNIPROT_STREAM_BASE_URL, query, UNIPROT_STREAM_SUFFIX
    )
}

/// Splits `accessions` into batches that respect both `MAX_UNIPROT_BATCH_SIZE`
/// and `MAX_UNIPROT_URL_LEN`, so batches stay safely within UniProt's query
/// complexity limits and produce reasonably sized URLs.
pub(super) fn batch_accessions(accessions: &[String]) -> Vec<Vec<String>> {
    let fixed_len = UNIPROT_STREAM_BASE_URL.len() + UNIPROT_STREAM_SUFFIX.len();
    let mut batches: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_len = fixed_len;

    for accession in accessions {
        let term_len = "accession:".len() + accession.len();
        let added_len = term_len + if current.is_empty() { 0 } else { "+OR+".len() };

        let would_exceed_len = current_len + added_len > MAX_UNIPROT_URL_LEN;
        let would_exceed_count = current.len() >= MAX_UNIPROT_BATCH_SIZE;

        if !current.is_empty() && (would_exceed_len || would_exceed_count) {
            batches.push(std::mem::take(&mut current));
            current_len = fixed_len;
        }

        current_len += term_len + if current.is_empty() { 0 } else { "+OR+".len() };
        current.push(accession.clone());
    }

    if !current.is_empty() {
        batches.push(current);
    }

    batches
}

#[cfg(test)]
mod tests {
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
}
