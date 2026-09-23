//! The shared HTTP error type.
//!
//! A3.1: this lived in `api/communities.rs` until that file was deleted. Seven modules import it
//! (`admin`, `conversations`, `directory`, `notifications`, `posts`, `reports`, `search`), so it
//! needed a home that is not tied to a domain concept. `api/mod.rs` re-exports it, which is why
//! every call site says `use super::StatusError`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// An `anyhow::Error` carrying the status code it should be reported as.
///
/// The blanket `From` means `?` on any error works and produces a 500; anything the client is
/// allowed to see is built explicitly with [`StatusError::with_status`].
pub struct StatusError {
    status: StatusCode,
    inner: anyhow::Error,
}

impl<E: Into<anyhow::Error>> From<E> for StatusError {
    fn from(err: E) -> Self {
        StatusError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            inner: err.into(),
        }
    }
}

impl StatusError {
    pub fn with_status(status: StatusCode, message: impl std::fmt::Display) -> Self {
        StatusError {
            status,
            inner: anyhow::anyhow!("{}", message),
        }
    }
}

impl IntoResponse for StatusError {
    fn into_response(self) -> Response {
        // Only the 500s are logged: a 404 or a 400 is the client's problem, not an incident.
        if self.status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("request error: {:?}", self.inner);
        }
        (
            self.status,
            Json(serde_json::json!({"error": self.inner.to_string()})),
        )
            .into_response()
    }
}
