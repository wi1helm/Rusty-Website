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
        // Generate a new random identifier for this session.
        //
        // This ID becomes the secret that the browser presents later
        // to prove that it has an authenticated session.
        let session_id = self.generate_session_id();

        let session = Session {
            id: session_id,
            member_id,
            organization_id,
        };

        // Serialize the session before storing it in the cache.
        //
        // The cache intentionally only knows about raw bytes, so
        // serialization is the responsibility of the session layer.
        //
        // TODO(security): look into ways if needed encrypt the data in redis so no attacked can use / desqrilize data and use.
        let value = serde_json::to_vec(&session)?;

        // Use a namespaced cache key so this doesn't collide with other
        // data stored in Redis, such as password-reset tokens,
        // email-verification tokens, rate limits, etc.
        let key = Self::cache_key(&session.id);

        // Store the session in the cache with the configured expiration time.
        //
        // Redis will automatically remove the session once SESSION_TTL
        // expires.
        self.cache.set(&key, &value, SESSION_TTL).await?;

        Ok(session)
    }

    /// Looks up an existing session by its session ID.
    ///
    /// Returns `Ok(None)` when the session does not exist or has expired.
    pub async fn get(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        // Build the same cache key used by `create()`.
        let key = Self::cache_key(session_id);

        // Ask the cache for the stored session.
        let Some(value) = self.cache.get(&key).await? else {
            // Redis returns no value when the key does not exist.
            //
            // This can mean either:
            // - the session was explicitly deleted, or
            // - the session's TTL expired.
            return Ok(None);
        };

        // Deserialize the cached bytes back into a Session.
        //
        // TODO(security): decrypt `value` here once at-rest encryption
        // is added, before handing it to serde_json.
        let session = serde_json::from_slice(&value)?;

        Ok(Some(session))
    }

    /// Deletes a session, effectively logging the member out.
    pub async fn delete(&self, session_id: &str) -> Result<(), SessionError> {
        // Build the same cache key used by `create()` and `get()`.
        let key = Self::cache_key(session_id);

        // Removing the session from the cache immediately invalidates it.
        //
        // The browser may still possess the old cookie, but that cookie
        // will no longer correspond to a valid server-side session.
        self.cache.delete(&key).await?;

        Ok(())
    }

    /// Builds the namespaced Redis key for a given session ID.
    ///
    /// e.g. `session:abc123`
    fn cache_key(session_id: &str) -> String {
        format!("session:{session_id}")
    }

    /// Generates a new cryptographically secure random session ID.
    ///
    /// The session ID is used as the identifier for an authenticated session.
    /// It must be unpredictable so that an attacker cannot guess another
    /// user's session ID and use it to authenticate as them.
    ///
    /// 32 random bytes = 256 bits of randomness.
    ///
    /// The bytes are encoded as hexadecimal so the result can safely be used
    /// as a string, for example as a Redis key and later as a cookie value.
    pub fn generate_session_id(&self) -> String {
        // Allocate 32 bytes of random data.
        let mut bytes = [0u8; 32];

        // Fill the byte array using rand's secure random-number generator.
        rand::fill(&mut bytes);

        // Convert the random bytes into a hexadecimal string.
        //
        // 32 random bytes become 64 hexadecimal characters.
        hex::encode(bytes)
    }
}