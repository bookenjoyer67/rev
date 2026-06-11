use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use socket2::{Domain, Socket, TcpKeepalive, Type};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::config::RelayConfig;
use komun_relay::storage::PersistentStore;

pub async fn spawn_relay(relay_config: RelayConfig, store: Arc<PersistentStore>) {
    let mut config = komun_relay::config::Config::default();
    config.server.port = relay_config.port;
    config.server.bind_address = relay_config.bind_address.clone();
    config.security.require_passwords = false;
    if relay_config.max_clients_per_room > 0 {
        config.rooms.max_clients = relay_config.max_clients_per_room;
    }

    let max_conn = config.server.max_connections.max(1);

    let state = Arc::new(komun_relay::state::AppState {
        rooms: DashMap::new(),
        shares: Mutex::new(komun_relay::share::ShareStore::new(
            config.share.max_shares,
            config.share.share_ttl_secs,
        )),
        rl: Mutex::new(komun_relay::rate::RateLimiter::new(config.rate_limit.clone())),
        config: config.clone(),
        store: store.clone(),
        #[cfg(feature = "mqtt-bridge")]
        mesh_uplink: tokio::sync::RwLock::new(None),
        #[cfg(feature = "reticulum-bridge")]
        reticulum_inject: tokio::sync::RwLock::new(None),
        #[cfg(feature = "mqtt-bridge")]
        mqtt_client: tokio::sync::RwLock::new(None),
        #[cfg(feature = "peer-relay")]
        peer_relay_txs: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        conn_semaphore: Arc::new(tokio::sync::Semaphore::new(max_conn)),
        start_time: Instant::now(),
        connections_accepted: AtomicU64::new(0),
        connections_rejected: AtomicU64::new(0),
        last_snapshot_time: std::sync::RwLock::new(None),
    });

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await;
            cleanup_state.store.flush_if_dirty().await;
            cleanup_state.store.cleanup_expired_ttls().await;
        }
    });

    let bind_addr = format!("{}:{}", relay_config.bind_address, relay_config.port);

    let addr: std::net::SocketAddr = match bind_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            error!("relay: invalid bind address '{}': {}", bind_addr, e);
            return;
        }
    };

    let listener = match bind_with_keepalive(addr) {
        Ok(l) => l,
        Err(e) => {
            error!("relay: failed to bind on {}: {}", bind_addr, e);
            return;
        }
    };

    info!("relay listening on {}", bind_addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let permit = match state.conn_semaphore.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        warn!("relay: max connections reached, rejecting {}", addr);
                        state.connections_rejected.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        continue;
                    }
                };
                state.connections_accepted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let s = state.clone();
                tokio::spawn(async move {
                    komun_relay::handler::handle(s, stream, addr).await;
                    drop(permit);
                });
            }
            Err(e) => {
                warn!("relay: accept error: {}", e);
            }
        }
    }
}

fn bind_with_keepalive(addr: std::net::SocketAddr) -> std::io::Result<TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    let ka = TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(10));
    socket.set_tcp_keepalive(&ka).ok();
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    socket.set_nonblocking(true)?;
    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}
