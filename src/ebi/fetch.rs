use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use tracing::info;

use crate::types::{UniProtCanonId, UniProtIsoId};
use crate::util::{with_retries, RateLimiter, RetryConfig};

use super::{source_type::validate_source_types, types::ProteinFeatureInfo};

pub(crate) const EBI_MAX_REQUESTS_PER_SECOND: u32 = 200;

const MAX_CONCURRENT_EBI_REQUESTS: usize = 10;

/// Builds the URL for a single accession using EBI's singular path-style
/// endpoint (`/variation/{accession}`), rather than the query-param batch
/// endpoint (`/variation?accession=A,B,C`). We always fetch one accession
/// at a time now, and the batch endpoint is known to reject/400 on certain
/// single-accession requests, so the singular endpoint is used uniformly.
fn build_variation_url(accession: &str, source_types: &[&str]) -> Result<String> {
    validate_source_types(source_types)?;
    let mut url = format!("https://www.ebi.ac.uk/proteins/api/variation/{}", accession);
    if !source_types.is_empty() {
        url.push_str("?sourcetype=");
        url.push_str(&source_types.join(","));
    }
    Ok(url)
}

async fn fetch_variation_single(
    accession: &str,
    source_types: &[&str],
    retry_config: &RetryConfig,
    client: &reqwest::Client,
) -> Result<Option<ProteinFeatureInfo>> {
    with_retries(
        &format!("fetching EBI variation for {}", accession),
        retry_config,
        || async {
            let url = build_variation_url(accession, source_types)?;
            let response = client
                .get(&url)
                .timeout(retry_config.timeout)
                .send()
                .await
                .with_context(|| format!("EBI variation request to {} failed", url))?;

            /*
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // No variation data for this accession - not an error.
                return Ok(None);
            }
            */

            if !response.status().is_success() {
                let status = response.status();
                return Err(anyhow!("EBI variation request for url {} failed: HTTP {}", url, status));
            }

            let body = response
                .text()
                .await
                .context("failed to read EBI variation response body")?;
            let info: ProteinFeatureInfo = serde_json::from_str(&body)
                .context("failed to parse EBI variation response as JSON")?;
            Ok(Some(info))
        },
    )
    .await
}

async fn fetch_variations_generic<K>(
    keys: &[K],
    as_str: impl Fn(&K) -> String,
    source_types: &[&str],
    retry_config: &RetryConfig,
    rate_limiter: &RateLimiter,
    client: &reqwest::Client,
) -> Result<HashMap<K, ProteinFeatureInfo>>
where
    K: Clone + Eq + std::hash::Hash,
{
    let mut result: HashMap<K, ProteinFeatureInfo> = HashMap::new();

    let source_types_owned: Vec<String> = source_types.iter().map(|s| s.to_string()).collect();

    // List-level progress has no single feature, so it stays a plain event.
    info!(
        accession_count = keys.len(),
        "Starting per-accession EBI variation fetch"
    );

    let mut results = stream::iter(keys.iter().cloned())
        .map(|key| {
            let accession = as_str(&key);
            let rate_limiter = rate_limiter.clone();
            let retry_config = *retry_config;
            let client = client.clone();
            let source_types_owned = source_types_owned.clone();

            async move {
                rate_limiter.throttle().await;
                let source_type_refs: Vec<&str> = source_types_owned.iter().map(String::as_str).collect();

                info!(accession = %accession, source_types = ?source_type_refs, "Fetching variation");

                let info = fetch_variation_single(&accession, &source_type_refs, &retry_config, &client).await?;

                Ok::<_, anyhow::Error>((key, info))
            }
        })
        .buffer_unordered(MAX_CONCURRENT_EBI_REQUESTS);

    while let Some(res) = results.next().await {
        let (key, info) = res?;
        if let Some(info) = info {
            result.insert(key, info);
        }
    }

    info!(
        total_results = result.len(),
        "Per-accession EBI variation fetch completed"
    );

    Ok(result)
}

pub(crate) async fn fetch_iso_variations(
    uniprot_ids: &[UniProtIsoId],
    source_types: &[&str],
    retry_config: &RetryConfig,
    rate_limiter: &RateLimiter,
    client: &reqwest::Client,
) -> Result<HashMap<UniProtIsoId, ProteinFeatureInfo>> {
    fetch_variations_generic(uniprot_ids, |id| id.as_str().to_string(), source_types, retry_config, rate_limiter, client).await
}

pub(crate) async fn fetch_canon_variations(
    uniprot_ids: &[UniProtCanonId],
    source_types: &[&str],
    retry_config: &RetryConfig,
    rate_limiter: &RateLimiter,
    client: &reqwest::Client,
) -> Result<HashMap<UniProtCanonId, ProteinFeatureInfo>> {
    fetch_variations_generic(uniprot_ids, |id| id.as_str().to_string(), source_types, retry_config, rate_limiter, client).await
}
