//! `/api/categories` — the taxonomy, served as rows rather than compiled in as an enum.
//!
//! Two halves with two different audiences:
//!
//! * `GET /api/categories?scope=` is public and unauthenticated. The aid form and the market form
//!   both read it at load, so it has to answer without a session.
//! * `POST /api/admin/categories` and `PATCH /api/admin/categories/{slug}` are the runtime editor
//!   (plan decision B8), open to **admin or superadmin**.
//!
//! The admin routes live here rather than in `api/admin.rs` because that module layers
//! [`require_superadmin`](crate::auth::require_superadmin) over everything it holds, and curating
//! a category list is not in the same class as granting somebody admin.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use komun_core::models::{Category, CategoryScope, CreateCategory, UpdateCategory};

use crate::auth::{record_audit, require_admin, AuthUser};
use crate::AppState;

use super::StatusError;

/// Audit actions, namespaced the same way `admin.role_change` is so one prefix filter finds every
/// administrative act.
pub(crate) const AUDIT_CREATE: &str = "admin.category_create";
pub(crate) const AUDIT_UPDATE: &str = "admin.category_update";

pub fn router(state: AppState) -> Router {
    let public = Router::new().route("/categories", get(list_categories));

    let admin = Router::new()
        .route(
            "/admin/categories",
            get(admin_list_categories).post(create_category),
        )
        .route("/admin/categories/{slug}", patch(update_category))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    public.merge(admin).with_state(state)
}

#[derive(Deserialize)]
struct ScopeQuery {
    scope: Option<String>,
}

/// What a caller without a session sees: the three fields a form needs.
///
/// `sort_order` is an ordering mechanism rather than information — the rows arrive in that order
/// already — and `active` would always be `true`, because an inactive row never reaches this
/// serializer at all.
#[derive(Serialize)]
struct CategoryView {
    slug: String,
    label: String,
    scope: CategoryScope,
}

impl From<&Category> for CategoryView {
    fn from(c: &Category) -> Self {
        CategoryView {
            slug: c.slug.clone(),
            label: c.label.clone(),
            scope: c.scope,
        }
    }
}

/// `GET /api/categories?scope=aid|market|both` — public, active rows only.
async fn list_categories(
    State(state): State<AppState>,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<Vec<CategoryView>>, StatusError> {
    let scope = parse_scope(query.scope.as_deref()).map_err(bad_request)?;
    let rows = crate::db::categories::list(&state.pool, scope, false).await?;
    Ok(Json(rows.iter().map(CategoryView::from).collect()))
}

/// The admin view: retired categories included, and every column the editor needs to show them.
async fn admin_list_categories(
    State(state): State<AppState>,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<Vec<Category>>, StatusError> {
    let scope = parse_scope(query.scope.as_deref()).map_err(bad_request)?;
    let rows = crate::db::categories::list(&state.pool, scope, true).await?;
    Ok(Json(rows))
}

async fn create_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateCategory>,
) -> Result<(StatusCode, Json<Category>), StatusError> {
    validate_slug(&input.slug).map_err(bad_request)?;
    validate_label(&input.label).map_err(bad_request)?;

    let created = crate::db::categories::create(&state.pool, &input)
        .await?
        .ok_or_else(|| {
            StatusError::with_status(
                StatusCode::CONFLICT,
                format!("category {:?} already exists", input.slug),
            )
        })?;

    record_audit(
        &state.pool,
        Some(auth.user_id),
        AUDIT_CREATE,
        // `audit_events.subject_id` is a UUID column and a category is keyed by its TEXT slug, so
        // the subject travels in `detail`. See [`audit_detail_create`].
        None,
        audit_detail_create(&created),
    )
    .await;

    Ok((StatusCode::CREATED, Json(created)))
}

async fn update_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    Json(input): Json<UpdateCategory>,
) -> Result<Json<Category>, StatusError> {
    if let Some(label) = &input.label {
        validate_label(label).map_err(bad_request)?;
    }
    // An all-empty body would otherwise be a successful no-op that writes an audit row saying
    // nothing changed, which is indistinguishable from a client bug that dropped its payload.
    if input.label.is_none()
        && input.scope.is_none()
        && input.sort_order.is_none()
        && input.active.is_none()
    {
        return Err(bad_request(
            "nothing to update: send at least one of label, scope, sort_order, active",
        ));
    }

    let before = crate::db::categories::get(&state.pool, &slug)
        .await?
        .ok_or_else(|| not_found(&slug))?;
    let after = crate::db::categories::update(&state.pool, &slug, &input)
        .await?
        .ok_or_else(|| not_found(&slug))?;

    // SPEC 1.6: the FTS trigger indexes the label, so a rename is a two-part operation — without
    // this, every post in the category stays searchable only under the word it used to have.
    let reindexed = if after.label != before.label {
        crate::db::categories::refresh_search_vectors(&state.pool, &slug).await?
    } else {
        0
    };

    record_audit(
        &state.pool,
        Some(auth.user_id),
        AUDIT_UPDATE,
        None,
        audit_detail_update(&slug, &before, &after, reindexed),
    )
    .await;

    Ok(Json(after))
}

/// The `detail` payload for a creation.
///
/// Pulled out of the handler because `audit_events.subject_id` cannot hold a category's identity:
/// it is a `UUID` column and a category is keyed by a TEXT slug. A function is what guarantees
/// the slug is present on every category audit row, since the column that would normally carry it
/// is always NULL for these two actions.
pub(crate) fn audit_detail_create(created: &Category) -> serde_json::Value {
    serde_json::json!({
        "slug": created.slug,
        "label": created.label,
        "scope": created.scope.as_str(),
        "sort_order": created.sort_order,
        "active": created.active,
    })
}

/// The `detail` payload for an edit: before and after, so the change can be reconstructed rather
/// than merely noticed, plus how many posts the relabel forced back through the FTS trigger.
pub(crate) fn audit_detail_update(
    slug: &str,
    before: &Category,
    after: &Category,
    posts_reindexed: u64,
) -> serde_json::Value {
    serde_json::json!({
        "slug": slug,
        "from": {
            "label": before.label,
            "scope": before.scope.as_str(),
            "sort_order": before.sort_order,
            "active": before.active,
        },
        "to": {
            "label": after.label,
            "scope": after.scope.as_str(),
            "sort_order": after.sort_order,
            "active": after.active,
        },
        "posts_reindexed": posts_reindexed,
    })
}

/// Parse `?scope=`. `None` means "every active category", which is the unfiltered list both forms
/// fall back to.
///
/// An unrecognised value is an error rather than an empty result: `?scope=markets` answering
/// `200 []` is indistinguishable to the caller from a server with no categories configured, and
/// the caller would have no way to find the typo.
pub(crate) fn parse_scope(raw: Option<&str>) -> Result<Option<CategoryScope>, String> {
    let Some(value) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    match CategoryScope::parse(value) {
        Some(scope) => Ok(Some(scope)),
        None => Err(format!(
            "scope must be one of {} (got {value:?})",
            accepted_scopes()
        )),
    }
}

/// The accepted `?scope=` values, rendered from the enum so the error message cannot fall behind
/// `chk_categories_scope`.
pub(crate) fn accepted_scopes() -> String {
    CategoryScope::ALL
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Slugs are lowercase-kebab and immutable after creation.
///
/// Immutable because `posts.category` is a foreign key onto this column: the slug is an
/// identifier other rows depend on, not a display string. Constrained to lowercase-kebab because
/// it appears in query strings and in `?category=` filters, where a slug with a space or an
/// uppercase letter is a source of silent mismatches.
pub(crate) fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() {
        return Err("slug must not be empty".to_string());
    }
    if slug.len() > 64 {
        return Err(format!("slug {slug:?} is longer than 64 characters"));
    }
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(format!("slug {slug:?} must not start or end with a dash"));
    }
    if slug.contains("--") {
        return Err(format!("slug {slug:?} must not contain a double dash"));
    }
    if !slug
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(format!(
            "slug {slug:?} must be lowercase-kebab: a-z, 0-9 and single dashes only"
        ));
    }
    Ok(())
}

/// The label is what a human reads and what full-text search indexes, so a blank one makes the
/// category unusable in both places.
pub(crate) fn validate_label(label: &str) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err("label must not be empty".to_string());
    }
    if label.chars().count() > 80 {
        return Err("label must be 80 characters or fewer".to_string());
    }
    Ok(())
}

pub(crate) fn bad_request(message: impl std::fmt::Display) -> StatusError {
    StatusError::with_status(StatusCode::BAD_REQUEST, message)
}

fn not_found(slug: &str) -> StatusError {
    StatusError::with_status(StatusCode::NOT_FOUND, format!("no category {slug:?}"))
}
