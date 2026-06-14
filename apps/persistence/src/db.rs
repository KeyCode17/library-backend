//! Postgres connection pool.

use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

/// Open a pooled connection to `database_url` (a Postgres DSN).
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(database_url.to_owned());
    options
        .max_connections(16)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(8))
        .sqlx_logging(false);
    Database::connect(options).await
}
