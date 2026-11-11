use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{info, warn};

//use crate::ebi::EBI_MAX_REQUESTS_PER_SECOND;
use crate::parsing::{parse_accession_list, parse_idmapping, parse_sample_variants};
use crate::shared_types::{EnsemblId, UniProtCanonId};
use crate::uniprot::{
    UniProtEntries, fetch_uniprot_entries, load_uniprot_xml, log_unresolved_accessions,
};
use crate::util::RetryConfig;
use crate::variant::Variant;

use super::{
    entry::{EntryContext, process_entries},
    fallback::handle_unmapped_transcripts,
    input::{ensembl_fallback_candidates, extend_with_mapped_accessions, read_omit_list},
};

/// Everything `run` needs, as resolved from the command line.
pub(crate) struct RunOptions {
    pub(crate) ensure_accessions: Option<PathBuf>,
    pub(crate) sample_variants: Option<PathBuf>,
    pub(crate) ensembl_idmapping: Option<PathBuf>,
    pub(crate) exclude_accessions: Option<PathBuf>,
    pub(crate) output: PathBuf,
    /// Local UniProt XML (optionally gzipped); primary source of entries.
    pub(crate) uniprot_xml: Option<PathBuf>,
    /// Query the UniProt API for accessions missing from `uniprot_xml`.
    pub(crate) uniprot_api_fallback: bool,
    /// Local Ensembl peptide FASTA (optionally gzipped); primary source of
    /// sequences for transcripts without a UniProt mapping.
    pub(crate) ensembl_fasta: Option<PathBuf>,
    /// Query the Ensembl API for transcripts missing from `ensembl_fasta`.
    pub(crate) ensembl_api_fallback: bool,
    //pub(crate) source_types: Vec<String>,
    pub(crate) retry_config: RetryConfig,
    //pub(crate) ebi_variants_enabled: bool,
    pub(crate) uniprot_variants_enabled: bool,
}

pub(crate) async fn run(opts: RunOptions) -> Result<()> {
    let retry_config = &opts.retry_config;
    warn_about_missing_inputs(&opts);
    let mut out: Box<dyn Write> =
        Box::new(File::create(&opts.output).with_context(|| {
            format!("failed to create output file '{}'", opts.output.display())
        })?);

    // 1. Parse inputs; a missing input counts as empty
    let mut accessions: Vec<UniProtCanonId> = match &opts.ensure_accessions {
        Some(path) => parse_accession_list(path)?,
        None => Vec::new(),
    };
    let global_sample_variants: HashMap<EnsemblId, Vec<Variant>> = match &opts.sample_variants {
        Some(path) => parse_sample_variants(path)?,
        None => HashMap::new(),
    };

    // 2. Parse id mapping (EnsemblId -> UniProtCanonId) and extend accessions
    let id_mapping: HashMap<EnsemblId, Vec<UniProtCanonId>> = match &opts.ensembl_idmapping {
        Some(path) => parse_idmapping(path)?,
        None => HashMap::new(),
    };
    extend_with_mapped_accessions(&mut accessions, global_sample_variants.keys(), &id_mapping);

    // The exclude list applies to both the ensured accessions and the XML file.
    let omit = read_omit_list(opts.exclude_accessions.as_deref())?;
    if !omit.is_empty() {
        accessions.retain(|id| !omit.contains(id));
    }

    // 3. Load every UniProt entry of the XML file; accessions from the list
    //    (and id mapping) that the file lacks are fetched from the API if
    //    enabled, and logged otherwise.
    //    Reading the XML is synchronous and blocks this runtime thread while
    //    it runs. That is fine: no other async work has been started yet.
    let mut uniprot_entries = match &opts.uniprot_xml {
        Some(path) => load_uniprot_xml(path, &omit)?,
        None => UniProtEntries::default(),
    };
    let missing = uniprot_entries.missing(&accessions);
    let unresolved = if opts.uniprot_api_fallback && !missing.is_empty() {
        info!(
            "{} required accessions not in the UniProt XML, querying the UniProt API",
            missing.len()
        );
        let fetched = fetch_uniprot_entries(&missing, retry_config).await;
        uniprot_entries.add_fetched(missing, fetched)
    } else {
        missing
    };
    log_unresolved_accessions(&unresolved, opts.uniprot_api_fallback);

    // 4. One output entry per UniProt entry; each consumes the sample-variant
    //    ENSTs it cross-references.
    let sample_variants = Arc::new(global_sample_variants);
    let ctx = EntryContext {
        sample_variants: Arc::clone(&sample_variants),
        /*
        source_types: Arc::new(opts.source_types.clone()),
        retry_config: *retry_config,
        ebi_rate_limiter: RateLimiter::per_second(EBI_MAX_REQUESTS_PER_SECOND),
        ebi_client: reqwest::Client::new(),
        ebi_variants_enabled: opts.ebi_variants_enabled,
        */
        uniprot_variants_enabled: opts.uniprot_variants_enabled,
    };

    // async not strictly required: kept to potentially re-enable ebi variant fetching
    let consumed_ensts = process_entries(&mut *out, uniprot_entries.into_vec(), ctx).await?;

    // 5. Sample-variant ENSTs not consumed by any UniProt entry → synthetic
    //    entries from the Ensembl FASTA, then the API if enabled. Nothing the
    //    id mapping ties to an omitted accession is included.
    let unmapped_enst_ids =
        ensembl_fallback_candidates(sample_variants.keys(), &id_mapping, &consumed_ensts, &omit);

    handle_unmapped_transcripts(
        &mut *out,
        unmapped_enst_ids,
        &sample_variants,
        opts.ensembl_fasta.as_deref(),
        opts.ensembl_api_fallback,
        retry_config,
    )
    .await
}

/// Warns once per missing input, so a run that writes little or nothing
/// explains why. Missing inputs are not errors: each simply counts as empty.
fn warn_about_missing_inputs(opts: &RunOptions) {
    if opts.ensure_accessions.is_none() {
        warn!("no --ensure-accessions given: only entries of the UniProt XML are written");
    }
    if opts.ensembl_idmapping.is_none() {
        warn!("no --ensembl-idmapping given: no accessions are derived from sample-variant ENSTs");
    }
    match (&opts.uniprot_xml, opts.uniprot_api_fallback) {
        (None, false) => warn!(
            "no --uniprot-xml or --uniprot-api-fallback given: no UniProt entries are written"
        ),
        (None, true) => {
            warn!("no --uniprot-xml given: all UniProt entries are fetched from the UniProt API")
        }
        (Some(_), _) => {}
    }
    if opts.sample_variants.is_none() {
        warn!("no --sample-variants given: only UniProt variants are written");
    }
}

#[cfg(test)]
#[path = "tests/test_run.rs"]
mod tests;
