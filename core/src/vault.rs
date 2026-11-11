//! The app's own store for sign-ins: sealed with a password the user sets (Argon2id for the key,
//! ChaCha20-Poly1305 for the data). Nothing here touches the system's keychain.

use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

const MARK: &[u8; 4] = b"HCO1";

fn cipher(password: &str, salt: &[u8]) -> Result<ChaCha20Poly1305, String> {
    let mut key = [0u8; 32];
    Argon2::default().hash_password_into(password.as_bytes(), salt, &mut key).map_err(|e| e.to_string())?;
    ChaCha20Poly1305::new_from_slice(&key).map_err(|e| e.to_string())
}

/// Seals `data` under `password`. Every call uses a fresh salt and nonce.
pub fn seal(password: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut salt).map_err(|e| e.to_string())?;
    getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
    let sealed = cipher(password, &salt)?.encrypt(&Nonce::from(nonce), data).map_err(|e| e.to_string())?;
    Ok([&MARK[..], &salt, &nonce, &sealed].concat())
}

/// Opens what `seal` made. A wrong password and a damaged file give the same error.
pub fn open(password: &str, sealed: &[u8]) -> Result<Vec<u8>, String> {
    let wrong = "The password is not right, or the file is damaged.";
    if sealed.len() < 32 || &sealed[..4] != MARK {
        return Err(wrong.into());
    }
    let (salt, rest) = sealed[4..].split_at(16);
    let (nonce, data) = rest.split_at(12);
    let nonce: [u8; 12] = nonce.try_into().map_err(|_| wrong.to_string())?;
    cipher(password, salt)?.decrypt(&Nonce::from(nonce), data).map_err(|_| wrong.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_sealed_opens_only_with_its_password() {
        let sealed = seal("correct horse", b"secret").unwrap();
        assert!(!sealed.windows(6).any(|w| w == b"secret"));
        assert_eq!(open("correct horse", &sealed).unwrap(), b"secret");
        assert!(open("wrong", &sealed).is_err());
        let mut damaged = sealed.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(open("correct horse", &damaged).is_err());
        assert_ne!(seal("correct horse", b"secret").unwrap(), sealed);
    }
}
