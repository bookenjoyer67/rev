mod registration;
mod health;
mod expiry;
mod bundle_cleanup;

use std::sync::Arc;

use crate::AppState;

pub fn spawn_background_tasks(state: AppState) {
    let config = &state.config;

    if config.discovery.listed && config.discovery.directory_url.is_some() {
        let s = state.clone();
        tokio::spawn(registration::registration_loop(s));
    }

    if config.discovery.directory_enabled {
        let s = state.clone();
        tokio::spawn(health::health_check_loop(s));
    }

    {
        let s = state.clone();
        tokio::spawn(expiry::expiry_loop(s));
    }

    {
        let s = state.clone();
        tokio::spawn(bundle_cleanup::user_cleanup_loop(s));
    }

    if config.federation.enabled {
        let pool = state.pool.clone();
        crate::federation::start_sync_loop(Arc::new(pool), 300); // every 5 minutes
    }
}
