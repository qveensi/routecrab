use routecrab::{
    config::Config,
    observability::{init_tracing, update_route_gauges},
    store::Store,
};

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();

    // Tracing must be set up before anything else logs.
    init_tracing(&cfg.log_level, &cfg.log_format);

    let store = Store::new();

    // Build the router first — this registers the global metrics recorder
    // (PrometheusMetricLayer::pair) so our custom gauges land in the same registry.
    let app = routecrab::web::router(store.clone(), cfg.clone());

    let bind_addr = format!("{}:{}", cfg.address, cfg.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {bind_addr}: {e}"));

    tracing::info!(
        address = %bind_addr,
        title = %cfg.title,
        "routecrab starting"
    );

    // Spawn k8s watch — gracefully degrade when no cluster is available.
    // watch() already handles Client::try_default() failure internally and returns early.
    tokio::spawn(routecrab::k8s::watch(store.clone(), cfg.clone()));

    // Spawn health checker only when enabled.
    if cfg.health_enabled {
        tokio::spawn(routecrab::health::run(store.clone(), cfg.clone()));
    }

    // Spawn metrics-updater: subscribe to store changes and recompute gauges.
    {
        let store_m = store.clone();
        tokio::spawn(async move {
            let mut rx = store_m.subscribe();
            // Compute initial snapshot immediately (store may already have routes if
            // watch front-ran us, though in practice it won't before the listener starts).
            update_route_gauges(&store_m.list());

            loop {
                match rx.recv().await {
                    Ok(_change) => {
                        // Recompute from full snapshot rather than tracking deltas —
                        // simpler and correct under concurrent upserts.
                        update_route_gauges(&store_m.list());
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // We fell behind; recompute from current snapshot to self-heal.
                        tracing::warn!(skipped = n, "metrics subscriber lagged; resyncing gauges");
                        update_route_gauges(&store_m.list());
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    // Graceful shutdown: wait for SIGTERM (unix) or Ctrl-C.
    let shutdown = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl-C handler");
        };

        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM handler");

            tokio::select! {
                _ = ctrl_c => {},
                _ = sigterm.recv() => {},
            }
        }

        #[cfg(not(unix))]
        ctrl_c.await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .unwrap_or_else(|e| tracing::error!("server error: {e}"));

    tracing::info!("routecrab stopped");
}
