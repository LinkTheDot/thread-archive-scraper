use rand::prelude::*;
use ratelimit::Ratelimiter;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DeviationRateLimiter {
  rng: Arc<Mutex<ThreadRng>>,
  rate_limiter: Arc<Ratelimiter>,
}

impl DeviationRateLimiter {
  /// The maximum range of deviation in nanoseconds.
  const DEVIATION: u64 = 236_857_093;

  pub fn new() -> anyhow::Result<Self> {
    let rate_limiter = Ratelimiter::builder(
      crate::MAX_REQUEST_RATE_LIMIT,
      crate::BASE_RATE_LIMIT_DURATION,
    )
    .max_tokens(4)
    .build()?;

    Ok(Self {
      rng: Arc::new(Mutex::new(rand::thread_rng())),
      rate_limiter: Arc::new(rate_limiter),
    })
  }

  pub async fn wait(&self) {
    let deviation = self.get_deviation().await;

    while let Err(wait_time) = self.rate_limiter.try_wait() {
      tokio::time::sleep(wait_time).await;
    }

    tokio::time::sleep(deviation).await;
  }

  async fn get_deviation(&self) -> Duration {
    let mut rng = self.rng.lock().await;
    let deviation = rng.gen_range(0..Self::DEVIATION);
    drop(rng);

    Duration::from_nanos(deviation)
  }
}
