mod auth;
mod broker;
mod db;
mod rate_limit;
mod routes;
mod state;
mod ws;

use axum::middleware::from_fn;
use routes::build_state;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::broker::Broker;
use crate::rate_limit::{rate_limit, RateLimiter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "excaildraw_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let broker = Broker::connect().await;
    let state = build_state(broker.clone()).await;

    let state_for_broker = state.clone();
    Broker::spawn_subscriber(
        broker,
        Arc::new(move |room_id, envelope| {
            let state = state_for_broker.clone();
            tokio::spawn(async move {
                state.forward_from_broker(&room_id, envelope).await;
            });
        }),
    );

    let limiter = RateLimiter::new(120, 60);
    let limiter_clone = limiter.clone();
    let app = routes::api_routes()
        .layer(from_fn(move |req, next| {
            let limiter = limiter_clone.clone();
            async move { rate_limit(limiter, req, next).await }
        }))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("excaildraw server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app).await.expect("server failed");
}
