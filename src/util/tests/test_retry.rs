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

#[tokio::test]
async fn permanent_errors_are_not_retried() {
    let calls = Cell::new(0);
    let result: Result<()> = with_retries("op", &config(5), || {
        calls.set(calls.get() + 1);
        async {
            Err(http_status_error(
                StatusCode::NOT_FOUND,
                "HTTP 404".to_string(),
            ))
        }
    })
    .await;
    assert_eq!(calls.get(), 1);
    assert!(format!("{:#}", result.unwrap_err()).contains("not retried"));
}

#[test]
fn rate_limits_and_server_errors_stay_retryable() {
    for status in [StatusCode::TOO_MANY_REQUESTS, StatusCode::BAD_GATEWAY] {
        let err = http_status_error(status, String::new());
        assert!(err.downcast_ref::<PermanentError>().is_none(), "{status}");
    }
}

#[test]
fn backoff_doubles_and_is_capped_without_overflow() {
    assert_eq!(backoff_ms(500, 1), 500);
    assert_eq!(backoff_ms(500, 3), 2000);
    assert_eq!(backoff_ms(500, 30), MAX_BACKOFF_MS);
    assert_eq!(backoff_ms(500, 100), MAX_BACKOFF_MS);
    assert_eq!(backoff_ms(u64::MAX, 2), MAX_BACKOFF_MS);
}
