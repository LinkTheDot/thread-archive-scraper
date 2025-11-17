use crate::ratelimiter::DeviationRateLimiter;
use anyhow::anyhow;
use reqwest::{Client, Response};
use std::time::Duration;

/// Sends a GET request to the desired URL, retrying with the desired amount of times if it fails.
///
/// Every failed attempt will wait for the passed in time.
///
/// # Errors
/// - Failed to get a response after the desired amount of attempts.
pub async fn get_with_retry(
  client: &Client,
  request_url: String,
  retry_count: usize,
  rate_limiter: &DeviationRateLimiter,
  wait_time: Duration,
) -> anyhow::Result<Response> {
  for iteration in 1..=retry_count {
    rate_limiter.wait().await;
    let result = client.get(&request_url).send().await;

    if let Ok(response) = result {
      return Ok(response);
    } else {
      tracing::warn!(
        "Failed to get a response from {:?}. {} more attempts left",
        request_url,
        retry_count - iteration
      );
      tokio::time::sleep(wait_time).await;

      continue;
    }
  }

  Err(anyhow!(
    "Failed to get a response from `{:?}` after {:?} tries.",
    request_url,
    retry_count,
  ))
}
