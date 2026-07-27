use std::env;
use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn setup() -> Result<PgPool, sqlx::Error> {
    let url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/postgres".to_string());

    let options = PgConnectOptions::from_str(&url)?.log_statements(log::LevelFilter::Off);

    PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
}

pub async fn get() -> Result<&'static PgPool, sqlx::Error> {
    DB_POOL.get_or_try_init(setup).await
}
