use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::types::{EnsemblId, Sequence};
use crate::util::open_maybe_gz;

use super::api::strip_stop_codon;

/// Stage label for log events of this reader.
const STAGE: &str = "ENSEMBL FASTA";

/// Extracts the (unversioned) transcript id from a FASTA header line
/// (without the leading `>`). Supports Ensembl peptide dumps, e.g.
/// `ENSP00000493376.2 pep chromosome:... transcript:ENST00000641515.2 ...`,
/// as well as headers that start with the ENST id itself.
fn transcript_id(header: &str) -> Option<EnsemblId> {
    let id = header
        .split_whitespace()
        .find_map(|token| token.strip_prefix("transcript:"))
        .or_else(|| header.split_whitespace().next())?;
    let id = EnsemblId::from_versioned(id);
    id.is_transcript().then_some(id)
}

/// Reads protein sequences for the transcripts in `wanted` from a FASTA
/// file (optionally gzipped). Sequences are truncated at the first `*`,
/// matching the Ensembl API path.
pub(crate) fn load_ensembl_fasta(
    path: &Path,
    wanted: &HashSet<EnsemblId>,
) -> Result<HashMap<EnsemblId, Sequence>> {
    let reader = open_maybe_gz(path)?;
    let mut sequences: HashMap<EnsemblId, Sequence> = HashMap::new();
    // `None` while inside a record we are not interested in.
    let mut current: Option<(EnsemblId, String)> = None;
    let mut unrecognized = 0usize;

    let finish = |record: Option<(EnsemblId, String)>, sequences: &mut HashMap<_, _>| {
        if let Some((id, seq)) = record {
            if sequences.contains_key(&id) {
                warn!(
                    stage = STAGE,
                    rejected = true,
                    "{}: duplicate FASTA record for {}, keeping the first",
                    path.display(),
                    id.as_str()
                );
                return;
            }
            let seq = strip_stop_codon(id.as_str(), seq);
            sequences.insert(id, Sequence::new(seq));
        }
    };

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read '{}'", path.display()))?;
        let line = line.trim_end();
        if let Some(header) = line.strip_prefix('>') {
            finish(current.take(), &mut sequences);
            match transcript_id(header) {
                Some(id) if wanted.contains(&id) => current = Some((id, String::new())),
                Some(_) => {}
                None => unrecognized += 1,
            }
        } else if let Some((_, seq)) = current.as_mut() {
            seq.push_str(line.trim());
        }
    }
    finish(current.take(), &mut sequences);

    if unrecognized > 0 {
        warn!(
            stage = STAGE,
            rejected = true,
            count = unrecognized,
            "{}: {} FASTA headers had no recognizable ENST id and were ignored",
            path.display(),
            unrecognized
        );
    }
    info!("{}: kept {} of {} requested transcripts", path.display(), sequences.len(), wanted.len());
    Ok(sequences)
}

#[cfg(test)]
mod tests {
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
        assert_eq!(transcript_id(header), Some(EnsemblId::new("ENST00000641515")));
    }

    #[test]
    fn transcript_id_from_bare_header() {
        assert_eq!(transcript_id("ENST00000641515.2"), Some(EnsemblId::new("ENST00000641515")));
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
}
