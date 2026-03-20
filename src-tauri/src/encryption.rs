use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng as ArgonRng};
use base64::{engine::general_purpose, Engine as _};
use std::sync::{Arc, Mutex};

pub struct EncryptionManager {
    cipher: Option<Aes256Gcm>,
    enabled: bool,
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            cipher: None,
            enabled: false,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn unlock(&mut self, password: &str, salt: &str) -> Result<(), String> {
        let key = Self::derive_key(password, salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;
        
        self.cipher = Some(cipher);
        self.enabled = true;
        Ok(())
    }

    pub fn lock(&mut self) {
        self.cipher = None;
        self.enabled = false;
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        if !self.enabled {
            return Ok(plaintext.to_string());
        }

        let cipher = self.cipher.as_ref()
            .ok_or("Encryption not initialized")?;

        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce_bytes, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&combined))
    }

    pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
        if !self.enabled {
            return Ok(encrypted.to_string());
        }

        let cipher = self.cipher.as_ref()
            .ok_or("Encryption not initialized")?;

        let combined = general_purpose::STANDARD.decode(encrypted)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        if combined.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| format!("UTF-8 conversion failed: {}", e))
    }

    fn derive_key(password: &str, salt_str: &str) -> Result<[u8; 32], String> {
        let salt = SaltString::from_b64(salt_str)
            .map_err(|e| format!("Invalid salt: {}", e))?;

        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

        let hash = password_hash.hash
            .ok_or("No hash generated")?;

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash.as_bytes()[..32]);
        Ok(key)
    }

    pub fn generate_salt() -> String {
        SaltString::generate(&mut ArgonRng).to_string()
    }

    pub fn verify_password(password: &str, hash_str: &str) -> Result<(), String> {
        let parsed_hash = PasswordHash::new(hash_str)
            .map_err(|e| format!("Invalid hash: {}", e))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| "Invalid password".to_string())
    }

    pub fn hash_password(password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut ArgonRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Password hashing failed: {}", e))?;

        Ok(password_hash.to_string())
    }
}

pub type SharedEncryption = Arc<Mutex<EncryptionManager>>;

pub fn create_shared() -> SharedEncryption {
    Arc::new(Mutex::new(EncryptionManager::new()))
}
