use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::shared_types::{EnsemblId, Sequence};
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
    info!(
        "{}: kept {} of {} requested transcripts",
        path.display(),
        sequences.len(),
        wanted.len()
    );
    Ok(sequences)
}

#[cfg(test)]
#[path = "tests/test_fasta.rs"]
mod tests;
