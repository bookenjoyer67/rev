//! M3 — `POST /api/matches/{match_id}/reviews` and `GET /api/users/{id}/reviews`.
//!
//! Two halves with two audiences, mounted separately for that reason:
//!
//! * writing a review is a participant's act on a deal, so it carries
//!   [`require_auth`](crate::auth::require_auth) — which is also where M3.2 lives: that middleware
//!   refuses every mutating method from an unverified account, so a review is covered by the same
//!   one rule as posting, responding and messaging rather than by a fourth copy of it here;
//! * reading somebody's reviews is public and unauthenticated, because a rating whose whole
//!   purpose is to help a stranger decide whether to meet you is no use behind a session. It is
//!   merged into the `/users` nest next to endorsements, which it sits beside on a profile.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{require_auth, AuthUser};
use crate::db::conversations::DealStep;
use crate::db::posts::{DEFAULT_LIMIT, MAX_LIMIT};
use crate::db::reviews::{ReviewRow, ReviewView};
use crate::AppState;

use super::categories::bad_request;
use super::StatusError;

/// SPEC B4: a review is a sentence or two about a meeting, not an essay. Bounded because this
/// text is server-readable and permanent — unlike the thread it describes, which is neither.
pub(crate) const MAX_BODY_CHARS: usize = 2000;

/// The star range, and the range `chk_deal_reviews_rating` enforces. Applied here first so `0`
/// and `6` are a 400 naming the field rather than a constraint violation arriving as a 500.
pub(crate) const MIN_RATING: i64 = 1;
pub(crate) const MAX_RATING: i64 = 5;

/// Writing a review: session required, and the thread decides the rest.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/matches/{match_id}/reviews", post(create_review))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

/// Reading them: public, and mounted inside the `/users` nest by `api::router`.
pub fn user_router(state: AppState) -> Router {
    Router::new()
        .route("/{id}/reviews", get(list_reviews))
        .with_state(state)
}

/// The raw review body.
///
/// `rating` arrives as a `serde_json::Value` rather than as an `i16` for the reason the post
/// filters arrive as strings: a typed field hands the rejection to serde, which answers **422**
/// with a message naming a Rust type. `4.5` and `"5"` are both things a client will send, and
/// both deserve a 400 that says what a rating is.
#[derive(Deserialize, Default)]
pub(crate) struct ReviewRequest {
    pub(crate) rating: Option<serde_json::Value>,
    pub(crate) body: Option<String>,
}

/// A review body that has passed every check that needs neither the thread nor the database.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValidReview {
    pub(crate) rating: i16,
    pub(crate) body: Option<String>,
}

async fn create_review(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(match_id): Path<Uuid>,
    Json(input): Json<ReviewRequest>,
) -> Result<(StatusCode, Json<ReviewRow>), StatusError> {
    // The 404/403 pair, and the reviewee, out of one load: an id nobody has is not found, an id
    // that exists but is not yours is forbidden, and the person you are reviewing is whoever is
    // on the other side of it. M3.1 — the reviewee is never a field the client supplies, or a
    // completed deal would be a licence to rate a stranger.
    let thread = crate::db::conversations::load_thread(&state.pool, match_id)
        .await?
        .ok_or_else(|| StatusError::with_status(StatusCode::NOT_FOUND, "conversation not found"))?;

    let reviewee_id = thread
        .other_participant(auth.user_id)
        .ok_or_else(|| StatusError::with_status(StatusCode::FORBIDDEN, "not a participant"))?;

    let review = validate_review(&input).map_err(bad_request)?;

    // Whether the deal is completed is decided inside the transaction, with the row locked —
    // asking out here as well would be a second answer that can already be stale by the time the
    // insert runs.
    match crate::db::reviews::create(
        &state.pool,
        match_id,
        auth.user_id,
        reviewee_id,
        review.rating,
        review.body.as_deref(),
    )
    .await?
    {
        DealStep::Done(row) => Ok((StatusCode::CREATED, Json(row))),
        DealStep::Conflict(why) => Err(StatusError::with_status(StatusCode::CONFLICT, why)),
    }
}

/// The raw pagination query. Strings for the same reason `PostFilters` uses them: `?limit=all`
/// should be a 400 that names the parameter, not a 422 naming `i64`.
#[derive(Deserialize, Default)]
pub(crate) struct ReviewPage {
    pub(crate) limit: Option<String>,
    pub(crate) offset: Option<String>,
}

async fn list_reviews(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(page): Query<ReviewPage>,
) -> Result<Json<Vec<ReviewView>>, StatusError> {
    let (limit, offset) = validate_page(&page).map_err(bad_request)?;
    let reviews = crate::db::reviews::list_for_user(&state.pool, user_id, limit, offset).await?;
    Ok(Json(reviews))
}

/// Every check on a review body that needs neither the thread nor the database.
///
/// Pure and `pub(crate)` so `tests::market` can pin every branch without a database.
pub(crate) fn validate_review(raw: &ReviewRequest) -> Result<ValidReview, String> {
    let rating = match raw.rating.as_ref() {
        None | Some(serde_json::Value::Null) => {
            return Err(format!(
                "rating is required and must be a whole number from {MIN_RATING} to {MAX_RATING}"
            ))
        }
        Some(value) => match value.as_i64() {
            Some(stars) if (MIN_RATING..=MAX_RATING).contains(&stars) => stars as i16,
            // In range but not an integer, or an integer outside it: both are answered with the
            // range, because that is the fact the client is missing in either case.
            Some(stars) => {
                return Err(format!(
                    "rating must be between {MIN_RATING} and {MAX_RATING} (got {stars})"
                ))
            }
            None => {
                return Err(format!(
                    "rating must be a whole number from {MIN_RATING} to {MAX_RATING} (got {value})"
                ))
            }
        },
    };

    let body = match trimmed(raw.body.as_deref()) {
        None => None,
        Some(body) => {
            // Characters, not bytes — the same rule an offer note counts by, and for the same
            // reason: a review written in a non-Latin script would otherwise be cut to a third of
            // the length the message promises.
            let length = body.chars().count();
            if length > MAX_BODY_CHARS {
                return Err(format!(
                    "body must be {MAX_BODY_CHARS} characters or fewer (got {length})"
                ));
            }
            Some(body.to_string())
        }
    };

    Ok(ValidReview { rating, body })
}

/// M3.3 — the same limit/offset convention `GET /api/posts` uses, over the same two constants,
/// so a client that has learned one list endpoint has learned this one.
///
/// Rejected rather than clamped: a caller that asks for 5,000 and is quietly handed 200 has no
/// way to know its pagination is wrong.
pub(crate) fn validate_page(raw: &ReviewPage) -> Result<(i64, i64), String> {
    let limit = bounded("limit", raw.limit.as_deref(), DEFAULT_LIMIT, 1, MAX_LIMIT)?;
    let offset = bounded("offset", raw.offset.as_deref(), 0, 0, i64::MAX)?;
    Ok((limit, offset))
}

fn bounded(name: &str, raw: Option<&str>, default: i64, min: i64, max: i64) -> Result<i64, String> {
    let Some(value) = trimmed(raw) else {
        return Ok(default);
    };

    let parsed: i64 = value
        .parse()
        .map_err(|_| format!("{name} must be a whole number (got {value:?})"))?;
    if parsed < min || parsed > max {
        return Err(format!(
            "{name} must be between {min} and {max} (got {parsed})"
        ));
    }
    Ok(parsed)
}

/// An absent field and a blank one mean the same thing: not supplied.
fn trimmed(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|value| !value.is_empty())
}
