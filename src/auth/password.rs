use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PasswordHashError {
    #[error("password hashing failed")]
    Hash,
    #[error("password hashing task failed")]
    Join(#[from] tokio::task::JoinError),
}

pub(crate) async fn hash_password(password: String) -> Result<String, PasswordHashError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| PasswordHashError::Hash)
    })
    .await?
}

pub(crate) async fn verify_password(password: String, encoded_hash: String, ) -> Result<bool, PasswordHashError> {
    Ok(tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&encoded_hash) else {
            return false;
        };

        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    })
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_and_verifies_passwords() {
        let password = "correct horse battery staple".to_owned();
        let hash = hash_password(password.clone())
            .await
            .expect("password hashing should succeed");

        assert_ne!(hash, password);
        assert!(hash.starts_with("$argon2id$"));
        assert!(
            verify_password(password, hash.clone())
                .await
                .expect("password verification should succeed")
        );
        assert!(
            !verify_password("wrong password".to_owned(), hash)
                .await
                .expect("password verification should succeed")
        );
    }

    #[tokio::test]
    async fn rejects_plaintext_or_malformed_stored_passwords() {
        assert!(
            !verify_password("password".to_owned(), "password".to_owned())
                .await
                .expect("malformed stored hashes should be treated as invalid credentials")
        );
    }
}
