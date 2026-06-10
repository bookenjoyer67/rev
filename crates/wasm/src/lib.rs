use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use rand::rngs::OsRng;
use rand_core::RngCore;
use sha2::{Sha256, Digest};
use wasm_bindgen::prelude::*;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};

#[wasm_bindgen]
pub struct KeyPair {
    signing_key: Vec<u8>,
    verifying_key: Vec<u8>,
}

#[wasm_bindgen]
impl KeyPair {
    #[wasm_bindgen(getter)]
    pub fn secret_key(&self) -> Vec<u8> {
        self.signing_key.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn public_key(&self) -> Vec<u8> {
        self.verifying_key.clone()
    }
}

#[wasm_bindgen]
pub fn generate_keypair() -> KeyPair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    KeyPair {
        signing_key: signing_key.to_bytes().to_vec(),
        verifying_key: verifying_key.to_bytes().to_vec(),
    }
}

#[wasm_bindgen]
pub fn sign(message: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, JsValue> {
    let key_bytes: [u8; 32] = secret_key
        .try_into()
        .map_err(|_| JsValue::from_str("invalid secret key length"))?;
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(message);
    Ok(signature.to_bytes().to_vec())
}

#[wasm_bindgen]
pub fn verify(message: &[u8], signature_bytes: &[u8], public_key: &[u8]) -> Result<bool, JsValue> {
    let key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| JsValue::from_str("invalid public key length"))?;
    let sig_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| JsValue::from_str("invalid signature length"))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let signature = Signature::from_bytes(&sig_bytes);

    Ok(verifying_key.verify(message, &signature).is_ok())
}

#[wasm_bindgen]
pub struct X25519KeyPair {
    secret_key: Vec<u8>,
    public_key: Vec<u8>,
}

#[wasm_bindgen]
impl X25519KeyPair {
    #[wasm_bindgen(getter)]
    pub fn secret_key(&self) -> Vec<u8> {
        self.secret_key.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn public_key(&self) -> Vec<u8> {
        self.public_key.clone()
    }
}

#[wasm_bindgen]
pub fn generate_x25519_keypair() -> X25519KeyPair {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = X25519PublicKey::from(&secret);

    X25519KeyPair {
        secret_key: secret.to_bytes().to_vec(),
        public_key: public.to_bytes().to_vec(),
    }
}

#[wasm_bindgen]
pub fn encrypt_message(plaintext: &[u8], recipient_x25519_pk: &[u8]) -> Result<Vec<u8>, JsValue> {
    let recipient_pk_bytes: [u8; 32] = recipient_x25519_pk
        .try_into()
        .map_err(|_| JsValue::from_str("invalid recipient public key length"))?;
    let recipient_pk = X25519PublicKey::from(recipient_pk_bytes);

    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pk);

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    let symmetric_key = hasher.finalize();

    let cipher = XChaCha20Poly1305::new_from_slice(&symmetric_key)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut envelope = Vec::with_capacity(32 + 24 + ciphertext.len());
    envelope.extend_from_slice(ephemeral_public.as_bytes());
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    Ok(envelope)
}

#[wasm_bindgen]
pub fn derive_shared_key(my_x25519_sk: &[u8], their_x25519_pk: &[u8]) -> Result<Vec<u8>, JsValue> {
    let my_sk_bytes: [u8; 32] = my_x25519_sk
        .try_into()
        .map_err(|_| JsValue::from_str("invalid secret key length"))?;
    let their_pk_bytes: [u8; 32] = their_x25519_pk
        .try_into()
        .map_err(|_| JsValue::from_str("invalid public key length"))?;

    let my_secret = StaticSecret::from(my_sk_bytes);
    let their_public = X25519PublicKey::from(their_pk_bytes);
    let shared = my_secret.diffie_hellman(&their_public);

    let mut hasher = Sha256::new();
    hasher.update(shared.as_bytes());
    Ok(hasher.finalize().to_vec())
}

#[wasm_bindgen]
pub fn encrypt_with_shared_key(plaintext: &[u8], shared_key: &[u8]) -> Result<Vec<u8>, JsValue> {
    let key_bytes: [u8; 32] = shared_key
        .try_into()
        .map_err(|_| JsValue::from_str("invalid key length"))?;

    let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut result = Vec::with_capacity(24 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

#[wasm_bindgen]
pub fn decrypt_with_shared_key(data: &[u8], shared_key: &[u8]) -> Result<Vec<u8>, JsValue> {
    if data.len() < 24 + 16 {
        return Err(JsValue::from_str("data too short"));
    }

    let key_bytes: [u8; 32] = shared_key
        .try_into()
        .map_err(|_| JsValue::from_str("invalid key length"))?;

    let nonce = XNonce::from_slice(&data[..24]);
    let ciphertext = &data[24..];

    let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| JsValue::from_str("decryption failed"))
}

#[wasm_bindgen]
pub fn decrypt_message(envelope: &[u8], my_x25519_sk: &[u8]) -> Result<Vec<u8>, JsValue> {
    if envelope.len() < 32 + 24 + 16 {
        return Err(JsValue::from_str("envelope too short"));
    }

    let my_sk_bytes: [u8; 32] = my_x25519_sk
        .try_into()
        .map_err(|_| JsValue::from_str("invalid secret key length"))?;
    let my_secret = StaticSecret::from(my_sk_bytes);

    let ephemeral_pk_bytes: [u8; 32] = envelope[..32]
        .try_into()
        .map_err(|_| JsValue::from_str("invalid ephemeral pk"))?;
    let ephemeral_pk = X25519PublicKey::from(ephemeral_pk_bytes);

    let nonce_bytes: [u8; 24] = envelope[32..56]
        .try_into()
        .map_err(|_| JsValue::from_str("invalid nonce"))?;
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = &envelope[56..];

    let shared_secret = my_secret.diffie_hellman(&ephemeral_pk);

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    let symmetric_key = hasher.finalize();

    let cipher = XChaCha20Poly1305::new_from_slice(&symmetric_key)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| JsValue::from_str("decryption failed"))?;

    Ok(plaintext)
}
