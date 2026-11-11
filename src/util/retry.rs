use std::future::Future;
use std::time::Duration;

use anyhow::{Context, Result};

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
/// are single-use and can't be re-awaited).
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
            Err(err) if attempt < config.max_attempts => {
                let backoff_ms = config.backoff_base_ms * 2u64.pow(attempt - 1);
                println!(
                    "Warning: {} failed (attempt {}/{}), retrying in {}ms: {:?}",
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
