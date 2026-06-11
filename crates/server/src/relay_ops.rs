use tracing::info;
use uuid::Uuid;

use komun_relay::storage::{CommunityConfig, PersistentStore};

pub async fn create_relay_community(
    store: &PersistentStore,
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<String, String> {
    let community_id = Uuid::now_v7().to_string();

    let config = CommunityConfig {
        community_id: community_id.clone(),
        name: name.to_string(),
        genesis_public_key: String::new(),
        public_key: String::new(),
        secret_key: String::new(),
        wrapped_dek: String::new(),
        key_derivation: "random".to_string(),
        published: visibility != "private",
        visibility: visibility.to_string(),
        description: description.to_string(),
        owner_pubkey: String::new(),
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
        password_hash: None,
        join_wrapped_dek: None,
        used_token_nonces: vec![],
    };

    store.register_community(config).await;
    store.mark_dirty();

    info!("[relay] created map community {} for {}", community_id, name);

    Ok(community_id)
}
