use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PostKind {
    Resource,
    Need,
    Offer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Food,
    Shelter,
    Health,
    Transport,
    Education,
    Labor,
    Legal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Active,
    Matched,
    Fulfilled,
    Expired,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub community_id: Uuid,
    pub author_id: Uuid,
    pub kind: PostKind,
    pub category: Category,
    pub title: String,
    pub body: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub urgency: Option<Urgency>,
    pub quantity: Option<i32>,
    pub status: PostStatus,
    pub visibility: super::community::Visibility,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub contact_method: Option<String>,
    pub verified_by: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub federated_id: Option<String>,
    pub origin_node: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePost {
    pub kind: PostKind,
    pub category: Category,
    pub title: String,
    pub body: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub urgency: Option<Urgency>,
    pub quantity: Option<i32>,
    pub visibility: Option<super::community::Visibility>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub contact_method: Option<String>,
}
