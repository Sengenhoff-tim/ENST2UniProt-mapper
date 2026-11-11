use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, anyhow};
use futures::stream::{self, StreamExt};
use tracing::{info, warn};

use crate::shared_types::UniProtCanonId;
use crate::util::{RateLimiter, RetryConfig, with_retries};

use super::{
    batch::{batch_accessions, build_uniprot_stream_url},
    entries::UniProtEntries,
    model::UniProtEntry,
    xml::parse_uniprot_xml,
};

/// Caps the number of UniProt batch requests in flight at once. Admission
/// is already paced by `RateLimiter`, but that only limits how fast new
/// requests are *started* — without this cap, every admitted request stays
/// open concurrently until it completes, which under load can pile up
/// enough simultaneous connections to make individual requests time out.
const MAX_CONCURRENT_UNIPROT_REQUESTS: usize = 8;

const UNIPROT_REQUESTS_PER_SECOND: u32 = 50;

async fn fetch_uniprot_batch(
    accessions: &[String],
    retry_config: &RetryConfig,
    client: &reqwest::Client,
) -> Result<Vec<(UniProtCanonId, UniProtEntry)>> {
    with_retries(
        &format!("fetching {} UniProt entries", accessions.len()),
        retry_config,
        || async {
            let url = build_uniprot_stream_url(accessions);
            let response = client
                .get(&url)
                .timeout(retry_config.timeout)
                .send()
                .await
                .with_context(|| {
                    format!("UniProt request failed for {} accessions", accessions.len())
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!("UniProt request failed: HTTP {}: {}", status, body));
            }

            let body = response
                .bytes()
                .await
                .context("failed to read UniProt response body")?;

            parse_response_body(&body)
        },
    )
    .await
}

/// Reads the entries of a UniProt REST response body (uncompressed XML).
fn parse_response_body(body: &[u8]) -> Result<Vec<(UniProtCanonId, UniProtEntry)>> {
    parse_uniprot_xml(body, &HashSet::new(), "UniProt REST response").map(UniProtEntries::into_vec)
}

/// Fetches UniProt entries for `accessions` from the REST API (XML format),
/// concurrently, while enforcing a single global `UNIPROT_REQUESTS_PER_SECOND`
/// cap shared by every in-flight batch.
///
/// Accessions that UniProt does not return (inactive, merged, or only known
/// as a secondary accession) are absent from the result. So are those of a
/// batch that still fails after retries: the failure is logged and the run
/// continues; the caller reports every missing accession.
pub(crate) async fn fetch_uniprot_entries(
    accessions: &[UniProtCanonId],
    retry_config: &RetryConfig,
) -> HashMap<UniProtCanonId, UniProtEntry> {
    let id_strings: Vec<String> = accessions
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();

    let batches = batch_accessions(&id_strings);
    let total = batches.len();

    // One RateLimiter, cloned into every task below, so the per-second cap
    // is enforced globally across all concurrent requests to this API.
    let rate_limiter = RateLimiter::per_second(UNIPROT_REQUESTS_PER_SECOND);
    let client = reqwest::Client::new();

    let mut entries = HashMap::new();
    let mut done = 0usize;

    let mut results = stream::iter(batches)
        .map(|chunk| {
            let rate_limiter = rate_limiter.clone();
            let retry_config = *retry_config;
            let client = client.clone();
            async move {
                rate_limiter.throttle().await;
                let result = fetch_uniprot_batch(&chunk, &retry_config, &client).await;
                (chunk.len(), result)
            }
        })
        .buffer_unordered(MAX_CONCURRENT_UNIPROT_REQUESTS);

    while let Some((batch_size, result)) = results.next().await {
        done += 1;
        match result {
            Ok(batch) => {
                entries.extend(batch);
                info!("Fetched UniProt batch {}/{}", done, total);
            }
            Err(e) => warn!(
                "UniProt batch {}/{} failed, its {} accessions stay missing: {:#}",
                done, total, batch_size, e
            ),
        }
    }

    entries
}

#[cfg(test)]
#[path = "tests/test_fetch.rs"]
mod tests;
