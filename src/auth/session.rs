use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::storage::cache::{Cache, CacheError};

/// The amount of time a session remains valid in the session store.
///
/// TODO: Decide whether this should eventually come from configuration.
const SESSION_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 1);

/// Represents an authenticated session.
///
/// The session itself is server-side state. The browser will only receive
/// the session ID through an HTTP cookie.
///
/// For example:
///
///     Browser cookie:
///         session=abc123
///
///     Redis:
///         session:abc123
///             -> member_id = 42
///             -> organization_id = 7
///
/// TODO(security): Right now this struct is serialized to plain JSON before
/// being stored in Redis. If Redis is ever compromised (backup leak, replica
/// with weaker access controls, misconfigured exposure, etc.) an attacker
/// gets member_id/organization_id for free. Worth experimenting with
/// encrypting the serialized bytes at rest (e.g. AES-GCM with a key from the
/// app's secret store) so a raw `GET session:<id>` from Redis just returns
/// ciphertext. Session ID itself can stay a plain lookup key -- it's already
/// high-entropy and unguessable, it's the *payload* that needs protecting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Random identifier used to find this session in the session store.
    pub id: String,

    /// The member who is authenticated by this session.
    pub member_id: i64,

    /// The organization the member belongs to.
    pub organization_id: i64,
}

/// Errors that can occur while creating, retrieving, or deleting sessions.
///
/// These two variants are kept separate even though, as of writing, both
/// currently result in the same user-facing outcome (a generic
/// "server error, try again" redirect). The split exists for
/// *observability*, not for user-facing branching:
///
/// - `Cache` failures are expected to happen occasionally in normal
///   operation -- a network blip, Redis being briefly unreachable, a
///   timeout. These are transient and "try again" is an honest thing to
///   tell the user.
///
/// - `Serialization` failures are NOT expected to happen in normal
///   operation. If a `Session` we just serialized fails to deserialize
///   back, that points to an actual bug -- e.g. the `Session` struct
///   shape changed and old cached sessions are no longer compatible, or
///   the stored bytes were corrupted/tampered with. "Try again" will not
///   fix this class of error, and it's the kind of thing we'd want to
///   know about (log at a higher severity / alert on) rather than let
///   silently fall into the same bucket as ordinary cache flakiness.
///
/// Keeping them as distinct variants means callers *can* log or alert on
/// them differently even while showing the user the same message.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// Something went wrong while communicating with the underlying cache.
    #[error("session storage error")]
    Cache(#[from] CacheError),

    /// The session could not be serialized or deserialized.
    #[error("session (de)serialization error")]
    Serialization(#[from] serde_json::Error),

    // Add other session-specific errors if they become necessary.
}

/// Manages the lifecycle of authenticated sessions.
///
/// The manager does not know or care that the underlying storage is Redis.
/// It only depends on the generic `Cache` abstraction.
///
/// This means the session system could later use another cache/storage
/// implementation without changing the authentication code.
pub struct SessionManager {
    cache: Arc<dyn Cache>,
}

impl SessionManager {
    /// Creates a new session manager using the application's cache.
    pub fn new(cache: Arc<dyn Cache>) -> Self {
        Self { cache }
    }

    /// Creates a new authenticated session for a member.
    ///
    /// The session ID is cryptographically random and unpredictable.
    /// Only the session ID will eventually be sent to the browser as a cookie.
    pub async fn create(&self, member_id: i64, organization_id: i64) -> Result<Session, SessionError> {
        let session_id = self.generate_session_id();

        let session = Session {
            id: session_id,
            member_id,
            organization_id,
        };

        // TODO(security): look into ways if needed encrypt the data in redis so no attacked can use / desqrilize data and use.
        let value = serde_json::to_vec(&session)?;

        let key = Self::cache_key(&session.id);

        self.cache.set(&key, &value, SESSION_TTL).await?;

        Ok(session)
    }

    /// Looks up an existing session by its session ID.
    ///
    /// Returns `Ok(None)` when the session does not exist or has expired.
    pub async fn get(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        let key = Self::cache_key(session_id);

        let Some(value) = self.cache.get(&key).await? else {
            return Ok(None);
        };

        // TODO(security): decrypt `value` here once at-rest encryption
        // is added, before handing it to serde_json.
        let session = serde_json::from_slice(&value)?;

        Ok(Some(session))
    }

    /// Deletes a session, effectively logging the member out.
    pub async fn delete(&self, session_id: &str) -> Result<(), SessionError> {
        let key = Self::cache_key(session_id);
        self.cache.delete(&key).await?;
        Ok(())
    }

    /// Builds the namespaced Redis key for a given session ID.
    fn cache_key(session_id: &str) -> String {
        format!("session:{session_id}")
    }

    /// Generates a new cryptographically secure random session ID.
    pub fn generate_session_id(&self) -> String {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes);
        hex::encode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A fake, in-memory stand-in for Redis. Implements the same `Cache`
    /// trait `RedisCache` does, so `SessionManager` can't tell the
    /// difference -- this is the whole point of depending on `dyn Cache`
    /// rather than a concrete Redis type.
    ///
    /// Note: this fake ignores TTL entirely, so it can prove
    /// create/get/delete logic is correct, but says nothing about whether
    /// real Redis expiry actually works. That needs a real-Redis
    /// integration test later.
    #[derive(Default)]
    struct FakeCache {
        store: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl Cache for FakeCache {
        async fn set(&self, key: &str, value: &[u8], _ttl: Duration) -> Result<(), CacheError> {
            self.store.lock().unwrap().insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            Ok(self.store.lock().unwrap().get(key).cloned())
        }

        async fn delete(&self, key: &str) -> Result<(), CacheError> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    fn manager_with_fake_cache() -> SessionManager {
        SessionManager::new(Arc::new(FakeCache::default()))
    }

    #[tokio::test]
    async fn create_then_get_returns_the_same_session() {
        let manager = manager_with_fake_cache();

        let created = manager.create(42, 7).await.expect("create should succeed");
        let fetched = manager.get(&created.id).await.expect("get should succeed");

        let fetched = fetched.expect("session should exist right after creation");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.member_id, 42);
        assert_eq!(fetched.organization_id, 7);
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_session_id() {
        let manager = manager_with_fake_cache();

        let result = manager.get("this-session-id-was-never-created").await.expect("get should not error");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_then_get_returns_none() {
        let manager = manager_with_fake_cache();

        let created = manager.create(1, 1).await.expect("create should succeed");
        manager.delete(&created.id).await.expect("delete should succeed");

        let result = manager.get(&created.id).await.expect("get should not error");
        assert!(result.is_none(), "session should be gone after delete");
    }

    #[tokio::test]
    async fn two_created_sessions_have_different_ids() {
        let manager = manager_with_fake_cache();

        let a = manager.create(1, 1).await.expect("create should succeed");
        let b = manager.create(1, 1).await.expect("create should succeed");

        assert_ne!(a.id, b.id);
    }

    #[tokio::test]
    async fn generated_session_ids_are_64_hex_characters() {
        let manager = manager_with_fake_cache();
        let id = manager.generate_session_id();

        assert_eq!(id.len(), 64, "32 random bytes hex-encoded should be 64 chars");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}