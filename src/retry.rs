use std::thread;
use std::time::Duration;

use tracing::warn;

/// Retry a fallible operation with exponential backoff.
///
/// - `max_retries`: number of retry attempts after the first failure (default: 3)
/// - `base_delay_ms`: initial delay in milliseconds, doubled each attempt (default: 500)
/// - Delay is capped at 30 seconds per attempt.
/// - Each retry is logged at `warn!` level.
pub fn retry_with_backoff<T, E, F>(max_retries: u32, base_delay_ms: u64, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    const MAX_DELAY_MS: u64 = 30_000;

    let mut attempt = 0u32;
    loop {
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt >= max_retries {
                    return Err(e);
                }
                let delay_ms = base_delay_ms
                    .saturating_mul(2u64.saturating_pow(attempt))
                    .min(MAX_DELAY_MS);
                warn!(
                    "attempt {}/{} failed: {}; retrying in {}ms",
                    attempt + 1,
                    max_retries + 1,
                    e,
                    delay_ms
                );
                thread::sleep(Duration::from_millis(delay_ms));
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_succeeds_immediately() {
        let result = retry_with_backoff(3, 10, || Ok::<_, String>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_fails_then_succeeds() {
        let count = Cell::new(0u32);
        let result = retry_with_backoff(3, 10, || {
            let c = count.get();
            count.set(c + 1);
            if c < 2 {
                Err(format!("fail #{}", c))
            } else {
                Ok("ok")
            }
        });
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn test_exhausts_retries() {
        let count = Cell::new(0u32);
        let result = retry_with_backoff(2, 10, || {
            let c = count.get();
            count.set(c + 1);
            Err::<(), _>(format!("fail #{}", c))
        });
        assert!(result.is_err());
        // 1 initial + 2 retries = 3 total attempts
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn test_zero_retries_means_single_attempt() {
        let count = Cell::new(0u32);
        let result = retry_with_backoff(0, 10, || {
            count.set(count.get() + 1);
            Err::<(), _>("always fails".to_string())
        });
        assert!(result.is_err());
        assert_eq!(count.get(), 1);
    }
}
