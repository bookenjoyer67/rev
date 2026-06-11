use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use chacha20poly1305::aead::{Aead, OsRng};
use hkdf::Hkdf;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use tracing::info;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use komun_relay::storage::{CommunityConfig, PersistentStore};

pub fn generate_map_password() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_community_password(password: &str, community_id: &str) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", password, community_id));
    hex::encode(hasher.finalize())
}

pub fn derive_community_keypair(password: &str, community_id: &str) -> (StaticSecret, PublicKey) {
    let mut seed = [0u8; 32];
    let salt = format!("piggpin:v1:pbkdf2:{}", community_id);
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 210_000, &mut seed);
    let secret = StaticSecret::from(seed);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn generate_dek() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

fn derive_ecies_key(dh_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let hk = Hkdf::<Sha256>::new(None, dh_bytes);
    let mut okm = [0u8; 32];
    hk.expand(b"ecies-v1", &mut okm).map_err(|e| format!("hkdf: {}", e))?;
    Ok(okm.to_vec())
}

fn ecies_seal(plaintext: &[u8], recipient_pub: &PublicKey) -> Result<Vec<u8>, String> {
    let ephemeral_sk = StaticSecret::random_from_rng(OsRng);
    let ephemeral_pk = PublicKey::from(&ephemeral_sk);
    let dh_shared = ephemeral_sk.diffie_hellman(recipient_pub);
    if dh_shared.as_bytes().iter().all(|&b| b == 0) {
        return Err("invalid public key (low-order point)".into());
    }
    let okm = derive_ecies_key(dh_shared.as_bytes())?;
    let cipher = ChaCha20Poly1305::new_from_slice(&okm).map_err(|e| format!("cipher: {}", e))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, plaintext).map_err(|e| format!("encrypt: {}", e))?;
    let mut sealed = Vec::with_capacity(32 + 12 + ct.len());
    sealed.extend_from_slice(ephemeral_pk.as_bytes());
    sealed.extend_from_slice(&nonce_bytes);
    sealed.extend_from_slice(&ct);
    Ok(sealed)
}

pub fn wrap_dek(dek: &[u8], public_key_hex: &str) -> Result<String, String> {
    let pk_bytes = hex::decode(public_key_hex).map_err(|e| format!("hex: {}", e))?;
    let pk_bytes: [u8; 32] = pk_bytes.try_into().map_err(|_| "invalid pubkey len")?;
    let pk = PublicKey::from(pk_bytes);
    let sealed = ecies_seal(dek, &pk)?;
    Ok(hex::encode(&sealed))
}

pub async fn create_relay_community(
    store: &PersistentStore,
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<(String, String), String> {
    let community_id = Uuid::now_v7().to_string();
    let password = generate_map_password();
    let password_hash = hash_community_password(&password, &community_id);

    let (secret, public) = derive_community_keypair(&password, &community_id);
    let genesis_public_key = hex::encode(public.as_bytes());
    let public_key_hex = genesis_public_key.clone();

    let dek = generate_dek();
    let wrapped_dek = wrap_dek(&dek, &public_key_hex)?;

    let config = CommunityConfig {
        community_id: community_id.clone(),
        name: name.to_string(),
        genesis_public_key,
        public_key: public_key_hex,
        secret_key: hex::encode(secret.as_bytes()),
        wrapped_dek,
        key_derivation: "pbkdf2".to_string(),
        published: visibility != "private",
        visibility: visibility.to_string(),
        description: description.to_string(),
        owner_pubkey: hex::encode(public.as_bytes()),
        members: vec![],
        governance: serde_json::json!({
            "contribution": "open",
            "validation": "none",
            "schema_authority": "any_member",
            "key_rotation": "founder_only",
            "fork_policy": "allowed",
            "join_policy": "open"
        }),
        bounds: None,
        password_hash: Some(password_hash),
        join_wrapped_dek: None,
        used_token_nonces: vec![],
    };

    store.register_community(config).await;
    store.mark_dirty();

    info!("[relay] created map community {} for {}", community_id, name);

    Ok((community_id, password))
}
