use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("argon2 hash error: {0}")]
    Hash(String),
    #[error("invalid stored hash")]
    InvalidStoredHash,
}

pub fn hash(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::Hash(e.to_string()))
}

/// Returns `Ok(true)` only on a verified match. Constant-time inside argon2.
pub fn verify(password: &str, stored_hash: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(stored_hash).map_err(|_| PasswordError::InvalidStoredHash)?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(PasswordError::Hash(e.to_string())),
    }
}
