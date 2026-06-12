use tracing::info;
use uuid::Uuid;

use komun_relay::storage::{CommunityConfig, PersistentStore};

pub async fn create_relay_community(
    store: &PersistentStore,
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<(String, Vec<u8>), String> {
    let community_id = Uuid::now_v7().to_string();
    let secret_bytes = Uuid::now_v7().as_bytes().to_vec();

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
            "join_policy": "open",
            "post_kind_emoji": {
                "need": "🆘",
                "offer": "🤝",
                "resource": "📦"
            },
            "post_kind_color": {
                "need": "#ef4444",
                "offer": "#16a34a",
                "resource": "#2563eb"
            },
            "default_schema": {
                "name": "Komun Post",
                "fields": [
                    {"key": "kind", "label": "Type", "type": "choice", "options": ["need", "offer", "resource"]},
                    {"key": "category", "label": "Category", "type": "choice", "options": ["food", "shelter", "health", "transport", "education", "labor", "legal", "other"]},
                    {"key": "urgency", "label": "Urgency", "type": "choice", "options": ["critical", "high", "medium", "low"]},
                    {"key": "quantity", "label": "Quantity", "type": "number"},
                    {"key": "contact", "label": "Contact", "type": "text"}
                ]
            }
        }),
        bounds: None,
        password_hash: None,
        join_wrapped_dek: None,
        used_token_nonces: vec![],
    };

    store.register_community(config).await;
    store.mark_dirty();

    info!("[relay] created map community {} for {}", community_id, name);

    Ok((community_id, secret_bytes))
}
