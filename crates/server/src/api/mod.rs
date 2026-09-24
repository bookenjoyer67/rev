mod reports;
mod admin;
// M1/M2: `pub(crate)` on these three only so `crate::tests::market` can unit-test their
// validators directly. Nothing outside the crate can reach them, and the routers are still
// mounted here.
pub(crate) mod categories;
pub(crate) mod conversations;
mod endorsements;
mod error;
pub(crate) mod posts;
mod health;
mod node;
mod notifications;
mod geocode;
mod link_preview;
mod search;
pub mod directory;
mod users;

// A3.2: `alliances` is no longer declared as a module. The file is still on disk because B owns
// its deletion on the other branch; leaving the `mod` out means nothing compiles it and nothing
// routes to it, so `GET /api/alliances` is a 404 here as the card requires.

use axum::Router;

use crate::AppState;
use crate::auth;

pub use error::StatusError;

pub fn router(state: AppState) -> Router {
    let mut r = Router::new()
        .route("/geocode", axum::routing::get(geocode::geocode))
        .merge(health::router())
        .merge(node::router(state.clone()))
        .merge(conversations::router(state.clone()))
        .merge(notifications::router(state.clone()))
        .merge(admin::router(state.clone()))
        // M1.2/M1.3: the public taxonomy and its admin editor. Mounted flat rather than nested
        // because the two halves live under different path prefixes (`/categories` and
        // `/admin/categories`) and behind different guards.
        .merge(categories::router(state.clone()))
        .merge(reports::router(state.clone()))
        .merge(search::router(state.clone()))
        .nest("/auth", auth::router(state.clone()))
        .nest("/users", users::router(state.clone()).merge(endorsements::router(state.clone())))
        // A3.1: posts are a flat, server-wide collection now — no tenant segment in the path.
        .nest("/posts", posts::router(state.clone()));

    r = r.route("/link-preview", axum::routing::get(link_preview::link_preview));

    if state.config.discovery.directory_enabled {
        r = r.merge(directory::router(state));
    }

    r
}
