use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PasswordHashError {
    #[error("password hashing failed")]
    Hash,
    #[error("password hashing task failed")]
    Join(#[from] tokio::task::JoinError),
    #[error("password hashing semaphore is closed")]
    SemaphoreClosed,
}

pub(crate) struct PasswordService {
    semaphore: Arc<Semaphore>,
}

impl PasswordService {
    pub(crate) fn new(max_concurrency: usize) -> Self {
        Self { semaphore: Arc::new(Semaphore::new(max_concurrency)) }
    }

    pub(crate) async fn hash_password(&self, password: String) -> Result<String, PasswordHashError> {
        self.run_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default().hash_password(password.as_bytes(), &salt).map(|hash| hash.to_string()).map_err(|_| PasswordHashError::Hash)
        })
        .await?
    }

    pub(crate) async fn verify_password(&self, password: String, encoded_hash: String) -> Result<bool, PasswordHashError> {
        self.run_blocking(move || {
            let Ok(parsed_hash) = PasswordHash::new(&encoded_hash) else {
                return false;
            };

            Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
        })
        .await
    }

    async fn run_blocking<T, F>(&self, operation: F) -> Result<T, PasswordHashError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        if self.semaphore.available_permits() == 0 {
            tracing::debug!("Argon2 concurrency limit saturated");
        }

        let permit = self.semaphore.clone().acquire_owned().await.map_err(|_| PasswordHashError::SemaphoreClosed)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation()
        })
        .await
        .map_err(PasswordHashError::Join)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        time::Duration,
    };

    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies_passwords() {
        let service = PasswordService::new(1);
        let password = "correct horse battery staple".to_owned();
        let hash = service.hash_password(password.clone()).await.expect("password hashing should succeed");

        assert_ne!(hash, password);
        assert!(hash.starts_with("$argon2id$"));
        assert!(service.verify_password(password, hash.clone()).await.expect("password verification should succeed"));
        assert!(!service.verify_password("wrong password".to_owned(), hash).await.expect("password verification should succeed"));
    }

    #[tokio::test]
    async fn rejects_plaintext_or_malformed_stored_passwords() {
        let service = PasswordService::new(1);
        assert!(!service.verify_password("password".to_owned(), "password".to_owned()).await.expect("malformed stored hashes should be treated as invalid credentials"));
    }

    #[tokio::test]
    async fn blocking_operations_never_exceed_the_semaphore_limit() {
        let service = Arc::new(PasswordService::new(2));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..12 {
            let service = service.clone();
            let active = active.clone();
            let maximum = maximum.clone();
            tasks.push(tokio::spawn(async move {
                service
                    .run_blocking(move || {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        maximum.fetch_max(current, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(10));
                        active.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await
                    .unwrap();
            }));
        }

        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cancelled_waiter_does_not_release_permit_before_blocking_work_finishes() {
        let service = Arc::new(PasswordService::new(1));
        let first_started = Arc::new(AtomicBool::new(false));
        let first_service = service.clone();
        let first_started_in_task = first_started.clone();
        let first = tokio::spawn(async move {
            first_service
                .run_blocking(move || {
                    first_started_in_task.store(true, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(60));
                })
                .await
        });

        while !first_started.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
        first.abort();

        let second_started = Arc::new(AtomicBool::new(false));
        let second_service = service.clone();
        let second_started_in_task = second_started.clone();
        let second = tokio::spawn(async move { second_service.run_blocking(move || second_started_in_task.store(true, Ordering::SeqCst)).await });

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!second_started.load(Ordering::SeqCst));
        second.await.unwrap().unwrap();
        assert!(second_started.load(Ordering::SeqCst));
    }
}
