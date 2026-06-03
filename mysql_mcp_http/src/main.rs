use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use mysql_mcp::config::Config;
use mysql_mcp::db::PoolManager;
use mysql_mcp::mcp::tools::MysqlMcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("mysql_mcp=info")),
        )
        .init();
    
    let cfg = Config::from_env();

    tracing::info!(
        "Connecting to {} database(s): {:?}",
        cfg.databases.len(),
        cfg.database_names()
    );

    let pool_manager = Arc::new(PoolManager::new(&cfg).await);

    let mcp_pool = pool_manager.clone();
    let mcp_cfg = cfg.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(MysqlMcp::new(mcp_pool.clone(), mcp_cfg.clone())),
        Arc::new(NeverSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true),
    );

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;

    tracing::info!("MySQL MCP server listening on {addr}");

    axum::serve(
        TcpListener::bind(addr).await?,
        // Permissive CORS: MCP clients (IDEs, CLI tools, web UIs) use varied origins;
        // the server is intended to run locally or behind a firewall.
        axum::Router::new()
            .route("/health", axum::routing::get(health))
            .route("/mcp", axum::routing::any_service(mcp_service))
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http()),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.unwrap();
        tracing::info!("Shutting down...");
    })
    .await?;

    pool_manager.close_all().await;
    tracing::info!("All connections closed");
    Ok(())
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}
