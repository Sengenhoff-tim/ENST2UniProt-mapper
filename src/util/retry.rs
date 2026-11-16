use std::future::Future;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use reqwest::StatusCode;
use tracing::warn;

/// Default per-request timeout (connect + write + read) applied to every
/// outgoing REST call, overridable via `--api-timeout-secs`.
/// Generous on purpose, as an invalid fetch is irrecoverable.
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default number of attempts (including the first) before a fetch is
/// treated as fatal, overridable via `--api-max-attempts`.
pub(crate) const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Default base backoff (doubled after each failed attempt), overridable
/// via `--api-retry-backoff-ms`.
pub(crate) const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;

/// Upper bound for a single backoff, however many attempts are configured.
const MAX_BACKOFF_MS: u64 = 60_000;

/// Marks an error that retrying cannot fix (e.g. HTTP 400 or 404);
/// `with_retries` returns it right away.
#[derive(Debug)]
pub(crate) struct PermanentError(String);

impl std::fmt::Display for PermanentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PermanentError {}

/// The error for an unsuccessful HTTP response: permanent for client errors
/// (4xx) except 429 Too Many Requests, retryable otherwise (429, 5xx).
pub(crate) fn http_status_error(status: StatusCode, message: String) -> anyhow::Error {
    if status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS {
        anyhow::Error::new(PermanentError(message))
    } else {
        anyhow!(message)
    }
}

/// Backoff before the retry that follows failed attempt `attempt` (1-based):
/// `base_ms` doubled per earlier failure, capped at `MAX_BACKOFF_MS`.
fn backoff_ms(base_ms: u64, attempt: u32) -> u64 {
    2u64.checked_pow(attempt.saturating_sub(1))
        .map_or(u64::MAX, |factor| base_ms.saturating_mul(factor))
        .min(MAX_BACKOFF_MS)
}

/// Bundles the retry/timeout knobs exposed on the CLI so fetch functions
#[derive(Clone, Copy)]
pub(crate) struct RetryConfig {
    pub(crate) timeout: Duration,
    pub(crate) max_attempts: u32,
    pub(crate) backoff_base_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            backoff_base_ms: DEFAULT_RETRY_BACKOFF_MS,
        }
    }
}

/// Async retry helper. `make_request` is called fresh on every attempt
/// (it must be `FnMut` returning a new future each time, since futures
/// are single-use and can't be re-awaited). A `PermanentError` is not retried.
pub(crate) async fn with_retries<T, Fut>(
    description: &str,
    config: &RetryConfig,
    mut make_request: impl FnMut() -> Fut,
) -> Result<T>
where
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match make_request().await {
            Ok(value) => return Ok(value),
            Err(err) if err.downcast_ref::<PermanentError>().is_some() => {
                return Err(err).with_context(|| format!("{} failed (not retried)", description));
            }
            Err(err) if attempt < config.max_attempts => {
                let backoff_ms = backoff_ms(config.backoff_base_ms, attempt);
                warn!(
                    "{} failed (attempt {}/{}), retrying in {}ms: {:#}",
                    description, attempt, config.max_attempts, backoff_ms, err
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("{} failed after {} attempts", description, attempt));
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/test_retry.rs"]
mod tests;
