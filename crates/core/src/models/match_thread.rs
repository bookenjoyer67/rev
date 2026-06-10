use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchStatus {
    Proposed,
    Accepted,
    Completed,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AidMatch {
    pub id: Uuid,
    pub post_id: Uuid,
    pub responder_id: Uuid,
    pub responder_post_id: Option<Uuid>,
    pub message: Option<String>,
    pub status: MatchStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMatch {
    pub message: Option<String>,
    pub responder_post_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub match_id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub body: String,
}
