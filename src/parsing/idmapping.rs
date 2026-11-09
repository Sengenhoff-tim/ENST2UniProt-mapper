use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};

use crate::types::{EnsemblId, UniProtCanonId, UniProtId};

use super::log::{Stage, log_line_rejection};

/// Parses a UniProt-canonical/Ensembl transcript id mapping file, returning a
/// map of Ensembl-ID → list of UniProt-Canon-IDs (an ENST can map to more
/// than one UniProt accession).
///
/// Expected format: tab-separated `<uniprot_id>[-<isoform>]\t<enst_id>[.<version>]`
/// e.g. "P62258-2\tENST00000571732.5" -> (ENST00000571732, [P62258])
pub(crate) fn parse_idmapping(path: &str) -> Result<HashMap<EnsemblId, Vec<UniProtCanonId>>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read id mapping file '{}'", path))?;
    let mut mapping: HashMap<EnsemblId, Vec<UniProtCanonId>> = HashMap::new();
    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let location = format!("{}:{}", path, line_no + 1);

        let mut parts = line.split('\t');
        let Some(uniprot_raw) = parts.next() else {
            continue;
        };
        let Some(enst_raw) = parts.next() else {
            log_line_rejection(
                Stage::IdMappingParsing,
                &location,
                &format!("skipped, missing ENST field for {}", uniprot_raw),
            );
            continue;
        };

        // e.g. "P62258-2" -> "P62258"
        let uniprot_id = UniProtId::from_accession(uniprot_raw).canonical();
        // e.g. "ENST00000571732.5" -> "ENST00000571732"
        let enst_id = EnsemblId::from_versioned(enst_raw);

        if !enst_id.is_transcript() {
            log_line_rejection(
                Stage::IdMappingParsing,
                &location,
                &format!("skipped, '{}' does not look like an ENST id", enst_raw),
            );
            continue;
        }

        let entry = mapping.entry(enst_id).or_default();
        if !entry.contains(&uniprot_id) {
            entry.push(uniprot_id);
        }
    }
    Ok(mapping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_file;

    fn parse(content: &str) -> HashMap<EnsemblId, Vec<UniProtCanonId>> {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "idmapping.tsv", content);
        parse_idmapping(path.to_str().unwrap()).unwrap()
    }

    fn accessions_of<'a>(
        mapping: &'a HashMap<EnsemblId, Vec<UniProtCanonId>>,
        enst: &str,
    ) -> Vec<&'a str> {
        mapping[enst].iter().map(UniProtCanonId::as_str).collect()
    }

    #[test]
    fn maps_unversioned_transcripts_to_canonical_accessions() {
        let mapping = parse("P62258-2\tENST00000571732.5\n");
        assert_eq!(accessions_of(&mapping, "ENST00000571732"), ["P62258"]);
    }

    #[test]
    fn collects_several_accessions_per_transcript_without_duplicates() {
        let mapping = parse("P1\tENST1.1\nP2\tENST1.1\nP1-2\tENST1.2\n");
        assert_eq!(accessions_of(&mapping, "ENST1"), ["P1", "P2"]);
    }

    #[test]
    fn skips_non_transcript_and_incomplete_lines() {
        let mapping = parse("P1\tENSP00000000001\nP2\n\nP3\tENST3\n");
        assert_eq!(mapping.len(), 1);
        assert_eq!(accessions_of(&mapping, "ENST3"), ["P3"]);
    }
}
