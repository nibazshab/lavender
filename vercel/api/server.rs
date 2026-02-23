use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_extra::headers::{Header, HeaderName, HeaderValue};
use axum_extra::{TypedHeader, headers};
use hyper::body::Bytes;
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use serde_json::json;
use sqlx::{Decode, Sqlite, SqlitePool, Transaction, Type};
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::env;
use std::fmt::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, path};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use app::{pool, router};

pub async fn app() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).peekable();
    let mut port: Option<u16> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" | "-p" => {
                port = args.next().map(|p| p.parse::<u16>()).transpose()?;
            }
            "--help" | "-h" => {
                println!(
                    "usage: {} [options]\n",
                    env::args().next().unwrap_or("app".to_string())
                );
                println!("options:");
                println!("  -h, --help");
                println!("  -p, --port <PORT>");
                return Ok(());
            }
            _ => {
                return Err(format!("unknown argument: {arg}").into());
            }
        }
    }

    let port = port.unwrap_or_else(|| {
        env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080)
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = TcpListener::bind(addr).await?;
    let router = router().into_make_service_with_connect_info::<SocketAddr>();

    println!("Server running on {addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(close())
        .await?;

    let pool = pool().await;
    pool.close().await;

    Ok(())
}

async fn close() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.unwrap();
    };

    #[cfg(unix)]
    let terminate = {
        async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .unwrap()
                .recv()
                .await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[derive(rust_embed::RustEmbed)]
#[folder = "templates/file_cabinets/"]
struct FileAssets;

#[derive(Debug)]
struct File {
    id: String,
    token: String,
}

#[derive(Debug, Copy, Clone)]
enum Column {
    Name,
    Token,
}

#[derive(Debug)]
struct TokenHeader(String);

enum Error {
    Io(std::io::Error),
    Sqlx(sqlx::Error),
    BadRequest(String),
    Forbidden,
    NotFound,
}

static ATTACHMENT_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path.display().to_string()
    };

    path::Path::new(&dir).join("attachment")
});

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

fn hash(input: &str) -> u32 {
    let mut hash: u32 = 5381; // djb2 initial value

    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }

    hash
}

impl File {
    async fn write_in_tx(
        filename: String,
        tx: &mut Transaction<'_, Sqlite>,
    ) -> Result<Self, sqlx::Error> {
        let id = rand_string(6);
        let file = File {
            id,
            token: random_token(),
        };

        sqlx::query("INSERT INTO files (id, name, token) VALUES (?1, ?2, ?3)")
            .bind(&file.id)
            .bind(&filename)
            .bind(&file.token)
            .execute(&mut **tx)
            .await?;

        Ok(file)
    }

    async fn remove_in_tx(id: String, tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    async fn read_column<T>(column: Column, id: String, pool: &SqlitePool) -> Result<T, sqlx::Error>
    where
        T: for<'r> Decode<'r, Sqlite> + Type<Sqlite> + Send + Unpin,
    {
        let query_str = match column {
            Column::Name => "SELECT name FROM files WHERE id = ?",
            Column::Token => "SELECT token FROM files WHERE id = ?",
        };

        sqlx::query_scalar(query_str).bind(id).fetch_one(pool).await
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
                eprintln!("{e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            Error::Sqlx(e) => {
                eprintln!("{e}");
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

fn file_router() -> axum::Router<SqlitePool> {
    axum::Router::new()
        .route("/b/", get(index_page).post(upload))
        .route("/b/{id}", get(download).delete(remove))
}

fn init_os_dir() -> std::io::Result<()> {
    let attachment = ATTACHMENT_PATH.clone();

    if !attachment.exists() {
        fs::create_dir_all(attachment)?;
    }

    Ok(())
}

fn schema() -> &'static str {
    "
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    name TEXT,
    token TEXT
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

    let key = hash(&file.id);
    let final_path = storage(key).await?;

    if let Err(e) = tokio::fs::rename(&tmp_path, &final_path).await {
        eprintln!("{e}");
        if let Err(e) = tokio::fs::remove_file(&tmp_path).await {
            eprintln!("{e}");
        }
        return Err(e.into());
    }

    tx.commit().await?;

    println!("{} created", file.id);
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

    let key = hash(&id);
    let filename = File::read_column::<String>(Column::Name, id, &pool).await?;

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
    let key = hash(&id);
    let token_recorded = File::read_column::<String>(Column::Token, id.clone(), &pool).await?;
    if token_input != token_recorded {
        return Err(Error::Forbidden);
    }

    let mut tx = pool.begin().await?;

    File::remove_in_tx(id.clone(), &mut tx).await?;

    let dest = storage(key).await?;
    tokio::fs::remove_file(&dest).await?;

    tx.commit().await?;

    println!("{id} removed");
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

fn release_assets(filename: &str, data: Cow<'static, [u8]>) -> Response {
    let content_type = if filename.ends_with(".js") {
        "text/javascript"
    } else if filename.ends_with(".css") {
        "text/css"
    } else {
        "application/octet-stream"
    };

    let bytes = match data {
        Cow::Borrowed(slice) => Bytes::from_static(slice),
        Cow::Owned(vec) => Bytes::from(vec),
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=15552000"), // 60 * 60 * 24 * 30 * 6, 6 months
        ],
        bytes,
    )
        .into_response()
}

fn random_token() -> String {
    rand::random::<[u8; 8]>()
        .iter()
        .fold(String::with_capacity(16), |mut s, b| {
            let _ = write!(&mut s, "{b:02x}");
            s
        })
}

async fn storage(key: u32) -> Result<PathBuf, std::io::Error> {
    // todo fix, key from i64 -> u32, is it work well?
    let with_hex = format!("{:016x}", key);
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
