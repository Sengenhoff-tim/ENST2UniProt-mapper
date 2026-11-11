use super::*;

#[tokio::test(start_paused = true)]
async fn spaces_admissions_by_the_minimum_interval() {
    let limiter = RateLimiter::per_second(10); // one every 100ms
    let start = tokio::time::Instant::now();

    limiter.throttle().await; // immediate
    assert_eq!(start.elapsed(), Duration::ZERO);
    limiter.throttle().await;
    limiter.throttle().await;

    assert_eq!(start.elapsed(), Duration::from_millis(200));
}

#[tokio::test(start_paused = true)]
async fn clones_share_one_limit() {
    let limiter = RateLimiter::per_second(10);
    let clone = limiter.clone();
    let start = tokio::time::Instant::now();

    limiter.throttle().await;
    clone.throttle().await;

    assert_eq!(start.elapsed(), Duration::from_millis(100));
}
