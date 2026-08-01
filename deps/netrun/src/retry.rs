use std::time::Duration;

use anyhow::{Result, bail};
use log::debug;

pub struct Retry {
    times:   usize,
    timeout: u64,
}

impl Retry {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            times:   3,
            timeout: 500,
        }
    }

    pub fn times(times: usize) -> Self {
        Self { times, timeout: 500 }
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn run<Ret, F>(mut self, fut: impl Fn() -> F) -> Result<Ret>
    where F: Future<Output = Result<Ret>> {
        let timeout = Duration::from_millis(self.timeout);

        assert_ne!(self.times, 0, "Trying to retry 0 times");

        while self.times > 0 {
            self.times -= 1;
            match tokio::time::timeout(timeout, fut()).await {
                Ok(Ok(r)) => return Ok(r),
                Ok(Err(err)) => {
                    debug!("Execution failed: {err}. Retries left: {}", self.times);
                    if self.times == 0 {
                        return Err(err);
                    }
                }
                Err(err) => {
                    debug!("Execution timeout: {err}. Retries left: {}", self.times);
                }
            }
        }

        bail!("Retry exceeded")
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use anyhow::anyhow;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use tokio::time::sleep;

    use super::*;

    /// This used to connect to a dead port and assert the errno of the refusal,
    /// which named the mac number for every platform except linux. On windows
    /// the same connect never got refused, it timed out, so the run ended with
    /// "Retry exceeded" and the test failed on every CI run.
    #[test(tokio::test)]
    async fn test_retry_failure() -> Result<()> {
        let attempts = AtomicUsize::new(0);

        let result: Result<()> = Retry::times(5)
            .run(|| async {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                Err(anyhow!("Attempt {attempt} failed"))
            })
            .await;

        assert_eq!("Attempt 5 failed", result.err().unwrap().to_string());
        assert_eq!(5, attempts.load(Ordering::Relaxed));

        Ok(())
    }

    #[test(tokio::test)]
    async fn test_retry_timeout() -> Result<()> {
        let result: Result<()> = Retry::times(5)
            .timeout(100)
            .run(|| async {
                sleep(Duration::from_secs_f32(5.0)).await;
                Ok(())
            })
            .await;

        assert_eq!(
            anyhow!("Retry exceeded").to_string(),
            result.err().unwrap().to_string()
        );

        Ok(())
    }

    #[test(tokio::test)]
    async fn test_retry_success() -> Result<()> {
        let result: u32 = Retry::times(5)
            .timeout(100)
            .run(|| async {
                sleep(Duration::from_secs_f32(0.0001)).await;
                Ok(10)
            })
            .await?;

        assert_eq!(10, result,);

        Ok(())
    }
}
