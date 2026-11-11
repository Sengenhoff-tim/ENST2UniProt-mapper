use super::*;
use crate::test_support::write_file;

fn parse(content: &str) -> HashMap<EnsemblId, Vec<Variant>> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(&dir, "variants.txt", content);
    parse_sample_variants(&path).unwrap()
}

#[test]
fn groups_variants_by_unversioned_transcript_with_indexed_ids() {
    let variants = parse("ENST1.5\t726G>726S\n  ENST1.5    10A>10C  \nENST2.1\t1M>1V\n");
    let ids: Vec<&str> = variants["ENST1"].iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, ["ENST1_0", "ENST1_1"]);
    assert_eq!(variants["ENST1"][0].begin, 726);
    assert_eq!(variants["ENST2"].len(), 1);
}

#[test]
fn transcripts_without_parsable_variants_are_left_out() {
    let variants = parse("ENST1\tgarbage\nENST1\t726G>731S\nENST2\t1M>1V\n");
    assert!(!variants.contains_key("ENST1"));
    assert_eq!(variants.len(), 1);
}

#[test]
fn ids_count_only_parsed_variants() {
    let variants = parse("ENST1\tgarbage\nENST1\t1M>1V\nENST1\t2A>2C\n");
    let ids: Vec<&str> = variants["ENST1"].iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, ["ENST1_0", "ENST1_1"]);
}

#[test]
fn skips_invalid_lines() {
    let variants = parse(
        "NM_0001\t1A>1C\n\
         ENST1\n\
         ENST1\t726G>731S\n\
         ENST1\tgarbage\n\
         \n\
         ENST1\t5K>5E\n",
    );
    assert_eq!(variants.len(), 1);
    assert_eq!(variants["ENST1"].len(), 1);
    assert_eq!(variants["ENST1"][0].begin, 5);
}
