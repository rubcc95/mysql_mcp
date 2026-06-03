use std::sync::Arc;

use mysql_mcp::{config::Config, db::PoolManager, mcp::tools::MysqlMcp};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let cfg = Config::from_env();

    tracing::info!(
        "Connecting to {} database(s): {:?}",
        cfg.databases.len(),
        cfg.database_names()
    );

    tracing::info!("Starting MCP server");
    // Create an instance of our counter router
    let service = MysqlMcp::new(Arc::new(PoolManager::new(&cfg).await), cfg)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
