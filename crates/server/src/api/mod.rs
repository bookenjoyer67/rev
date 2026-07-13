mod alliances;
mod reports;
mod admin;
mod communities;
mod conversations;
mod endorsements;
mod posts;
mod health;
mod node;
mod notifications;
mod geocode;
mod link_preview;
mod search;
pub mod directory;
mod users;

use axum::Router;

use crate::AppState;
use crate::auth;

pub fn router(state: AppState) -> Router {
    let mut r = Router::new()
        .route("/geocode", axum::routing::get(geocode::geocode))
        .merge(health::router())
        .merge(node::router(state.clone()))
        .merge(conversations::router(state.clone()))
        .merge(notifications::router(state.clone()))
        .merge(admin::router(state.clone()))
        .merge(alliances::router(state.clone()))
        .merge(reports::router(state.clone()))
        .merge(search::router(state.clone()))
        .nest("/auth", auth::router(state.clone()))
        .nest("/users", users::router(state.clone()).merge(endorsements::router(state.clone())))
        .nest("/communities", communities::router(state.clone()))
        .nest("/communities/{slug}/posts", posts::router(state.clone()));

    r = r.route("/link-preview", axum::routing::get(link_preview::link_preview));

    if state.config.discovery.directory_enabled {
        r = r.merge(directory::router(state));
    }

    r
}
