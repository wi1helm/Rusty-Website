use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::http::HeaderValue;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::{
    config::{RateLimitConfig, TrustedProxyNetwork},
    storage::cache::Cache,
};

type HmacSha256 = Hmac<Sha256>;

pub struct RateLimiter {
    cache: Arc<dyn Cache>,
    config: RateLimitConfig,
    hmac_key: Vec<u8>,
    trusted_proxies: Vec<TrustedProxyNetwork>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitPolicy {
    LoginIp,
    LoginAccount,
    LoginIpAccount,
    SignupIp,
    SignupEmail,
    SignupDailyIp,
}

impl RateLimitPolicy {
    fn key_name(self) -> &'static str {
        match self {
            Self::LoginIp => "login:ip",
            Self::LoginAccount => "login:account",
            Self::LoginIpAccount => "login:ip-account",
            Self::SignupIp => "signup:ip-hour",
            Self::SignupEmail => "signup:email-hour",
            Self::SignupDailyIp => "signup:ip-day",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limiting infrastructure is unavailable")]
    Unavailable,
    #[error("rate limit exceeded")]
    Exceeded { policy: RateLimitPolicy, retry_after: Duration },
}

pub struct LoginAccountReservation {
    cache: Arc<dyn Cache>,
    key: String,
    attempt_number: u64,
}

impl LoginAccountReservation {
    /// Commits this provisional slot as a failed credential attempt.
    pub fn record_failure(self) -> Duration {
        let exponent = self.attempt_number.saturating_sub(1).min(5) as u32;
        Duration::from_millis(100 * 2u64.pow(exponent))
    }

    /// Releases the provisional slot for successful logins and infrastructure errors.
    pub async fn release(self) -> Result<(), RateLimitError> {
        self.cache.decrement(&self.key).await.map_err(|error| {
            tracing::error!(?error, "Redis rate-limiter release failed");
            RateLimitError::Unavailable
        })
    }
}

impl RateLimiter {
    pub fn new(cache: Arc<dyn Cache>, config: RateLimitConfig, hmac_key: String, trusted_proxies: Vec<TrustedProxyNetwork>) -> Self {
        Self { cache, config, hmac_key: hmac_key.into_bytes(), trusted_proxies }
    }

    /// Uses forwarded client addresses only when the immediate network peer is explicitly trusted.
    pub fn client_ip(&self, peer: SocketAddr, forwarded_for: Option<&HeaderValue>) -> IpAddr {
        if !self.trusted_proxies.iter().any(|network| network.contains(peer.ip())) {
            return peer.ip();
        }

        let Some(forwarded_for) = forwarded_for.and_then(|value| value.to_str().ok()) else {
            return peer.ip();
        };

        // Walk from the closest forwarded hop toward the client, ignoring trusted proxies.
        // The first untrusted address is the client even when an attacker supplied earlier
        // entries in the header; the edge proxy must append/overwrite the actual peer address.
        for address in forwarded_for.split(',').rev().map(str::trim) {
            let Ok(address) = address.parse::<IpAddr>() else {
                tracing::warn!("malformed X-Forwarded-For from trusted proxy; using proxy address");
                return peer.ip();
            };
            if !self.trusted_proxies.iter().any(|network| network.contains(address)) {
                return address;
            }
        }

        peer.ip()
    }

    pub async fn check_login_request(&self, ip: IpAddr, email: &str) -> Result<(), RateLimitError> {
        self.check_login_ip_request(ip).await?;
        self.check_login_ip_account_request(ip, email).await
    }

    pub async fn check_login_ip_request(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        self.increment(RateLimitPolicy::LoginIp, &ip.to_string(), self.config.login_ip_limit, self.config.login_ip_window).await
    }

    pub async fn check_login_ip_account_request(&self, ip: IpAddr, email: &str) -> Result<(), RateLimitError> {
        self.increment(RateLimitPolicy::LoginIpAccount, &format!("{ip}\0{email}"), self.config.login_ip_account_limit, self.config.login_ip_account_window).await
    }

    /// Reserves one account attempt before authentication, so concurrent requests cannot
    /// all pass the failed-attempt threshold. Call `record_failure` for invalid credentials,
    /// or `release` for success and infrastructure failures.
    pub async fn reserve_login_account(self: &Arc<Self>, email: &str) -> Result<LoginAccountReservation, RateLimitError> {
        let key = self.key(RateLimitPolicy::LoginAccount, email);
        let (attempt_number, _) = self.increment_key(&key, self.config.login_account_limit, self.config.login_account_window, RateLimitPolicy::LoginAccount).await?;
        Ok(LoginAccountReservation { cache: self.cache.clone(), key, attempt_number })
    }

    pub async fn check_signup_attempt(&self, ip: IpAddr, email: &str) -> Result<(), RateLimitError> {
        self.check_signup_ip_request(ip).await?;
        self.check_signup_email_attempt(email).await
    }

    pub async fn check_signup_ip_request(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        self.increment(RateLimitPolicy::SignupIp, &ip.to_string(), self.config.signup_ip_limit, self.config.signup_ip_window).await?;
        self.increment(RateLimitPolicy::SignupDailyIp, &ip.to_string(), self.config.signup_daily_ip_limit, self.config.signup_daily_ip_window).await
    }

    pub async fn check_signup_email_attempt(&self, email: &str) -> Result<(), RateLimitError> {
        self.increment(RateLimitPolicy::SignupEmail, email, self.config.signup_email_limit, self.config.signup_email_window).await?;
        Ok(())
    }

    async fn increment(&self, policy: RateLimitPolicy, identifier: &str, limit: u64, window: Duration) -> Result<(), RateLimitError> {
        let key = self.key(policy, identifier);
        self.increment_key(&key, limit, window, policy).await.map(|_| ())
    }

    async fn increment_key(&self, key: &str, limit: u64, window: Duration, policy: RateLimitPolicy) -> Result<(u64, Duration), RateLimitError> {
        let (count, retry_after) = self.cache.increment_with_expiry(key, window).await.map_err(|error| {
            tracing::error!(?error, policy = policy.key_name(), "Redis rate-limiter operation failed");
            RateLimitError::Unavailable
        })?;
        tracing::debug!(policy = policy.key_name(), count, "rate-limit counter updated");

        if count > limit {
            tracing::warn!(policy = policy.key_name(), "rate limit rejected request");
            return Err(RateLimitError::Exceeded { policy, retry_after });
        }

        Ok((count, retry_after))
    }

    fn key(&self, policy: RateLimitPolicy, identifier: &str) -> String {
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&self.hmac_key).expect("HMAC accepts keys of any length");
        mac.update(policy.key_name().as_bytes());
        mac.update(&[0]);
        mac.update(identifier.as_bytes());
        format!("rate-limit:v1:{}:{}", policy.key_name(), hex::encode(mac.finalize().into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        net::SocketAddr,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Instant,
    };

    use crate::storage::cache::CacheError;
    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct MemoryCache {
        values: Mutex<HashMap<String, (u64, Instant)>>,
        fail: AtomicBool,
    }

    #[async_trait]
    impl Cache for MemoryCache {
        async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            self.check_failure()?;
            Ok(None)
        }

        async fn set(&self, _key: &str, _value: &[u8], _ttl: Duration) -> Result<(), CacheError> {
            self.check_failure()
        }

        async fn delete(&self, key: &str) -> Result<(), CacheError> {
            self.check_failure()?;
            self.values.lock().unwrap().remove(key);
            Ok(())
        }

        async fn increment_with_expiry(&self, key: &str, ttl: Duration) -> Result<(u64, Duration), CacheError> {
            self.check_failure()?;
            let mut values = self.values.lock().unwrap();
            let now = Instant::now();
            let entry = values.entry(key.to_owned()).or_insert((0, now + ttl));
            if entry.1 <= now {
                *entry = (0, now + ttl);
            }
            entry.0 += 1;
            Ok((entry.0, entry.1.saturating_duration_since(now)))
        }

        async fn decrement(&self, key: &str) -> Result<(), CacheError> {
            self.check_failure()?;
            let mut values = self.values.lock().unwrap();
            if let Some((count, _)) = values.get_mut(key) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    values.remove(key);
                }
            }
            Ok(())
        }
    }

    impl MemoryCache {
        fn check_failure(&self) -> Result<(), CacheError> {
            if self.fail.load(Ordering::Relaxed) { Err(CacheError::Unavailable) } else { Ok(()) }
        }
    }

    fn limiter(cache: Arc<dyn Cache>) -> Arc<RateLimiter> {
        Arc::new(RateLimiter::new(
            cache,
            RateLimitConfig {
                login_ip_limit: 100,
                login_ip_window: Duration::from_secs(60),
                login_account_limit: 5,
                login_account_window: Duration::from_secs(300),
                login_ip_account_limit: 5,
                login_ip_account_window: Duration::from_secs(300),
                signup_ip_limit: 5,
                signup_ip_window: Duration::from_secs(3600),
                signup_email_limit: 3,
                signup_email_window: Duration::from_secs(3600),
                signup_daily_ip_limit: 20,
                signup_daily_ip_window: Duration::from_secs(86400),
            },
            "a test hmac secret with enough entropy bytes".to_owned(),
            Vec::new(),
        ))
    }

    #[tokio::test]
    async fn allows_requests_within_limit_and_rejects_the_next_request() {
        let cache = Arc::new(MemoryCache::default());
        let limiter = limiter(cache);

        for index in 0..5 {
            let email = format!("member{index}@example.com");
            limiter.check_signup_attempt("192.0.2.1".parse().unwrap(), &email).await.unwrap();
        }
        let error = limiter.check_signup_attempt("192.0.2.1".parse().unwrap(), "one@example.com").await.unwrap_err();

        assert!(matches!(error, RateLimitError::Exceeded { policy: RateLimitPolicy::SignupIp, .. }));
    }

    #[tokio::test]
    async fn signup_email_limit_is_shared_across_ips_and_accounts_are_independent() {
        let limiter = limiter(Arc::new(MemoryCache::default()));

        for ip in ["192.0.2.1", "192.0.2.2", "192.0.2.3"] {
            limiter.check_signup_attempt(ip.parse().unwrap(), "one@example.com").await.unwrap();
        }
        assert!(matches!(limiter.check_signup_attempt("192.0.2.4".parse().unwrap(), "one@example.com").await, Err(RateLimitError::Exceeded { policy: RateLimitPolicy::SignupEmail, .. })));
        limiter.check_signup_attempt("192.0.2.4".parse().unwrap(), "two@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn failed_account_attempts_are_shared_across_ips_and_success_releases_reservation() {
        let limiter = limiter(Arc::new(MemoryCache::default()));

        for _ in 0..5 {
            limiter.reserve_login_account("same@example.com").await.unwrap().record_failure();
        }
        assert!(matches!(limiter.reserve_login_account("same@example.com").await, Err(RateLimitError::Exceeded { policy: RateLimitPolicy::LoginAccount, .. })));
        limiter.reserve_login_account("different@example.com").await.unwrap().release().await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_requests_cannot_exceed_an_ip_account_limit() {
        let cache = Arc::new(MemoryCache::default());
        let mut limits = limiter(cache.clone());
        Arc::get_mut(&mut limits).unwrap().config.login_ip_account_limit = 5;

        let mut tasks = Vec::new();
        for _ in 0..32 {
            let limiter = limits.clone();
            tasks.push(tokio::spawn(async move { limiter.check_login_request("192.0.2.5".parse().unwrap(), "same@example.com").await.is_ok() }));
        }
        let mut accepted = 0;
        for task in tasks {
            accepted += usize::from(task.await.unwrap());
        }

        assert_eq!(accepted, 5);
    }

    #[tokio::test]
    async fn expired_counter_starts_a_new_window() {
        let cache = Arc::new(MemoryCache::default());
        let limiter = Arc::new(RateLimiter::new(
            cache,
            RateLimitConfig {
                login_ip_limit: 1,
                login_ip_window: Duration::from_millis(20),
                login_account_limit: 1,
                login_account_window: Duration::from_secs(1),
                login_ip_account_limit: 1,
                login_ip_account_window: Duration::from_millis(20),
                signup_ip_limit: 1,
                signup_ip_window: Duration::from_secs(1),
                signup_email_limit: 1,
                signup_email_window: Duration::from_secs(1),
                signup_daily_ip_limit: 1,
                signup_daily_ip_window: Duration::from_secs(1),
            },
            "a test hmac secret with enough entropy bytes".to_owned(),
            Vec::new(),
        ));

        limiter.check_login_request("192.0.2.1".parse().unwrap(), "one@example.com").await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        limiter.check_login_request("192.0.2.1".parse().unwrap(), "one@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn redis_failures_are_distinguishable_from_rejections() {
        let cache = Arc::new(MemoryCache::default());
        cache.fail.store(true, Ordering::Relaxed);
        let limiter = limiter(cache);

        assert!(matches!(limiter.check_signup_attempt("192.0.2.1".parse().unwrap(), "one@example.com").await, Err(RateLimitError::Unavailable)));
    }

    #[tokio::test]
    async fn login_and_signup_use_independent_counter_namespaces() {
        let limiter = limiter(Arc::new(MemoryCache::default()));

        for _ in 0..4 {
            limiter.check_login_request("192.0.2.1".parse().unwrap(), "one@example.com").await.unwrap();
        }
        limiter.check_signup_attempt("192.0.2.1".parse().unwrap(), "one@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn signup_daily_ip_budget_is_independent_of_the_hourly_budget() {
        let mut config = RateLimitConfig::default();
        config.signup_ip_limit = 100;
        config.signup_email_limit = 100;
        config.signup_daily_ip_limit = 2;
        let limiter = Arc::new(RateLimiter::new(Arc::new(MemoryCache::default()), config, "a test hmac secret with enough entropy bytes".to_owned(), Vec::new()));
        let ip = "192.0.2.8".parse().unwrap();

        limiter.check_signup_ip_request(ip).await.unwrap();
        limiter.check_signup_ip_request(ip).await.unwrap();
        assert!(matches!(limiter.check_signup_ip_request(ip).await, Err(RateLimitError::Exceeded { policy: RateLimitPolicy::SignupDailyIp, .. })));
    }

    #[tokio::test]
    async fn concurrent_account_reservations_cannot_exceed_the_failed_attempt_limit() {
        let limiter = limiter(Arc::new(MemoryCache::default()));
        let mut tasks = Vec::new();

        for _ in 0..32 {
            let limiter = limiter.clone();
            tasks.push(tokio::spawn(async move {
                match limiter.reserve_login_account("same@example.com").await {
                    Ok(reservation) => {
                        reservation.record_failure();
                        true
                    }
                    Err(_) => false,
                }
            }));
        }

        let mut accepted = 0;
        for task in tasks {
            accepted += usize::from(task.await.unwrap());
        }
        assert_eq!(accepted, 5);
    }

    #[test]
    fn account_keys_do_not_contain_raw_email_addresses() {
        let limiter = limiter(Arc::new(MemoryCache::default()));
        let key = limiter.key(RateLimitPolicy::LoginAccount, "private@example.com");

        assert!(key.starts_with("rate-limit:v1:login:account:"));
        assert!(!key.contains("private@example.com"));
    }

    #[test]
    fn forwarded_ips_are_used_only_for_trusted_proxy_peers() {
        let limiter = RateLimiter::new(
            Arc::new(MemoryCache::default()),
            RateLimitConfig {
                login_ip_limit: 1,
                login_ip_window: Duration::from_secs(1),
                login_account_limit: 1,
                login_account_window: Duration::from_secs(1),
                login_ip_account_limit: 1,
                login_ip_account_window: Duration::from_secs(1),
                signup_ip_limit: 1,
                signup_ip_window: Duration::from_secs(1),
                signup_email_limit: 1,
                signup_email_window: Duration::from_secs(1),
                signup_daily_ip_limit: 1,
                signup_daily_ip_window: Duration::from_secs(1),
            },
            "a test hmac secret with enough entropy bytes".to_owned(),
            vec!["10.0.0.0/8".parse().unwrap()],
        );
        let header = HeaderValue::from_static("203.0.113.8, 10.1.2.3");

        assert_eq!(limiter.client_ip("10.1.2.3:443".parse::<SocketAddr>().unwrap(), Some(&header)), "203.0.113.8".parse::<IpAddr>().unwrap());
        assert_eq!(limiter.client_ip("198.51.100.4:443".parse().unwrap(), Some(&header)), "198.51.100.4".parse::<IpAddr>().unwrap());
    }
}
