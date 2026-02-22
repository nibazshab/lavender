use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::headers::{Header, HeaderName, HeaderValue};
use axum_extra::{TypedHeader, headers};
use serde_json::json;
use sqlx::{Decode, Sqlite, SqlitePool, Transaction, Type};
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, path};
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use crate::core::release_assets;
use crate::{data_dir, utils};

#[derive(rust_embed::RustEmbed)]
#[folder = "templates/features/file/"]
struct FileAssets;

#[derive(Debug)]
struct File {
    key: i64,
    id: String,
    token: String,
}

#[derive(Debug, Copy, Clone)]
enum Column {
    Name,
    Token,
}

#[derive(Debug)]
pub struct TokenHeader(String);

pub enum Error {
    Io(std::io::Error),
    Sqlx(sqlx::Error),
    BadRequest(String),
    Forbidden,
    NotFound,
}

static ATTACHMENT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| path::Path::new(&data_dir()).join("attachment"));

impl File {
    async fn write_in_tx(
        filename: String,
        tx: &mut Transaction<'_, Sqlite>,
    ) -> Result<Self, sqlx::Error> {
        let id = utils::rand_string(6);
        let file = File {
            key: utils::hash(&id),
            id,
            token: random_token(),
        };

        sqlx::query("INSERT INTO files (key, id, name, token) VALUES (?1, ?2, ?3, ?4)")
            .bind(file.key)
            .bind(&file.id)
            .bind(&filename)
            .bind(&file.token)
            .execute(&mut **tx)
            .await?;

        Ok(file)
    }

    async fn remove_in_tx(key: i64, tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE key = ?")
            .bind(key)
            .execute(&mut **tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    async fn read_column<T>(column: Column, key: i64, pool: &SqlitePool) -> Result<T, sqlx::Error>
    where
        T: for<'r> Decode<'r, Sqlite> + Type<Sqlite> + Send + Unpin,
    {
        let query_str = match column {
            Column::Name => "SELECT name FROM files WHERE key = ?",
            Column::Token => "SELECT token FROM files WHERE key = ?",
        };

        sqlx::query_scalar(query_str)
            .bind(key)
            .fetch_one(pool)
            .await
    }
}

impl Header for TokenHeader {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("token");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let val = values.next().ok_or_else(headers::Error::invalid)?;
        let val_str = val.to_str().map_err(|_| headers::Error::invalid())?;
        Ok(TokenHeader(val_str.to_owned()))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        if let Ok(val) = HeaderValue::from_str(&self.0) {
            values.extend(std::iter::once(val));
        }
    }
}

impl PartialEq<String> for TokenHeader {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::Io(e) => {
                error!("{e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            Error::Sqlx(e) => {
                error!("{e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            Error::NotFound => (StatusCode::NOT_FOUND, "Not Found".to_string()),
        };

        (status, message).into_response()
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Sqlx(err),
        }
    }
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/b/", get(index_page).post(upload))
        .route("/b/{id}", get(download).delete(remove))
}

pub fn init_os_dir() -> std::io::Result<()> {
    let attachment = ATTACHMENT_PATH.clone();

    if !attachment.exists() {
        fs::create_dir_all(attachment)?;
    }

    Ok(())
}

pub fn schema() -> &'static str {
    "
CREATE TABLE IF NOT EXISTS files (
    key INTEGER PRIMARY KEY,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    token TEXT NOT NULL
);"
}

async fn upload(
    State(pool): State<SqlitePool>,
    TypedHeader(host): TypedHeader<headers::Host>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
        .ok_or_else(|| Error::BadRequest("Expecting a file.".to_string()))?;

    let tmp_dir = ATTACHMENT_PATH.join("_tmp");
    tokio::fs::create_dir_all(&tmp_dir).await?;
    let tmp_path = tmp_dir.join(temp_filename());

    let mut dest = tokio::fs::File::create(&tmp_path).await?;
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        dest.write_all(&chunk).await?;
    }
    dest.sync_all().await?;

    let mut tx = pool.begin().await?;

    let filename = field.file_name().unwrap_or("unknown").to_string();
    let file = File::write_in_tx(filename, &mut tx).await?;
    let final_path = storage(file.key).await?;

    if let Err(e) = tokio::fs::rename(&tmp_path, &final_path).await {
        error!("{e}");
        if let Err(e) = tokio::fs::remove_file(&tmp_path).await {
            error!("{e}");
        }
        return Err(e.into());
    }

    tx.commit().await?;

    info!("{} created", file.id);
    Ok(Json(json!({
        "link": format!("{host}/b/{}", file.id),
        "token": file.token,
    })))
}

async fn download(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, Error> {
    if id == "script.js" || id == "style.css" || id == "yy.js" {
        return Ok(page(Path(id)));
    }

    let key = utils::hash(&id);
    let filename = File::read_column::<String>(Column::Name, key, &pool).await?;

    let dest = storage(key).await?;

    let metadata = tokio::fs::metadata(&dest).await?;
    if !metadata.is_file() {
        return Err(Error::NotFound);
    }

    let f = tokio::fs::File::open(dest).await?;
    let stream = tokio_util::io::ReaderStream::new(f);
    let body = Body::from_stream(stream);

    let safe_name = safe_filename(&filename);

    let headers = [(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{safe_name}\""),
    )];

    Ok((headers, body).into_response())
}

async fn remove(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
    TypedHeader(token_input): TypedHeader<TokenHeader>,
) -> Result<impl IntoResponse, Error> {
    let key = utils::hash(&id);
    let token_recorded = File::read_column::<String>(Column::Token, key, &pool).await?;
    if token_input != token_recorded {
        return Err(Error::Forbidden);
    }

    let mut tx = pool.begin().await?;

    File::remove_in_tx(key, &mut tx).await?;

    let dest = storage(key).await?;
    tokio::fs::remove_file(&dest).await?;

    tx.commit().await?;

    info!("{id} removed");
    Ok(StatusCode::OK)
}

async fn index_page() -> impl IntoResponse {
    let id = "index.html".to_string();
    page(Path(id))
}

fn page(Path(id): Path<String>) -> Response {
    match FileAssets::get(&id) {
        Some(obj) => release_assets(&id, obj.data),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn random_token() -> String {
    rand::random::<[u8; 8]>()
        .iter()
        .fold(String::with_capacity(16), |mut s, b| {
            let _ = write!(&mut s, "{b:02x}");
            s
        })
}

async fn storage(key: i64) -> Result<PathBuf, std::io::Error> {
    let with_hex = format!("{:016x}", key as u64);
    let dir = &with_hex[0..2];
    let filename = &with_hex[2..];

    let dir_path = ATTACHMENT_PATH.join(dir);
    tokio::fs::create_dir_all(&dir_path).await?;

    Ok(dir_path.join(filename))
}

fn temp_filename() -> String {
    rand::random::<u32>().to_string()
}

fn safe_filename(name: &str) -> Cow<'_, str> {
    if !name
        .chars()
        .any(|c| matches!(c, '"' | '\\' | '/' | ':' | '|' | '<' | '>' | '?' | '*'))
    {
        return Cow::Borrowed(name);
    }

    let mut s = String::with_capacity(name.len() + 20);
    for c in name.chars() {
        match c {
            '"' => s.push_str("%22"),
            '\\' => s.push_str("%5C"),
            '/' => s.push_str("%2F"),
            ':' => s.push_str("%3A"),
            '|' => s.push_str("%7C"),
            '<' => s.push_str("%3C"),
            '>' => s.push_str("%3E"),
            '?' => s.push_str("%3F"),
            '*' => s.push_str("%2A"),
            _ => s.push(c),
        }
    }
    Cow::Owned(s)
}
