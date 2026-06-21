use routecrab::{
    config::Config,
    observability::{init_tracing, update_route_gauges},
    store::Store,
};
use tokio::sync::watch;

/// Resolve when the process receives SIGTERM (unix) or Ctrl-C. Falls back to a
/// never-completing future if a handler cannot be installed, rather than
/// panicking on a non-input path.
async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    {
        let mut sigterm =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("cannot install SIGTERM handler: {e}");
                    ctrl_c.await;
                    return;
                }
            };
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await;
}

/// Resolve when the shutdown flag flips to `true` (or the sender is dropped).
async fn await_shutdown(mut rx: watch::Receiver<bool>) {
    let _ = rx.wait_for(|&v| v).await;
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();

    // Tracing must be set up before anything else logs.
    init_tracing(&cfg.log_level, &cfg.log_format);

    let store = Store::new();

    // One shutdown signal fans out to every long-lived task + both servers.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    {
        let shutdown_tx = shutdown_tx.clone();
        tokio::spawn(async move {
            shutdown_signal().await;
            tracing::info!("shutdown signal received");
            let _ = shutdown_tx.send(true);
        });
    }

    // Build the pair once — registers the global metrics recorder so all custom
    // gauges land in the same registry.
    let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
    let app = routecrab::web::router(store.clone(), cfg.clone(), prometheus_layer);

    // Dedicated metrics listener (separate port), gated by config.
    if cfg.metrics_enabled {
        let metrics_app = routecrab::web::metrics_router(metric_handle);
        let metrics_addr = format!("{}:{}", cfg.metrics_address, cfg.metrics_port);
        match tokio::net::TcpListener::bind(&metrics_addr).await {
            Ok(l) => {
                tracing::info!(address = %metrics_addr, "metrics endpoint listening");
                let rx = shutdown_rx.clone();
                tokio::spawn(async move {
                    if let Err(e) = axum::serve(l, metrics_app)
                        .with_graceful_shutdown(await_shutdown(rx))
                        .await
                    {
                        tracing::error!("metrics server error: {e}");
                    }
                });
            }
            Err(e) => tracing::error!("cannot bind metrics {metrics_addr}: {e}"),
        }
    } else {
        tracing::info!("metrics endpoint disabled (ROUTECRAB_METRICS_ENABLED=false)");
    }

    let bind_addr = format!("{}:{}", cfg.address, cfg.port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("cannot bind {bind_addr}: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        address = %bind_addr,
        title = %cfg.title,
        "routecrab starting"
    );

    // Spawn k8s watch — gracefully degrade when no cluster is available.
    // watch() already handles Client::try_default() failure internally and returns early.
    {
        let rx = shutdown_rx.clone();
        let (store, cfg) = (store.clone(), cfg.clone());
        tokio::spawn(async move {
            tokio::select! {
                _ = routecrab::k8s::watch(store, cfg) => {}
                _ = await_shutdown(rx) => {}
            }
        });
    }

    // Spawn health checker only when enabled.
    if cfg.health_enabled {
        let rx = shutdown_rx.clone();
        let (store, cfg) = (store.clone(), cfg.clone());
        tokio::spawn(async move {
            tokio::select! {
                _ = routecrab::health::run(store, cfg) => {}
                _ = await_shutdown(rx) => {}
            }
        });
    }

    // Spawn metrics-updater: subscribe to store changes and recompute gauges.
    {
        let store_m = store.clone();
        let mut rx_shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut rx = store_m.subscribe();
            // Compute initial snapshot immediately (store may already have routes if
            // watch front-ran us, though in practice it won't before the listener starts).
            update_route_gauges(&store_m.list());

            loop {
                tokio::select! {
                    _ = rx_shutdown.changed() => break,
                    recv = rx.recv() => match recv {
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
            }
        });
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(await_shutdown(shutdown_rx.clone()))
        .await
        .unwrap_or_else(|e| tracing::error!("server error: {e}"));

    // Ensure background tasks observe shutdown even if the server stopped for
    // another reason (e.g. bind/accept error path).
    let _ = shutdown_tx.send(true);

    tracing::info!("routecrab stopped");
}
