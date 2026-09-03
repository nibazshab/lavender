use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Error, PgPool};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn setup() -> Result<PgPool, Error> {
    let url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/postgres".to_string());

    let options = PgConnectOptions::from_str(&url)?.log_statements(log::LevelFilter::Off);

    let db = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await?;

    let s = r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                content TEXT
            );
            "#;

    sqlx::query(s).execute(&db).await?;

    Ok(db)
}

async fn get() -> Result<&'static PgPool, Error> {
    DB_POOL.get_or_try_init(setup).await
}

struct Postgres;

impl crate::Database<Error> for Postgres {
    fn ping(&self) -> crate::Result<()> {
        DB_POOL.
    }

    fn read(&self, id: &str) -> impl Future<Output = Result<Option<String>, Error>> + Send {
        async move {
            let db = get().await?;
            let content = sqlx::query_scalar("SELECT content FROM notes WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
            Ok(content)
        }
    }

    fn write(&self, id: &str, content: &str) -> impl Future<Output = Result<(), Error>> + Send {
        async move {
            let db = get().await?;
            sqlx::query(
                "INSERT INTO notes (id, content) VALUES ($1, $2)
                 ON CONFLICT(id) DO UPDATE SET content = excluded.content",
            )
            .bind(id)
            .bind(content)
            .execute(db)
            .await?;
            Ok(())
        }
    }
}
