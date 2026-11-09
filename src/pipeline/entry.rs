use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Write};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};

// use crate::ebi::{fetch_canon_variations, fetch_iso_variations};
use crate::format::format_entry;
use crate::isoform::{EntryIsoforms, collect_and_reconstruct_isoforms};
use crate::types::{EnsemblId, Sequence, UniProtCanonId, UniProtId, UniProtIsoId};
use crate::uniprot::UniProtEntry;
//use crate::util::{RateLimiter, RetryConfig};
use crate::variant::{JoinVariants, Variant};

use super::{
    log::{Stage, warn_pipeline_rejection},
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
            eprint!(
                "\rProcessed entry {}/{}: {}",
                next,
                total,
                canonical_id.as_str()
            );
            io::stderr().flush().ok();
        }
    }
    eprintln!();

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
        combined_variants.join(uniprot_variants);
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
mod tests {
    use super::*;
    use crate::test_support::{minimal_entry_xml, parse_entry, rich_entry, variant};

    /// Sample variants for the rich entry's transcripts: ENST..01 (displayed
    /// isoform -1), ENST..02 (isoform -2), ENST..03 (canonical) and an
    /// unrelated ENST..09.
    fn sample_variants() -> HashMap<EnsemblId, Vec<Variant>> {
        [
            ("ENST00000000001", variant("s1", 1, 1, Some("A"), Some("M"))),
            ("ENST00000000002", variant("s2", 2, 2, Some("W"), Some("Y"))),
            ("ENST00000000003", variant("s3", 4, 4, Some("E"), Some("Q"))),
            ("ENST00000000009", variant("s9", 1, 1, Some("A"), Some("C"))),
        ]
        .into_iter()
        .map(|(enst, v)| (EnsemblId::new(enst), vec![v]))
        .collect()
    }

    fn context(uniprot_variants_enabled: bool) -> EntryContext {
        EntryContext {
            sample_variants: Arc::new(sample_variants()),
            uniprot_variants_enabled,
        }
    }

    fn sorted<T: Ord + Clone>(set: &HashSet<T>) -> Vec<T> {
        let mut items: Vec<T> = set.iter().cloned().collect();
        items.sort();
        items
    }

    #[test]
    fn sample_variants_are_routed_to_canonical_or_isoform() {
        let entry = rich_entry();
        let isoforms = collect_and_reconstruct_isoforms(&entry);
        let (variants, consumed) = collect_sample_variants(
            &UniProtCanonId::new("P99999"),
            isoforms.displayed_id.as_ref(),
            &entry,
            &isoforms.sequences,
            &sample_variants(),
        );

        let placed: Vec<(&str, Option<&str>)> = variants
            .iter()
            .map(|v| (v.id.as_str(), v.isoform.as_ref().map(UniProtIsoId::as_str)))
            .collect();
        // The displayed isoform -1 is an alias for the canonical sequence.
        assert_eq!(
            placed,
            [("s1", None), ("s2", Some("P99999-2")), ("s3", None)]
        );
        assert_eq!(
            sorted(&consumed),
            ["ENST00000000001", "ENST00000000002", "ENST00000000003"].map(EnsemblId::new)
        );
    }

    #[test]
    fn transcripts_of_unreconstructed_isoforms_are_not_consumed() {
        let entry = rich_entry();
        let (variants, consumed) = collect_sample_variants(
            &UniProtCanonId::new("P99999"),
            None,
            &entry,
            &HashMap::new(),
            &sample_variants(),
        );
        // Without the -1 alias and the -2 sequence only the canonical ENST..03 is used.
        assert_eq!(variants.len(), 1);
        assert_eq!(sorted(&consumed), [EnsemblId::new("ENST00000000003")]);
    }

    #[tokio::test]
    async fn uniprot_variants_are_included_only_when_enabled() {
        let id = UniProtCanonId::new("P99999");
        let (with, _) = process_entry(&id, rich_entry(), context(true))
            .await
            .unwrap();
        let (without, _) = process_entry(&id, rich_entry(), context(false))
            .await
            .unwrap();
        assert!(with.contains("/id=\"VAR_000001\""));
        assert!(!without.contains("/id=\"VAR_000001\""));
        assert!(without.contains("/id=\"s3\""));
    }

    #[tokio::test]
    async fn identical_sample_and_uniprot_variants_are_merged() {
        let id = UniProtCanonId::new("P99999");
        let mut ctx = context(true);
        // Same change as VAR_000001 (9 K -> E), via the canonical transcript.
        ctx.sample_variants = Arc::new(
            [(
                EnsemblId::new("ENST00000000003"),
                vec![variant("s3", 9, 9, Some("K"), Some("E"))],
            )]
            .into(),
        );
        let (line, _) = process_entry(&id, rich_entry(), ctx).await.unwrap();
        assert!(line.contains("/id=\"s3_VAR_000001\""), "{line}");
    }

    #[tokio::test]
    async fn process_entries_writes_in_input_order_and_reports_consumed_transcripts() {
        let entries: Vec<(UniProtCanonId, UniProtEntry)> = ["P3", "P1", "P2"]
            .into_iter()
            .map(|a| {
                (
                    UniProtCanonId::new(a),
                    parse_entry(&minimal_entry_xml(a, "")),
                )
            })
            .chain([(UniProtCanonId::new("P99999"), rich_entry())])
            .collect();
        let mut out = Vec::new();

        let consumed = process_entries(&mut out, entries, context(true))
            .await
            .unwrap();

        let out = String::from_utf8(out).unwrap();
        let ids: Vec<&str> = out
            .lines()
            .filter_map(|l| l.strip_prefix("ID   "))
            .map(|l| &l[..6])
            .collect();
        assert_eq!(ids, ["P3    ", "P1    ", "P2    ", "P99999"]);
        assert_eq!(consumed.len(), 3);
    }

    #[test]
    fn failed_entries_are_skipped_without_output() {
        let mut out = Vec::new();
        let mut consumed = HashSet::new();
        let failed: EntryResult = Err(anyhow::anyhow!("boom"));
        write_entry_result(&mut out, &mut consumed, &UniProtCanonId::new("P1"), failed).unwrap();
        assert!(out.is_empty());
        assert!(consumed.is_empty());
    }
}
