use super::*;

#[test]
fn ensembl_id_from_versioned_strips_the_version() {
    assert_eq!(
        EnsemblId::from_versioned("ENST00000269305.9").as_str(),
        "ENST00000269305"
    );
    assert_eq!(
        EnsemblId::from_versioned("ENST00000269305").as_str(),
        "ENST00000269305"
    );
}

#[test]
fn ensembl_id_recognizes_transcripts() {
    assert!(EnsemblId::new("ENST00000269305").is_transcript());
    assert!(!EnsemblId::new("ENSP00000269305").is_transcript());
    assert!(!EnsemblId::new("NM_000546").is_transcript());
}
