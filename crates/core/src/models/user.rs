use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub public_key: String,
    pub encryption_public_key: Option<String>,
    pub role: String,
    pub community_count: u32,
    pub post_count: u32,
    pub verified_post_count: u32,
    pub endorsement_count: u32,
    pub joined_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    #[serde(default)]
    pub profile_json: serde_json::Value,
}
