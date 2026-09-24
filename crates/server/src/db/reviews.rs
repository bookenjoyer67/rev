//! M3 — deal reviews.
//!
//! A review is the one thing on this thread that outlives it: an offer is a step in a
//! negotiation, but a rating follows somebody around the marketplace afterwards. So the rules are
//! narrow on purpose — only a participant, only against a deal that actually completed, and only
//! once per person per deal, enforced by `UNIQUE (match_id, reviewer_id)` in the schema and
//! mapped back to a 409 here rather than escaping as a 500.
//!
//! There is no counter column anywhere. `rating_avg` and `rating_count` on a profile are computed
//! from these rows (`db::users::get_profile`), because a denormalised total is a second answer to
//! the same question that can only ever drift from the first.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use komun_core::models::MatchStatus;

use super::conversations::{lock_status, DealStep};

/// The message a duplicate review is refused with.
pub const ALREADY_REVIEWED: &str = "you have already reviewed this deal";

/// A review as it was written, returned to its author.
#[derive(Serialize, Clone, Debug, FromRow)]
pub struct ReviewRow {
    pub id: Uuid,
    pub match_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i16,
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A review as it appears on somebody's profile.
///
/// SPEC B4 decision: reviews are attributed. The reviewer's id and name travel with every row,
/// because "4 stars" from nobody in particular is a number a marketplace cannot act on — there is
/// no one to ask, and nothing to weigh it against.
#[derive(Serialize, Clone, Debug, FromRow)]
pub struct ReviewView {
    pub id: Uuid,
    pub match_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewer_display_name: String,
    pub rating: i16,
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Whether a thread in `current` may be reviewed.
///
/// Pure, so the rule can be pinned without a database; the status it is asked about is read under
/// the row lock inside [`create`], where the answer cannot change between the check and the write.
///
/// SPEC B4: "writable only against a completed deal". A proposal anyone can open, and a thread
/// somebody withdrew from, would otherwise both be a one-star review of a stranger you never
/// traded with.
pub fn check_reviewable(current: MatchStatus) -> Result<(), String> {
    if current == MatchStatus::Completed {
        return Ok(());
    }
    // The current status is named for the same reason every other 409 on this thread names it:
    // "not completed" tells the caller they were wrong, and nothing about what to do next.
    Err(format!("this deal is not completed (status: {current})"))
}

/// M3.1 — write one review, or say why not.
///
/// `reviewee_id` is supplied by the caller of this function, not by the HTTP client: the handler
/// derives it from the thread (`Thread::other_participant`).
pub async fn create(
    pool: &PgPool,
    match_id: Uuid,
    reviewer_id: Uuid,
    reviewee_id: Uuid,
    rating: i16,
    body: Option<&str>,
) -> Result<DealStep<ReviewRow>> {
    let mut tx = pool.begin().await?;

    let status = lock_status(&mut tx, match_id).await?;
    if let Err(why) = check_reviewable(status) {
        return Ok(DealStep::Conflict(why));
    }

    let inserted = sqlx::query_as::<_, ReviewRow>(
        r#"INSERT INTO deal_reviews (id, match_id, reviewer_id, reviewee_id, rating, body, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, match_id, reviewer_id, reviewee_id, rating, body, created_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(match_id)
    .bind(reviewer_id)
    .bind(reviewee_id)
    .bind(rating)
    .bind(body)
    .bind(Utc::now())
    .fetch_one(&mut *tx)
    .await;

    let row = match inserted {
        Ok(row) => row,
        // The duplicate is caught by the database, not by a SELECT first: a check-then-insert
        // has a window two concurrent requests both pass, and the constraint does not. What this
        // arm exists for is the *mapping* — an unmapped 23505 leaves here as an anyhow error and
        // is reported to the reviewer as "internal error", which reads as a server fault rather
        // than as "you already did this".
        Err(e) if is_unique_violation(&e) => {
            return Ok(DealStep::Conflict(ALREADY_REVIEWED.to_string()))
        }
        Err(e) => return Err(e.into()),
    };

    tx.commit().await?;
    Ok(DealStep::Done(row))
}

/// M3.3 — somebody's reviews, newest first.
///
/// `id` breaks the tie for the same reason `list_offers` does: `created_at` is microsecond
/// resolution, and two rows written in the same microsecond would otherwise come back in an order
/// Postgres is free to change between calls — which, under LIMIT/OFFSET, means a row appearing on
/// two pages or on none.
pub async fn list_for_user(
    pool: &PgPool,
    reviewee_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReviewView>> {
    let rows = sqlx::query_as::<_, ReviewView>(
        r#"SELECT r.id, r.match_id, r.reviewer_id,
                  u.display_name AS reviewer_display_name,
                  r.rating, r.body, r.created_at
           FROM deal_reviews r
           JOIN users u ON u.id = r.reviewer_id
           WHERE r.reviewee_id = $1
           ORDER BY r.created_at DESC, r.id DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(reviewee_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// `23505` is Postgres' unique-violation SQLSTATE. `deal_reviews` carries exactly one unique
/// constraint — `UNIQUE (match_id, reviewer_id)` — so on this table the code identifies it
/// unambiguously.
fn is_unique_violation(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(db) => db.code().is_some_and(|code| code == "23505"),
        _ => false,
    }
}
