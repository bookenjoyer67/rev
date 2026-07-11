use axum::{extract::State, routing::get, Json, Router};

use komun_core::models::Alliance;
use crate::AppState;
use super::communities::StatusError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/alliances", get(list_alliances))
        .with_state(state)
}

async fn list_alliances(
    State(state): State<AppState>,
) -> Result<Json<Vec<Alliance>>, StatusError> {
    let alliances = crate::db::alliances::list_alliances(&state.pool).await?;
    Ok(Json(alliances))
}
