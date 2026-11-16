use crate::shared_types::{EnsemblId, Sequence};
use crate::util::{RetryConfig, http_status_error, with_retries};

use super::sequence::strip_stop_codon;
use anyhow::{Context, Result};
use std::collections::HashMap;

const MAX_ENSEMBL_BATCH_SIZE: usize = 50;
const ENSEMBL_SEQUENCE_URL: &str = "https://rest.ensembl.org/sequence/id?type=protein";

#[derive(serde::Serialize)]
struct EnsemblSequenceRequestBody<'a> {
    ids: &'a [String],
}

#[derive(Debug, serde::Deserialize)]
struct EnsemblSequenceResponse {
    molecule: String,
    seq: String,
    query: String,
    #[serde(rename = "id")]
    #[allow(dead_code)]
    response_id: String,
}

async fn fetch_ensembl_sequence_batch(
    ids: &[String],
    retry_config: &RetryConfig,
    client: &reqwest::Client,
) -> Result<Vec<EnsemblSequenceResponse>> {
    with_retries(
        &format!("fetching Ensembl sequences for {} ids", ids.len()),
        retry_config,
        || async {
            let response = client
                .post(ENSEMBL_SEQUENCE_URL)
                .timeout(retry_config.timeout)
                .json(&EnsemblSequenceRequestBody { ids })
                .send()
                .await
                .context("Ensembl sequence request failed")?;

            if !response.status().is_success() {
                let status = response.status();
                return Err(http_status_error(
                    status,
                    format!(
                        "{},Ensembl sequence request failed: HTTP {}",
                        ids.join("|"),
                        status
                    ),
                ));
            }

            let responses: Vec<EnsemblSequenceResponse> = response
                .json()
                .await
                .context("failed to parse Ensembl sequence response")?;
            Ok(responses)
        },
    )
    .await
}

/// Fetches protein sequences for `enst_ids` from the Ensembl REST API, in
/// batches. Transcripts Ensembl does not return are absent from the result,
/// as are those of a batch that still fails after retries: the failure is
/// logged and the run continues; the caller reports every missing transcript.
pub(crate) async fn fetch_ensembl_sequences(
    enst_ids: &[EnsemblId],
    retry_config: &RetryConfig,
) -> Vec<(EnsemblId, Sequence)> {
    let id_strings: Vec<String> = enst_ids.iter().map(|id| id.as_str().to_string()).collect();
    let mut sequences: HashMap<String, Sequence> = HashMap::new();
    let client = reqwest::Client::new();

    let total = id_strings.len().div_ceil(MAX_ENSEMBL_BATCH_SIZE);
    for (i, chunk) in id_strings.chunks(MAX_ENSEMBL_BATCH_SIZE).enumerate() {
        let responses = match fetch_ensembl_sequence_batch(chunk, retry_config, &client).await {
            Ok(responses) => responses,
            Err(e) => {
                tracing::warn!(
                    "Ensembl batch {}/{} failed, its {} transcripts stay missing: {:#}",
                    i + 1,
                    total,
                    chunk.len(),
                    e
                );
                continue;
            }
        };
        for record in responses {
            if record.molecule != "protein" {
                tracing::info!(
                    enst = record.query.as_str(),
                    "expected molecule type 'protein', got '{}'; skipping",
                    record.molecule
                );
                continue;
            }
            let seq = strip_stop_codon(&record.query, record.seq);
            sequences.insert(record.query, Sequence::new(seq));
        }
    }

    enst_ids
        .iter()
        .filter_map(|id| sequences.remove(id.as_str()).map(|seq| (id.clone(), seq)))
        .collect()
}
