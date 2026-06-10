use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

fn derive_key(password: &str, room_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(format!("rustcanvas:{room_id}:{password}"));
    hasher.finalize().into()
}

pub struct RoomCipher {
    key: [u8; 32],
}

impl RoomCipher {
    pub fn from_password(password: &str, room_id: &str) -> Self {
        Self {
            key: derive_key(password, room_id),
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        let mut nonce_bytes = [0u8; 12];
        getrandom::fill(&mut nonce_bytes).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut out = nonce_bytes.to_vec();
        out.extend(ciphertext);
        Ok(STANDARD.encode(out))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let raw = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
        if raw.len() < 13 {
            return Err("ciphertext too short".into());
        }
        let (nonce_bytes, data) = raw.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = cipher.decrypt(nonce, data).map_err(|e| e.to_string())?;
        String::from_utf8(plain).map_err(|e| e.to_string())
    }
}
