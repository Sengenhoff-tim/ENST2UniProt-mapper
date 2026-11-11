use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Write};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};

// use crate::ebi::{fetch_canon_variations, fetch_iso_variations};
use crate::format::format_entry;
use crate::isoform::{EntryIsoforms, collect_and_reconstruct_isoforms};
use crate::shared_types::{EnsemblId, Sequence, UniProtCanonId, UniProtId, UniProtIsoId};
use crate::uniprot::UniProtEntry;
//use crate::util::{RateLimiter, RetryConfig};
use crate::variant::{JoinVariants, Variant};

use super::{
    log::{Stage, log_pipeline_rejection, warn_pipeline_rejection},
    variants::{retain_valid, try_add_variant},
};

/// Caps how many UniProt entries are processed concurrently. This only pays
/// off once `process_entry` awaits something (e.g. EBI requests); while it
/// doesn't, entries effectively run one after another.
const MAX_CONCURRENT_ENTRIES: usize = 16;

/// Everything `process_entry` needs besides the entry itself. Cheap to clone
/// (shared state sits behind `Arc`s), so each concurrent task gets its own.
#[derive(Clone)]
pub(super) struct EntryContext {
    /// ENST-derived sample variants, by transcript.
    pub(super) sample_variants: Arc<HashMap<EnsemblId, Vec<Variant>>>,
    /*
    pub(super) source_types: Arc<Vec<String>>,
    pub(super) retry_config: RetryConfig,
    pub(super) ebi_rate_limiter: RateLimiter,
    pub(super) ebi_client: reqwest::Client,
    pub(super) ebi_variants_enabled: bool,
    */
    pub(super) uniprot_variants_enabled: bool,
}

type EntryResult = Result<(String, HashSet<EnsemblId>)>;

/// Processes all UniProt entries (up to `MAX_CONCURRENT_ENTRIES` at once)
/// and writes one output entry each, in input order. Returns the
/// sample-variant ENSTs they consumed. An entry that fails is logged and
/// skipped; only write errors abort.
///
/// Entries complete in any order (`buffer_unordered`, so a slow entry never
/// blocks new ones from starting). Entries that finish ahead of an earlier
/// one wait in `pending` until it is written; at most about
/// `MAX_CONCURRENT_ENTRIES` wait at a time.
pub(super) async fn process_entries(
    out: &mut dyn Write,
    entries: Vec<(UniProtCanonId, UniProtEntry)>,
    ctx: EntryContext,
) -> Result<HashSet<EnsemblId>> {
    let total = entries.len();
    let mut consumed_ensts = HashSet::new();

    let mut results = stream::iter(entries.into_iter().enumerate())
        .map(|(index, (canonical_id, entry))| {
            let ctx = ctx.clone();
            async move {
                let result = process_entry(&canonical_id, entry, ctx).await;
                (index, canonical_id, result)
            }
        })
        .buffer_unordered(MAX_CONCURRENT_ENTRIES);

    // Finished entries by input position, waiting for their turn.
    let mut pending: BTreeMap<usize, (UniProtCanonId, EntryResult)> = BTreeMap::new();
    let mut next = 0usize;

    while let Some((index, canonical_id, result)) = results.next().await {
        pending.insert(index, (canonical_id, result));

        while let Some((canonical_id, result)) = pending.remove(&next) {
            write_entry_result(out, &mut consumed_ensts, &canonical_id, result)?;
            next += 1;
            print!(
                "\rProcessed entry {}/{}: {}",
                next,
                total,
                canonical_id.as_str()
            );
            io::stdout().flush().ok();
        }
    }
    if total > 0 {
        println!();
    }

    Ok(consumed_ensts)
}

/// Writes a successful entry (recording the ENSTs it consumed) or logs why
/// it was skipped.
fn write_entry_result(
    out: &mut dyn Write,
    consumed_ensts: &mut HashSet<EnsemblId>,
    canonical_id: &UniProtCanonId,
    result: EntryResult,
) -> Result<()> {
    match result {
        Ok((line, ensts)) => {
            writeln!(out, "{}", line).with_context(|| {
                format!("failed to write output entry for {}", canonical_id.as_str())
            })?;
            consumed_ensts.extend(ensts);
        }
        Err(e) => warn_pipeline_rejection(
            Stage::EntryProcessing,
            canonical_id.as_str(),
            &format!("skipped entry entirely due to error: {:?}", e),
        ),
    }
    Ok(())
}

/// Processes a single UniProt entry: reconstructs isoforms, joins
/// ENST-derived sample variants with UniProt-annotated variants, and returns
/// the formatted output entry plus the ENSTs it consumed.
async fn process_entry(
    canonical_id: &UniProtCanonId,
    entry: UniProtEntry,
    ctx: EntryContext,
) -> EntryResult {
    let EntryIsoforms {
        displayed_id: canonical_iso_alias,
        sequences: isoforms,
        natural_variants: uniprot_variants,
    } = collect_and_reconstruct_isoforms(&entry);

    let (mut combined_variants, assigned_enst) = collect_sample_variants(
        canonical_id,
        canonical_iso_alias.as_ref(),
        &entry,
        &isoforms,
        &ctx.sample_variants,
    );

    if ctx.uniprot_variants_enabled {
        let resolved = resolve_uniprot_variants(
            uniprot_variants,
            &entry,
            canonical_iso_alias.as_ref(),
            &isoforms,
        );
        combined_variants.join(resolved);
    }

    /*
    if ctx.ebi_variants_enabled {
        let ebi_variants = fetch_ebi_variants(&ctx, &canonical_id, &entry, &isoforms).await?;
        combined_variants.join(ebi_variants);
    }
    */

    let valid_variants = retain_valid(combined_variants, canonical_id.as_str());

    let line = format_entry(
        canonical_id.as_str(),
        entry.sequence(),
        &valid_variants,
        Some(&entry),
    );

    Ok((line, assigned_enst))
}

/// Resolves the entry's UniProt variants against the sequence they lie on: the
/// canonical sequence (also for the displayed isoform alias) or a
/// reconstructed isoform. Variants on an isoform without a sequence are
/// logged and dropped.
fn resolve_uniprot_variants(
    variants: Vec<Variant>,
    entry: &UniProtEntry,
    canonical_iso_alias: Option<&UniProtIsoId>,
    isoforms: &HashMap<UniProtIsoId, Sequence>,
) -> Vec<Variant> {
    let mut resolved = Vec::new();
    for var in variants {
        let seq = match &var.isoform {
            None => Some(entry.sequence()),
            Some(iso) if Some(iso) == canonical_iso_alias => Some(entry.sequence()),
            Some(iso) => isoforms.get(iso).map(Sequence::as_str),
        };
        match seq {
            Some(seq) => try_add_variant(&mut resolved, var, seq),
            None => log_pipeline_rejection(
                Stage::VariantResolution,
                &var.id,
                &format!("variant {var}: isoform sequence is not available, variant omitted"),
            ),
        }
    }
    resolved
}

/// Routes the sample variants of every ENST cross-referenced by `entry` onto
/// the canonical sequence or the referenced isoform. Returns the resolved
/// variants and the ENSTs that were consumed.
fn collect_sample_variants(
    canonical_id: &UniProtCanonId,
    canonical_iso_alias: Option<&UniProtIsoId>,
    entry: &UniProtEntry,
    isoforms: &HashMap<UniProtIsoId, Sequence>,
    global_sample_variants: &HashMap<EnsemblId, Vec<Variant>>,
) -> (Vec<Variant>, HashSet<EnsemblId>) {
    let mut entry_sample_variants: Vec<Variant> = Vec::new();
    let mut assigned_enst = HashSet::new();

    for (uniprot_id, ensembl_id) in entry.ensembl_mapping() {
        let matches = match &uniprot_id {
            UniProtId::Canonical(id) => id == canonical_id,
            UniProtId::Isoform(id) => Some(id) == canonical_iso_alias,
        };

        if matches {
            if let Some(variants) = global_sample_variants.get(&ensembl_id) {
                for var in variants {
                    try_add_variant(&mut entry_sample_variants, var.clone(), entry.sequence());
                }
                assigned_enst.insert(ensembl_id);
            }
        } else {
            let UniProtId::Isoform(iso_id) = uniprot_id else {
                continue;
            };

            if let Some(iso_variants) = global_sample_variants.get(&ensembl_id)
                && let Some(iso_seq) = isoforms.get(&iso_id).map(|s| s.as_str())
            {
                for var in iso_variants {
                    let mut var = var.clone();
                    var.isoform = Some(iso_id.clone());
                    try_add_variant(&mut entry_sample_variants, var, iso_seq);
                }
                assigned_enst.insert(ensembl_id);
            }
        }
    }

    (entry_sample_variants, assigned_enst)
}

/*
/// Fetches EBI variants for the canonical accession and every reconstructed
/// isoform, resolving each against its own sequence.
async fn fetch_ebi_variants(
    ctx: &EntryContext,
    canonical_id: &UniProtCanonId,
    entry: &UniProtEntry,
    isoforms: &HashMap<UniProtIsoId, Sequence>,
) -> Result<Vec<Variant>> {
    let source_type_refs: Vec<&str> = ctx.source_types.iter().map(String::as_str).collect();

    let mut ebi_canon_variants = fetch_canon_variations(
        std::slice::from_ref(canonical_id),
        &source_type_refs,
        &ctx.retry_config,
        &ctx.ebi_rate_limiter,
        &ctx.ebi_client,
    )
    .await?;

    let iso_keys: Vec<UniProtIsoId> = isoforms.keys().cloned().collect();
    let mut ebi_iso_variants = fetch_iso_variations(
        &iso_keys,
        &source_type_refs,
        &ctx.retry_config,
        &ctx.ebi_rate_limiter,
        &ctx.ebi_client,
    )
    .await?;

    let mut ebi_variants = Vec::new();

    for (_, feat_info) in ebi_canon_variants.drain() {
        for ft in feat_info.features {
            if let Some(canon_variant) = ft.to_canon_variant(entry.primary_accession()) {
                try_add_variant(&mut ebi_variants, canon_variant, entry.sequence());
            }
        }
    }

    for (iso_id, feat_info) in ebi_iso_variants.drain() {
        for ft in feat_info.features {
            if let Some(iso_seq) = isoforms.get(&iso_id) {
                if let Some(iso_variant) = ft.to_iso_variant(&iso_id) {
                    try_add_variant(&mut ebi_variants, iso_variant, iso_seq.as_str());
                }
            }
        }
    }

    Ok(ebi_variants)
}
*/
#[cfg(test)]
#[path = "tests/test_entry.rs"]
mod tests;
