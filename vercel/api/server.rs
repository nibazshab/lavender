use axum::body::Body;
use axum::extract::multipart::MultipartError;
use axum::extract::{Multipart, Path};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::headers::{Header, HeaderName, HeaderValue};
use axum_extra::{TypedHeader, headers};
use hyper::body::Bytes;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use rust_embed::RustEmbed;
use serde::Serialize;
use sqlx::{Decode, Sqlite, SqlitePool, Transaction, Type};
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::env;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::{fs, path};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use app::{pool, router as main_router};

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

    let router = main_router()
        .merge(file_router())
        .into_make_service_with_connect_info::<SocketAddr>();

    let attachment = ATTACHMENT_PATH.as_path();
    if !attachment.exists() {
        fs::create_dir_all(attachment)?;
    }

    let pool = pool().await;
    init_file_schema(&pool).await?;

    println!("Server running on {addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(close())
        .await?;

    pool.close().await;

    Ok(())
}

static ATTACHMENT_PATH: LazyLock<path::PathBuf> = LazyLock::new(|| {
    let dir = {
        let mut path = env::current_exe().unwrap();
        path.pop();
        path.display().to_string()
    };

    path::Path::new(&dir).join("attachment")
});

#[derive(RustEmbed)]
#[folder = "templates/file_cabinets/"]
struct FileAssets;

#[derive(Debug)]
struct File {
    id: String,
    token: String,
}

#[derive(Serialize)]
struct Link {
    url: String,
    token: String,
}

#[derive(Debug, Copy, Clone)]
enum Column {
    Name,
    Token,
    Mime,
}

#[derive(Debug)]
struct TokenHeader(String);

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

enum Error {
    Io(std::io::Error),
    Sqlx(sqlx::Error),
    BadRequest(String),
    Forbidden,
    NotFound,
    Unpredictable,
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
            Error::Unpredictable => {
                eprintln!("Unpredictable Error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
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

impl From<MultipartError> for Error {
    fn from(err: MultipartError) -> Self {
        Error::BadRequest(err.to_string())
    }
}

const EXT_MAP: &[(&str, &str)] = &[
    ("jar", "application/java-archive"),
    ("json", "application/json"),
    ("pdf", "application/pdf"),
    ("rss", "application/rss+xml"),
    ("wasm", "application/wasm"),
    ("xhtml", "application/xhtml+xml"),
    ("xhtm", "application/xhtml+xml"),
    ("xht", "application/xhtml+xml"),
    ("dtd", "application/xml-dtd"),
    ("xsl", "application/xml"),
    ("xml", "application/xml"),
    ("xslt", "application/xslt+xml"),
    ("zip", "application/zip"),
    ("flac", "audio/flac"),
    ("m4a", "audio/mp4"),
    ("mp2", "audio/mpeg"),
    ("mp3", "audio/mpeg"),
    ("mpga", "audio/mpeg"),
    ("ogg", "audio/ogg"),
    ("opus", "audio/ogg"),
    ("oga", "audio/ogg"),
    ("spx", "audio/ogg"),
    ("wav", "audio/wav"),
    ("mka", "audio/x-matroska"),
    ("m3u", "audio/x-mpegurl"),
    ("m3u8", "audio/x-mpegurl"),
    ("otf", "font/otf"),
    ("ttf", "font/ttf"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("apng", "image/apng"),
    ("avif", "image/avif"),
    ("gif", "image/gif"),
    ("jpeg", "image/jpeg"),
    ("jpe", "image/jpeg"),
    ("jpg", "image/jpeg"),
    ("jfif", "image/jpeg"),
    ("jxl", "image/jxl"),
    ("png", "image/png"),
    ("svg", "image/svg+xml"),
    ("svgz", "image/svg+xml"),
    ("webp", "image/webp"),
    ("css", "text/css"),
    ("html", "text/html"),
    ("htm", "text/html"),
    ("js", "text/javascript"),
    ("mjs", "text/javascript"),
    ("txt", "text/plain"),
    ("asc", "text/plain"),
    ("conf", "text/plain"),
    ("log", "text/plain"),
    ("mp4", "video/mp4"),
    ("m4v", "video/mp4"),
    ("mpeg", "video/mpeg"),
    ("mpe", "video/mpeg"),
    ("mpg", "video/mpeg"),
    ("qt", "video/quicktime"),
    ("mov", "video/quicktime"),
    ("webm", "video/webm"),
    ("mkv", "video/x-matroska"),
    ("avi", "video/x-msvideo"),
];

const DEFAULT_MIMETYPE: &str = "application/octet-stream";

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

async fn init_file_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    const SCHEMA: &str = r#"
       CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            name TEXT,
            token TEXT,
            mime TEXT
        );
        "#;

    sqlx::query(SCHEMA).execute(pool).await?;

    Ok(())
}

fn file_router() -> Router {
    Router::new()
        .route("/file/", get(home).post(storage))
        .route("/file/{id}", get(download).delete(remove))
}

async fn home() -> impl IntoResponse {
    let id = "index.html".to_string();
    file_assets(id)
}

async fn storage(
    TypedHeader(host): TypedHeader<headers::Host>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    // received
    let mut field = multipart
        .next_field()
        .await?
        .ok_or(Error::BadRequest("Invalid input".to_string()))?;

    // temp dir
    let _tmp = ATTACHMENT_PATH.join("_tmp");
    let tmp = _tmp.join(rand::random::<u32>().to_string());
    tokio::fs::create_dir_all(&_tmp).await?;

    // save temp/data
    let mut dest = tokio::fs::File::create(&tmp).await?;
    while let Some(chunk) = field.chunk().await? {
        dest.write_all(&chunk).await?;
    }
    dest.sync_all().await?;

    // db transaction
    let pool = pool().await;
    let mut tx = pool.begin().await?;

    // write db info
    let filename = field.file_name().unwrap_or("unknown").to_string();
    let file = File::write_in_tx(filename, &mut tx).await?;

    // final dir
    let key = hash(&file.id);
    let fin = path_by(key);
    let parent = fin.parent().ok_or(Error::Unpredictable)?;
    tokio::fs::create_dir_all(parent).await?;

    // move temp/data to final/data
    if let Err(e) = tokio::fs::rename(&tmp, &fin).await {
        eprintln!("{e}");

        // db rollback, delete temp/data
        if let Err(e) = tokio::fs::remove_file(&tmp).await {
            eprintln!("{e}");
        }
        return Err(e.into());
    }

    // db commit
    tx.commit().await?;

    println!("{} created", file.id);

    let link = Link {
        url: format!("{host}/file/{}", file.id),
        token: file.token,
    };

    Ok(Json(link))
}

async fn download(Path(id): Path<String>) -> Result<impl IntoResponse, Error> {
    if id == "script.js" || id == "style.css" || id == "yy.js" {
        return Ok(file_assets(id));
    }

    let key = hash(&id);

    // todo 2 req -> 1 req
    let filename = File::read_column::<String>(Column::Name, id.clone()).await?;
    let mime = File::read_column::<String>(Column::Mime, id).await?;

    let dest = path_by(key);
    let metadata = tokio::fs::metadata(&dest).await?;
    if !metadata.is_file() {
        return Err(Error::Unpredictable);
    }

    let obj = tokio::fs::File::open(dest).await?;
    let stream = tokio_util::io::ReaderStream::new(obj);
    let body = Body::from_stream(stream);

    let filename = escape(&filename);
    let headers = [
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        ),
        (header::CONTENT_TYPE, mime),
    ];

    Ok((headers, body).into_response())
}

async fn remove(
    Path(id): Path<String>,
    TypedHeader(input): TypedHeader<TokenHeader>,
) -> Result<impl IntoResponse, Error> {
    let key = hash(&id);
    let recorded = File::read_column::<String>(Column::Token, id.clone()).await?;

    if input != recorded {
        return Err(Error::Forbidden);
    }

    let pool = pool().await;
    let mut tx = pool.begin().await?;

    File::remove_in_tx(id.clone(), &mut tx).await?;

    let dest = path_by(key);
    tokio::fs::remove_file(&dest).await?;

    tx.commit().await?;

    println!("{id} removed");

    Ok(StatusCode::OK)
}

fn file_assets(file: String) -> Response {
    match FileAssets::get(&file) {
        Some(obj) => {
            let bytes = match obj.data {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(vec) => Bytes::from(vec),
            };

            (
                [
                    (header::CONTENT_TYPE, guess_mime(&file)),
                    (header::CACHE_CONTROL, "public, max-age=15552000"), // 60 * 60 * 24 * 30 * 6, 6 months
                ],
                bytes,
            )
                .into_response()
        }

        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

fn random_token() -> String {
    rand::random::<[u8; 8]>()
        .iter()
        .fold(String::with_capacity(16), |mut s, b| {
            let _ = write!(&mut s, "{b:02x}");
            s
        })
}

fn hash(input: &str) -> u32 {
    let mut hash: u32 = 5381; // djb2 initial value

    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }

    hash
}

fn guess_mime(filename: &str) -> &'static str {
    let ext = path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(String::new);

    for (e, mime) in EXT_MAP {
        if *e == ext {
            return mime;
        }
    }
    DEFAULT_MIMETYPE
}

fn path_by(key: u32) -> path::PathBuf {
    let with_hex = format!("{:016x}", key);
    let dir = &with_hex[0..2];
    let filename = &with_hex[2..];

    ATTACHMENT_PATH.join(dir).join(filename)
}

fn escape(str: &str) -> Cow<'_, str> {
    if !str
        .chars()
        .any(|c| matches!(c, '"' | '\\' | '/' | ':' | '|' | '<' | '>' | '?' | '*'))
    {
        return Cow::Borrowed(str);
    }

    let mut s = String::with_capacity(str.len() + 20);
    for c in str.chars() {
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

        let mime = guess_mime(&filename);

        sqlx::query("INSERT INTO files (id, name, token, mime) VALUES (?1, ?2, ?3, ?4)")
            .bind(&file.id)
            .bind(&filename)
            .bind(&file.token)
            .bind(mime)
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

    async fn read_column<T>(column: Column, id: String) -> Result<T, sqlx::Error>
    where
        T: for<'r> Decode<'r, Sqlite> + Type<Sqlite> + Send + Unpin,
    {
        let pool = pool().await;

        let query_str = match column {
            Column::Name => "SELECT name FROM files WHERE id = ?",
            Column::Token => "SELECT token FROM files WHERE id = ?",
            Column::Mime => "SELECT mime FROM files WHERE id = ?",
        };

        sqlx::query_scalar(query_str).bind(id).fetch_one(pool).await
    }
}
