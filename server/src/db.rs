use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use std::path;
use std::str::FromStr;

use crate::mode::Note;
use crate::{data_dir, features, utils};

impl Note {
    pub async fn write(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let key = utils::hash(&self.id);

        sqlx::query(
            "
INSERT INTO notes (key, id, content) VALUES (?1, ?2, ?3)
ON CONFLICT(key) DO UPDATE SET
    content = excluded.content
",
        )
        .bind(key)
        .bind(&self.id)
        .bind(&self.content)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn read(id: String, pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let key = utils::hash(&id);

        let content = sqlx::query_scalar("SELECT content FROM notes WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();

        Ok(Note { id, content })
    }
}

pub async fn init_schemas(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut sql = String::from(
        "
CREATE TABLE IF NOT EXISTS notes (
    key INTEGER PRIMARY KEY,
    id TEXT NOT NULL,
    content TEXT NOT NULL
);",
    );

    sql.push_str(features::schemas());

    sqlx::query(sql.as_str()).execute(pool).await?;
    Ok(())
}

pub async fn pool() -> Result<SqlitePool, sqlx::Error> {
    let dir = data_dir();
    let db_url = path::Path::new(format!("sqlite:{dir}").as_str())
        .join("note.db")
        .display()
        .to_string();
    println!("Connecting to {db_url}");

    let options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true);

    SqlitePool::connect_with(options).await
}
