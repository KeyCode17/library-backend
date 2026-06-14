//! Gateway entrypoint: bind a TCP listener and serve the composed router.

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("gateway listening on {addr}");

    // Compose the app and run the notification due-date scheduler alongside it.
    let (app, scheduler) = gateway::build();
    tokio::spawn(scheduler.run());

    axum::serve(listener, app).await?;
    Ok(())
}
