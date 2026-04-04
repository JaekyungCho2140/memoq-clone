//! IP/username-keyed rate limiting for auth endpoints.
//!
//! Uses the GCRA algorithm via the `governor` crate.
//! The rate limiter is stored in `AppState` and checked inside handlers
//! rather than as a Tower layer, to keep `TestServer` compatibility.

use governor::{clock::QuantaClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use std::{num::NonZeroU32, sync::Arc};

pub type KeyedLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, QuantaClock>;

/// Build a rate limiter that allows `per_minute` requests per key per minute.
pub fn build_limiter(per_minute: u32) -> Arc<KeyedLimiter> {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute.max(1)).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_allows_up_to_quota() {
        let limiter = build_limiter(3);
        let key = "test-ip".to_string();
        assert!(limiter.check_key(&key).is_ok());
        assert!(limiter.check_key(&key).is_ok());
        assert!(limiter.check_key(&key).is_ok());
        // 4th request within the same minute burst should be denied
        assert!(limiter.check_key(&key).is_err());
    }

    #[test]
    fn different_keys_are_independent() {
        let limiter = build_limiter(1);
        assert!(limiter.check_key(&"a".to_string()).is_ok());
        // Different key should still be allowed
        assert!(limiter.check_key(&"b".to_string()).is_ok());
        // But same key again is denied
        assert!(limiter.check_key(&"a".to_string()).is_err());
    }
}
