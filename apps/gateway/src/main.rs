//! Gateway entrypoint: open the DB, migrate, compose the app, and serve it.

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let database_url = resolve_database_url();
    let db = persistence::db::connect(&database_url).await?;
    gateway::migrate(&db).await?;

    let email_sender = std::sync::Arc::new(iam::infrastructure::ResendEmailSender::from_env());
    let (app, scheduler) = gateway::build(db, email_sender).await;
    tokio::spawn(scheduler.run());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("gateway listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Resolve `DATABASE_URL`. Fail closed in production if unset (like the JWT
/// secret); fall back to a local dev DSN otherwise.
fn resolve_database_url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return url;
        }
    }

    let environment = std::env::var("APP_ENV")
        .or_else(|_| std::env::var("RUST_ENV"))
        .unwrap_or_default()
        .to_lowercase();
    if matches!(environment.as_str(), "production" | "prod") {
        panic!("DATABASE_URL must be set in production");
    }

    eprintln!(
        "WARN [gateway]: DATABASE_URL unset; using the local dev default \
         postgres://postgres:postgres@localhost:5432/postgres"
    );
    "postgres://postgres:postgres@localhost:5432/postgres".to_owned()
}
