//! Conversations.
//!
//! A3.3: the server no longer sees message text. `messages` holds `ciphertext BYTEA NOT NULL`
//! and `nonce BYTEA` — there is no `body` column, and the three statements that wrote or read
//! one were failing against the squashed schema anyway. What crosses this boundary is opaque
//! bytes; base64 is applied at the edge so the JSON stays printable.

use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[derive(Serialize)]
pub struct ConversationPreview {
    pub match_id: Uuid,
    pub post_id: Uuid,
    pub post_title: String,
    pub post_kind: String,
    pub other_party_id: Uuid,
    pub other_party_name: String,
    /// The last message, still sealed. The client holds the conversation key and renders the
    /// preview itself; the old plaintext `last_message` was the one field that made the whole
    /// list readable from the database.
    pub last_message_ciphertext: Option<String>,
    pub last_message_nonce: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Conversation {
    pub match_id: Uuid,
    pub post_id: Uuid,
    pub post_title: String,
    pub post_kind: String,
    pub responder_id: Uuid,
    pub author_id: Uuid,
    pub responder_name: String,
    pub author_name: String,
    pub status: String,
    pub messages: Vec<MessageRow>,
    pub created_at: DateTime<Utc>,
}

/// What goes over the wire: base64 of the stored bytes, never text.
#[derive(Serialize, Clone)]
pub struct MessageRow {
    pub id: Uuid,
    pub match_id: Uuid,
    pub sender_id: Uuid,
    pub ciphertext: String,
    pub nonce: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct MessageDbRow {
    id: Uuid,
    match_id: Uuid,
    sender_id: Uuid,
    ciphertext: Vec<u8>,
    nonce: Option<Vec<u8>>,
    created_at: DateTime<Utc>,
}

impl From<MessageDbRow> for MessageRow {
    fn from(r: MessageDbRow) -> Self {
        MessageRow {
            id: r.id,
            match_id: r.match_id,
            sender_id: r.sender_id,
            ciphertext: b64(&r.ciphertext),
            nonce: r.nonce.as_deref().map(b64),
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct ConversationPreviewRow {
    match_id: Uuid,
    post_id: Uuid,
    post_title: String,
    post_kind: String,
    other_party_id: Uuid,
    other_party_name: String,
    last_message_ciphertext: Option<Vec<u8>>,
    last_message_nonce: Option<Vec<u8>>,
    last_message_at: Option<DateTime<Utc>>,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<ConversationPreviewRow> for ConversationPreview {
    fn from(r: ConversationPreviewRow) -> Self {
        ConversationPreview {
            match_id: r.match_id,
            post_id: r.post_id,
            post_title: r.post_title,
            post_kind: r.post_kind,
            other_party_id: r.other_party_id,
            other_party_name: r.other_party_name,
            last_message_ciphertext: r.last_message_ciphertext.as_deref().map(b64),
            last_message_nonce: r.last_message_nonce.as_deref().map(b64),
            last_message_at: r.last_message_at,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

pub async fn create_match(
    pool: &PgPool,
    post_id: Uuid,
    responder_id: Uuid,
    ciphertext: &[u8],
    nonce: Option<&[u8]>,
) -> Result<(Uuid, Uuid)> {
    let post_author = sqlx::query_scalar::<_, Uuid>("SELECT author_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await?;

    if let Some(author_id) = post_author {
        if author_id == responder_id {
            return Err(anyhow!("cannot respond to your own post"));
        }
    }

    let mut tx = pool.begin().await?;
    let match_id = Uuid::now_v7();
    let message_id = Uuid::now_v7();
    let now = Utc::now();

    // `matches.message` is left NULL on purpose. It is a plaintext TEXT column, and writing the
    // opening message into it as well as into `messages` would put a readable copy of the one
    // thing this card exists to stop storing right back in the database.
    sqlx::query(
        "INSERT INTO matches (id, post_id, responder_id, status, created_at) VALUES ($1, $2, $3, 'proposed', $4)"
    )
    .bind(match_id)
    .bind(post_id)
    .bind(responder_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO messages (id, match_id, sender_id, ciphertext, nonce, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(message_id)
    .bind(match_id)
    .bind(responder_id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    if let Some(author_id) = post_author {
        let responder_name = sqlx::query_scalar::<_, String>("SELECT display_name FROM users WHERE id = $1")
            .bind(responder_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();
        let post_title = sqlx::query_scalar::<_, String>("SELECT title FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();
        super::notifications::create(
            pool, author_id, "response",
            &format!("{} responded to: {}", responder_name, post_title),
            None,
            Some(&format!("/messages/{}", match_id)),
        ).await.ok();
    }

    Ok((match_id, message_id))
}

pub async fn list_conversations(pool: &PgPool, user_id: Uuid) -> Result<Vec<ConversationPreview>> {
    let rows = sqlx::query_as::<_, ConversationPreviewRow>(
        r#"SELECT
            m.id AS match_id,
            m.post_id,
            p.title AS post_title,
            p.kind AS post_kind,
            CASE WHEN p.author_id = $1 THEN m.responder_id ELSE p.author_id END AS other_party_id,
            CASE WHEN p.author_id = $1 THEN ru.display_name ELSE au.display_name END AS other_party_name,
            last.ciphertext AS last_message_ciphertext,
            last.nonce AS last_message_nonce,
            last.created_at AS last_message_at,
            m.status,
            m.created_at
        FROM matches m
        JOIN posts p ON p.id = m.post_id
        JOIN users ru ON ru.id = m.responder_id
        JOIN users au ON au.id = p.author_id
        LEFT JOIN LATERAL (
            SELECT ciphertext, nonce, created_at
            FROM messages WHERE match_id = m.id
            ORDER BY created_at DESC LIMIT 1
        ) last ON true
        WHERE m.responder_id = $1 OR p.author_id = $1
        ORDER BY COALESCE(last.created_at, m.created_at) DESC"#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_conversation(pool: &PgPool, match_id: Uuid, user_id: Uuid) -> Result<Conversation> {
    let row = sqlx::query_as::<_, MatchDetailRow>(
        r#"SELECT
            m.id AS match_id, m.post_id, p.title AS post_title, p.kind AS post_kind,
            m.responder_id, p.author_id,
            ru.display_name AS responder_name, au.display_name AS author_name,
            m.status, m.created_at
        FROM matches m
        JOIN posts p ON p.id = m.post_id
        JOIN users ru ON ru.id = m.responder_id
        JOIN users au ON au.id = p.author_id
        WHERE m.id = $1 AND (m.responder_id = $2 OR p.author_id = $2)"#
    )
    .bind(match_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("conversation not found"))?;

    let messages = sqlx::query_as::<_, MessageDbRow>(
        "SELECT id, match_id, sender_id, ciphertext, nonce, created_at FROM messages WHERE match_id = $1 ORDER BY created_at ASC"
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;

    Ok(Conversation {
        match_id: row.match_id,
        post_id: row.post_id,
        post_title: row.post_title,
        post_kind: row.post_kind,
        responder_id: row.responder_id,
        author_id: row.author_id,
        responder_name: row.responder_name,
        author_name: row.author_name,
        status: row.status,
        messages: messages.into_iter().map(Into::into).collect(),
        created_at: row.created_at,
    })
}

pub async fn send_message(
    pool: &PgPool,
    match_id: Uuid,
    sender_id: Uuid,
    ciphertext: &[u8],
    nonce: Option<&[u8]>,
) -> Result<MessageRow> {
    let participant = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM matches m JOIN posts p ON p.id = m.post_id WHERE m.id = $1 AND ($2 = m.responder_id OR $2 = p.author_id))"
    )
    .bind(match_id)
    .bind(sender_id)
    .fetch_one(pool)
    .await?;

    if !participant {
        return Err(anyhow!("not a participant in this conversation"));
    }

    let id = Uuid::now_v7();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO messages (id, match_id, sender_id, ciphertext, nonce, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(id)
    .bind(match_id)
    .bind(sender_id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(now)
    .execute(pool)
    .await?;

    let other_party = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT CASE WHEN p.author_id = $2 THEN m.responder_id ELSE p.author_id END
           FROM matches m JOIN posts p ON p.id = m.post_id WHERE m.id = $1"#
    )
    .bind(match_id)
    .bind(sender_id)
    .fetch_optional(pool)
    .await?;

    if let Some(recipient_id) = other_party {
        let sender_name = sqlx::query_scalar::<_, String>("SELECT display_name FROM users WHERE id = $1")
            .bind(sender_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();
        // The notification carries who, not what — it is stored in cleartext.
        super::notifications::create(
            pool, recipient_id, "message",
            &format!("New message from {}", sender_name),
            None,
            Some(&format!("/messages/{}", match_id)),
        ).await.ok();
    }

    Ok(MessageRow {
        id,
        match_id,
        sender_id,
        ciphertext: b64(ciphertext),
        nonce: nonce.map(b64),
        created_at: now,
    })
}

pub async fn update_status(pool: &PgPool, match_id: Uuid, status: &str) -> Result<()> {
    let resolved_at = if status == "completed" || status == "withdrawn" {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query("UPDATE matches SET status = $2, resolved_at = $3 WHERE id = $1")
        .bind(match_id)
        .bind(status)
        .bind(resolved_at)
        .execute(pool)
        .await?;

    if status == "completed" {
        sqlx::query(
            "UPDATE posts SET status = 'fulfilled', updated_at = now() WHERE id = (SELECT post_id FROM matches WHERE id = $1)"
        )
        .bind(match_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

#[derive(FromRow)]
struct MatchDetailRow {
    match_id: Uuid,
    post_id: Uuid,
    post_title: String,
    post_kind: String,
    responder_id: Uuid,
    author_id: Uuid,
    responder_name: String,
    author_name: String,
    status: String,
    created_at: DateTime<Utc>,
}
