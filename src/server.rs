use axum::body::{Body, Bytes};
use axum::extract::multipart::MultipartError;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Request};
use axum::http::{StatusCode, Uri, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, middleware};
use axum_extra::TypedHeader;
use axum_extra::headers::{self, Header, HeaderName, HeaderValue};
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use rust_embed::RustEmbed;
use serde::Serialize;
use sqlx::{Decode, Sqlite, SqlitePool, Transaction, Type};
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::{env, fs, path};
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::pool;
use crate::router as main_router;

pub async fn app() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).peekable();
    let mut port: Option<u16> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" | "-p" => {
                port = args.next().map(|p| p.parse::<u16>()).transpose()?;
            }
            "--help" | "-h" => {
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
        .layer(middleware::from_fn(log_middleware))
        .into_make_service_with_connect_info::<SocketAddr>();

    let attachment = ATTACHMENT_PATH.as_path();
    if !attachment.exists() {
        fs::create_dir_all(attachment)?;
    }

    let pool = pool().await;
    init_file_schema(pool).await?;

    println!("app running on {addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(close())
        .await?;

    pool.close().await;

    Ok(())
}

static BASE_URL: LazyLock<Option<String>> = LazyLock::new(|| env::var("BASE_URL").ok());

static ATTACHMENT_PATH: LazyLock<path::PathBuf> = LazyLock::new(|| {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.push("attachment");
    path
});

#[derive(RustEmbed)]
#[folder = "templates/file_cabinets/"]
struct FileAssets;

#[derive(Debug)]
struct File {
    id: String,
    token: String,
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

#[derive(Serialize)]
struct Link {
    url: String,
    token: String,
}

#[derive(Debug, Clone, Copy)]
enum Column {
    Token,
}

#[derive(Debug, Clone, Copy)]
enum MultiColum {
    NameMime,
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

macro_rules! mime_table {
    ($($mime:expr => $ext:expr),* $(,)?) => {
        &[$(($mime, $ext)),*]
    };
}

#[rustfmt::skip]
const MIME_MAP: &[(&str, &str)] = mime_table! {
    "application/java-archive" => "jar",
    "application/json"         => "json",
    "application/pdf"          => "pdf",
    "application/rss+xml"      => "rss",
    "application/wasm"         => "wasm",
    "application/xhtml+xml"    => "xhtml xhtm xht",
    "application/xml-dtd"      => "dtd",
    "application/xml"          => "xsl xml",
    "application/xslt+xml"     => "xslt",
    "application/zip"          => "zip",
    "audio/flac"               => "flac",
    "audio/mp4"                => "m4a",
    "audio/mpeg"               => "mp2 mp3 mpga",
    "audio/ogg"                => "ogg opus oga spx",
    "audio/wav"                => "wav",
    "audio/x-matroska"         => "mka",
    "audio/x-mpegurl"          => "m3u m3u8",
    "font/otf"                 => "otf",
    "font/ttf"                 => "ttf",
    "font/woff"                => "woff",
    "font/woff2"               => "woff2",
    "image/apng"               => "apng",
    "image/avif"               => "avif",
    "image/gif"                => "gif",
    "image/jpeg"               => "jpeg jpe jpg jfif",
    "image/jxl"                => "jxl",
    "image/png"                => "png",
    "image/svg+xml"            => "svg svgz",
    "image/webp"               => "webp",
    "text/css"                 => "css",
    "text/html"                => "html htm",
    "text/javascript"          => "js mjs",
    "text/plain"               => "txt asc conf log",
    "video/mp4"                => "mp4 m4v",
    "video/mpeg"               => "mpeg mpe mpg",
    "video/quicktime"          => "qt mov",
    "video/webm"               => "webm",
    "video/x-matroska"         => "mkv",
    "video/x-msvideo"          => "avi",
};

const DEFAULT_MIMETYPE: &str = "application/octet-stream";

async fn close() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.unwrap();
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        signal(SignalKind::terminate()).unwrap().recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn log_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    println!("Request: {method} {path}");

    let response = next.run(req).await;

    println!("Status: {}", response.status());

    response
}

async fn home() -> impl IntoResponse {
    let id = "index.html";
    file_assets(id)
}

async fn storage(
    uri: Uri,
    TypedHeader(host): TypedHeader<headers::Host>,
    referer: Option<TypedHeader<headers::Referer>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let mut field = multipart
        .next_field()
        .await?
        .ok_or(Error::BadRequest("Invalid input".to_string()))?;

    let _tmp = ATTACHMENT_PATH.join("_tmp");
    tokio::fs::create_dir_all(&_tmp).await?;

    let tmp = NamedTempFile::new_in(&_tmp)?;
    let obj = tmp.reopen()?;
    let dest = tokio::fs::File::from_std(obj);
    let mut writer = BufWriter::with_capacity(64 * 1024, dest);

    while let Some(chunk) = field.chunk().await? {
        writer.write_all(&chunk).await?;
    }
    writer.flush().await?;

    let pool = pool().await;
    let mut tx = pool.begin().await?;

    let filename = field.file_name().filter(|&s| s != "-").unwrap_or("unknown");
    let file = File::write_in_tx(filename, &mut tx).await?;

    let key = hash(&file.id);
    let fin = path_by(key);
    let parent = fin.parent().ok_or(Error::Unpredictable)?;
    tokio::fs::create_dir_all(parent).await?;

    tmp.persist(&fin).map_err(|e| Error::Io(e.error))?;

    tx.commit().await?;

    println!("storage: {} -> {key:08x}: {filename}", file.id);

    let base = match BASE_URL.as_deref() {
        Some(base_url) => base_url.trim_end_matches('/').to_string() + "/file",
        None => referer
            .map(|TypedHeader(r)| r.to_string().trim_end_matches('/').to_string())
            .unwrap_or_else(|| {
                format!(
                    "{}{}",
                    host.to_string().trim_end_matches('/'),
                    uri.path().trim_end_matches('/')
                )
            }),
    };

    let link = Link {
        url: format!("{base}/{}", file.id),
        token: file.token,
    };

    Ok(Json(link))
}

async fn download(Path(id): Path<String>) -> Result<impl IntoResponse, Error> {
    if matches!(id.as_str(), "script.js" | "style.css" | "yy.js") {
        return Ok(file_assets(&id));
    }

    let key = hash(&id);
    let (filename, mime) = File::read_multi_column(MultiColum::NameMime, &id).await?;

    let obj = path_by(key);
    let dest = tokio::fs::File::open(&obj).await?;
    let metadata = dest.metadata().await?;
    let size = metadata.len();

    let stream = tokio_util::io::ReaderStream::new(dest);
    let body = Body::from_stream(stream);

    let headers = [
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", escape(&filename)),
        ),
        (header::CONTENT_TYPE, mime),
        (header::CONTENT_LENGTH, size.to_string()),
    ];

    Ok((headers, body).into_response())
}

async fn remove(
    Path(id): Path<String>,
    TypedHeader(token): TypedHeader<TokenHeader>,
) -> Result<impl IntoResponse, Error> {
    let key = hash(&id);
    let recorded = File::read_column::<String>(Column::Token, &id).await?;

    if token != recorded {
        return Err(Error::Forbidden);
    }

    let pool = pool().await;
    let mut tx = pool.begin().await?;

    File::remove_in_tx(&id, &mut tx).await?;

    let dest = path_by(key);
    tokio::fs::remove_file(&dest).await?;

    tx.commit().await?;

    println!("remove: {id} -> {key:08x}");

    Ok(StatusCode::OK)
}

fn file_assets(file: &str) -> Response {
    match FileAssets::get(file) {
        Some(obj) => {
            let bytes = match obj.data {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(vec) => Bytes::from(vec),
            };

            let headers = [
                (header::CONTENT_TYPE, guess_mime(file)),
                (header::CACHE_CONTROL, "public, max-age=15552000"), // 60 * 60 * 24 * 30 * 6, 6 months
            ];

            (headers, bytes).into_response()
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
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if ext.is_empty() {
        return DEFAULT_MIMETYPE;
    }

    for (mime, exts) in MIME_MAP {
        if exts.split_whitespace().any(|x| x == ext) {
            return mime;
        }
    }

    DEFAULT_MIMETYPE
}

fn path_by(key: u32) -> path::PathBuf {
    let hex = format!("{key:08x}");
    let (dir, filename) = hex.split_at(2);

    ATTACHMENT_PATH.join(dir).join(filename)
}

fn escape(input: &str) -> Cow<'_, str> {
    if !input.contains(['"', '\\', '/', ':', '|', '<', '>', '?', '*']) {
        return Cow::Borrowed(input);
    }

    let mut s = String::with_capacity(input.len() + 10);

    for c in input.chars() {
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
        filename: &str,
        tx: &mut Transaction<'_, Sqlite>,
    ) -> Result<Self, sqlx::Error> {
        let id = rand_string(6);
        let file = File {
            id,
            token: random_token(),
        };

        let mime = guess_mime(filename);

        sqlx::query("INSERT INTO files (id, name, token, mime) VALUES (?1, ?2, ?3, ?4)")
            .bind(&file.id)
            .bind(filename)
            .bind(&file.token)
            .bind(mime)
            .execute(&mut **tx)
            .await?;

        Ok(file)
    }

    async fn remove_in_tx(id: &str, tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    async fn read_column<T>(column: Column, id: &str) -> Result<T, sqlx::Error>
    where
        T: for<'r> Decode<'r, Sqlite> + Type<Sqlite> + Send + Unpin,
    {
        let pool = pool().await;

        let query_str = match column {
            Column::Token => "SELECT token FROM files WHERE id = ?",
        };

        sqlx::query_scalar(query_str).bind(id).fetch_one(pool).await
    }

    async fn read_multi_column(
        column: MultiColum,
        id: &str,
    ) -> Result<(String, String), sqlx::Error> {
        let pool = pool().await;

        let sql = match column {
            MultiColum::NameMime => "SELECT name, mime FROM files WHERE id = ?",
        };

        sqlx::query_as::<_, (String, String)>(sql)
            .bind(id)
            .fetch_one(pool)
            .await
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
        .layer(DefaultBodyLimit::max(30 << 20)) // 30 MB
        .layer(CorsLayer::permissive())
}
