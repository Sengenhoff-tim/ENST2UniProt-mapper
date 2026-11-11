//! UniProt XML fixtures and helpers to parse them.

use std::collections::HashSet;

use crate::shared_types::UniProtCanonId;
use crate::uniprot::{UniProtEntry, parse_uniprot_xml};

/// 20-residue reference sequence in which position `n` (1-based) holds the
/// `n`-th letter, which keeps expected substrings easy to read:
/// 1=A 2=C 3=D 4=E 5=F 6=G 7=H 8=I 9=K 10=L 11=M ... 18=V 19=W 20=Y.
pub(crate) const SEQ: &str = "ACDEFGHIKLMNPQRSTVWY";

/// Wraps `<entry>` elements in a UniProt XML document.
pub(crate) fn uniprot_xml(entries: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <uniprot xmlns=\"http://uniprot.org/uniprot\">\n{}\n</uniprot>\n",
        entries.join("\n")
    )
}

/// A minimal `<entry>` with `accession`, sequence `SEQ`, and `body` inserted
/// before the sequence.
pub(crate) fn minimal_entry_xml(accession: &str, body: &str) -> String {
    format!(
        r#"<entry dataset="Swiss-Prot" created="2000-01-01" modified="2024-01-01" version="1">
  <accession>{accession}</accession>
  {body}
  <sequence length="20" mass="2000" checksum="0123456789ABCDEF" modified="2000-01-01" version="1">{SEQ}</sequence>
</entry>"#
    )
}

/// A realistic `<entry>` exercising every element the reader keeps:
/// - names: protein "Test protein", gene "TST" (synonym "TS")
/// - isoforms: `-1` displayed, `-2` described by VSP_000001 + VSP_000002
///   (expected sequence `AWEFGHIKLMNPQRST`), `-3` not described
/// - VSP_000001: 2..3 `CD` -> `W` with evidence key 1 (PubMed:12345)
/// - VSP_000002: 18..20 missing
/// - VAR_000001: 9 `K` -> `E` (canonical)
/// - VAR_000002: 5 `G` -> `A` on isoform `-2`
/// - Ensembl: ENST..01 -> `-1`, ENST..02 -> `-2`, ENST..03 -> canonical
/// - plus elements that must be ignored (a `site` feature, another comment,
///   a non-Ensembl dbReference)
pub(crate) fn rich_entry_xml(accession: &str) -> String {
    format!(
        r#"<entry dataset="Swiss-Prot" created="2000-01-01" modified="2024-01-01" version="10">
  <accession>{accession}</accession>
  <accession>Q00000</accession>
  <name>TEST_HUMAN</name>
  <protein>
    <recommendedName><fullName>Test protein</fullName></recommendedName>
    <alternativeName><fullName>Other name</fullName></alternativeName>
  </protein>
  <gene><name type="primary">TST</name><name type="synonym">TS</name></gene>
  <organism><name type="scientific">Homo sapiens</name><dbReference type="NCBI Taxonomy" id="9606"/></organism>
  <comment type="function"><text>Does things.</text></comment>
  <comment type="alternative products">
    <event type="alternative splicing"/>
    <isoform><id>{accession}-1</id><name>1</name><sequence type="displayed"/></isoform>
    <isoform><id>{accession}-2</id><name>2</name><name>Beta</name><sequence type="described" ref="VSP_000001 VSP_000002"/></isoform>
    <isoform><id>{accession}-3</id><name>3</name><sequence type="not described"/></isoform>
    <text>Additional isoforms seem to exist.</text>
  </comment>
  <dbReference type="Ensembl" id="ENST00000000001.3"><molecule id="{accession}-1"/><property type="protein sequence ID" value="ENSP00000000001.1"/></dbReference>
  <dbReference type="Ensembl" id="ENST00000000002.1"><molecule id="{accession}-2"/></dbReference>
  <dbReference type="Ensembl" id="ENST00000000003.1"/>
  <dbReference type="PDB" id="1ABC"/>
  <feature type="splice variant" id="VSP_000001" description="in isoform 2" evidence="1"><original>CD</original><variation>W</variation><location><begin position="2"/><end position="3"/></location></feature>
  <feature type="splice variant" id="VSP_000002" description="in isoform 2"><location><begin position="18"/><end position="20"/></location></feature>
  <feature type="sequence variant" id="VAR_000001" description="in dbSNP:rs1"><original>K</original><variation>E</variation><location><position position="9"/></location></feature>
  <feature type="sequence variant" id="VAR_000002"><original>G</original><variation>A</variation><location sequence="{accession}-2"><position position="5"/></location></feature>
  <feature type="site" description="Cleavage"><location><position position="4"/></location></feature>
  <evidence type="ECO:0000303" key="1"><source><dbReference type="PubMed" id="12345"/></source></evidence>
  <sequence length="20" mass="2000" checksum="0123456789ABCDEF" modified="2000-01-01" version="1">{SEQ}</sequence>
</entry>"#
    )
}

/// Parses XML `<entry>` elements (nothing omitted).
pub(crate) fn parse_entries(entries: &[String]) -> Vec<(UniProtCanonId, UniProtEntry)> {
    parse_uniprot_xml(uniprot_xml(entries).as_bytes(), &HashSet::new(), "test")
        .expect("test XML must parse")
        .into_vec()
}

/// Parses a single XML `<entry>` element.
pub(crate) fn parse_entry(entry: &str) -> UniProtEntry {
    let mut entries = parse_entries(&[entry.to_string()]);
    assert_eq!(entries.len(), 1, "expected exactly one entry");
    entries.remove(0).1
}

/// `rich_entry_xml("P99999")`, parsed.
pub(crate) fn rich_entry() -> UniProtEntry {
    parse_entry(&rich_entry_xml("P99999"))
}
