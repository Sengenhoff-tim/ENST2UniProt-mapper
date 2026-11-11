use std::cell::Cell;

use anyhow::anyhow;

use super::*;

fn config(max_attempts: u32) -> RetryConfig {
    RetryConfig {
        max_attempts,
        backoff_base_ms: 0,
        ..RetryConfig::default()
    }
}

#[tokio::test]
async fn returns_first_success_without_retrying() {
    let calls = Cell::new(0);
    let result = with_retries("op", &config(3), || {
        calls.set(calls.get() + 1);
        async { Ok::<_, anyhow::Error>(42) }
    })
    .await;
    assert_eq!(result.unwrap(), 42);
    assert_eq!(calls.get(), 1);
}

#[tokio::test]
async fn retries_until_success() {
    let calls = Cell::new(0);
    let result = with_retries("op", &config(3), || {
        calls.set(calls.get() + 1);
        let attempt = calls.get();
        async move {
            if attempt < 3 {
                Err(anyhow!("flaky"))
            } else {
                Ok(attempt)
            }
        }
    })
    .await;
    assert_eq!(result.unwrap(), 3);
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let calls = Cell::new(0);
    let result: Result<()> = with_retries("fetching things", &config(2), || {
        calls.set(calls.get() + 1);
        async { Err(anyhow!("down")) }
    })
    .await;
    assert_eq!(calls.get(), 2);
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("fetching things failed after 2 attempts"),
        "{err}"
    );
    assert!(err.contains("down"), "{err}");
}
