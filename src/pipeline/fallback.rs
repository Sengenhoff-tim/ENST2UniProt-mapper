use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::ensembl::{fetch_ensembl_sequences, load_ensembl_fasta};
use crate::format::format_entry;
use crate::shared_types::EnsemblId;
use crate::util::RetryConfig;
use crate::variant::Variant;

use super::{
    log::{Stage, log_pipeline_event, log_pipeline_rejection},
    variants::{retain_valid, try_add_variant},
};

/// ENSTs with no UniProt cross-reference mapping. Each one whose protein
/// sequence is found (Ensembl FASTA, then the Ensembl API if enabled) and that
/// keeps at least one valid variant on it gets a synthetic entry; the rest are
/// logged.
pub(super) async fn handle_unmapped_transcripts(
    out: &mut dyn Write,
    unmapped_enst_ids: Vec<EnsemblId>,
    global_sample_variants: &HashMap<EnsemblId, Vec<Variant>>,
    ensembl_fasta: Option<&Path>,
    ensembl_api_fallback: bool,
    retry_config: &RetryConfig,
) -> Result<()> {
    if unmapped_enst_ids.is_empty() {
        return Ok(());
    }

    // Reading the FASTA is synchronous and blocks this runtime thread while
    // it runs. That is fine: entry processing has finished, nothing else runs.
    let mut sequences = match ensembl_fasta {
        Some(path) => {
            let wanted: HashSet<EnsemblId> = unmapped_enst_ids.iter().cloned().collect();
            load_ensembl_fasta(path, &wanted)?
        }
        None => HashMap::new(),
    };

    let missing: Vec<EnsemblId> = unmapped_enst_ids
        .iter()
        .filter(|id| !sequences.contains_key(*id))
        .cloned()
        .collect();
    if ensembl_api_fallback && !missing.is_empty() {
        info!(
            "{} transcripts not in the Ensembl FASTA, querying the Ensembl API",
            missing.len()
        );
        sequences.extend(fetch_ensembl_sequences(&missing, retry_config).await);
    }

    // Split into resolved (input order) and unresolved transcripts.
    let mut enst_sequences = Vec::new();
    let mut unresolved = Vec::new();
    for id in unmapped_enst_ids {
        match sequences.remove(&id) {
            Some(seq) => enst_sequences.push((id, seq)),
            None => unresolved.push(id),
        }
    }

    for (enst_id, seq) in &enst_sequences {
        let variants = global_sample_variants
            .get(enst_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut resolved_variants = Vec::new();
        for variant in variants {
            try_add_variant(&mut resolved_variants, variant.clone(), seq.as_str());
        }
        let valid_variants = retain_valid(resolved_variants, enst_id.as_str());

        // An Ensembl entry exists only to carry sample variants.
        if valid_variants.is_empty() {
            log_pipeline_event(
                Stage::TranscriptMapping,
                enst_id.as_str(),
                "no entry written: none of its variants fit the Ensembl sequence",
            );
            continue;
        }

        write_entry(out, enst_id, seq.as_str(), &valid_variants)?;
    }

    for enst in unresolved {
        log_pipeline_event(
            Stage::TranscriptMapping,
            enst.as_str(),
            &format!(
                "no sequence found in the Ensembl FASTA{}",
                if ensembl_api_fallback {
                    " or API"
                } else {
                    " (API fallback disabled)"
                }
            ),
        );
        let variants = global_sample_variants
            .get(&enst)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for var in variants {
            log_pipeline_rejection(
                Stage::TranscriptMapping,
                enst.as_str(),
                &format!(
                    "variant {} could not be mapped: no UniProt entry and no Ensembl sequence",
                    var.id
                ),
            );
        }
    }

    Ok(())
}

/// Write one synthetic flat-file entry, named after its transcript, to `out`.
fn write_entry(
    out: &mut dyn Write,
    enst_id: &EnsemblId,
    sequence: &str,
    variants: &[Variant],
) -> Result<()> {
    writeln!(
        out,
        "{}",
        format_entry(enst_id.as_str(), sequence, variants, None)
    )
    .with_context(|| format!("failed to write output entry for {}", enst_id))
}

#[cfg(test)]
#[path = "tests/test_fallback.rs"]
mod tests;
