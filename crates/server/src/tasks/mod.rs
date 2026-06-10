mod registration;
mod health;

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
}
