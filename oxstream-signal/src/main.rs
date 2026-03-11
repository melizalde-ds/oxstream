use anyhow::Result;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new().route("/", get(method_router));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn method_router() -> &'static str {
    "Hello, world!"
}
