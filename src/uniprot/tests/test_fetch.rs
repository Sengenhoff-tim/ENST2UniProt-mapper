use super::*;

/// A real, uncompressed response of the stream endpoint
/// (`format=xml`) for one batch of 39 accessions.
const RESPONSE: &[u8] = include_bytes!("../../../testdata/uniprot_rest_response_39.xml");

const ACCESSIONS: [&str; 39] = [
    "A0A1B0GXE3",
    "F5GWT4",
    "E7EX48",
    "F6S8N6",
    "A6PVN5",
    "A0A669KAZ0",
    "C9J1J1",
    "A0A994J7X6",
    "A0A8I5KSW0",
    "F8W7Q0",
    "A0A804HI81",
    "F8VXG7",
    "A0A994J3P6",
    "G3V2A4",
    "E9PMV1",
    "C9JWE6",
    "H0YL86",
    "K7EPH4",
    "E7EV43",
    "H3BRT1",
    "A0A804HJ39",
    "A0A804HIW2",
    "A0A994J6E2",
    "A0A8I5KR26",
    "A0A087WXW0",
    "J3QR00",
    "P01358",
    "P01858",
    "P02728",
    "P02729",
    "P0DOX2",
    "P0DOX3",
    "P0DOX4",
    "P0DOX5",
    "P0DOX6",
    "P0DOX7",
    "P0DOX8",
    "P22103",
    "P69208",
];

#[test]
fn uncompressed_response_yields_every_requested_entry() {
    let entries = parse_response_body(RESPONSE).unwrap();
    let mut found: Vec<&str> = entries.iter().map(|(id, _)| id.as_str()).collect();
    let mut expected = ACCESSIONS.to_vec();
    found.sort();
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn uncompressed_response_entries_are_complete() {
    let entries: HashMap<_, _> = parse_response_body(RESPONSE).unwrap().into_iter().collect();

    let titin = &entries["A0A1B0GXE3"];
    assert_eq!(titin.protein_name(), "Titin");
    assert_eq!(titin.sequence().len(), 35939);
    assert!(titin.sequence().starts_with("MTTQAPTFTQ"));

    let peptide = &entries["P01358"];
    assert_eq!(peptide.protein_name(), "Gastric juice peptide 1");
    assert_eq!(peptide.sequence(), "LAAGKVEDSD");
}

#[test]
fn cut_response_is_an_error() {
    let cut = &RESPONSE[..RESPONSE.len() / 2];
    assert!(parse_response_body(cut).is_err());
}
