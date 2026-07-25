pub mod app;
mod database;

use askama::Template;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRequest, Multipart, Path, Request};
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Router, routing::get};
use axum_extra::{TypedHeader, headers};
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use tower_http::cors::CorsLayer;

use crate::database::lib::db;

enum Error {
    BadRequest(String),
    Template(askama::Error),
    Sqlx(sqlx::Error),
}

impl From<askama::Error> for Error {
    fn from(err: askama::Error) -> Self {
        Error::Template(err)
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Sqlx(err)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                StatusCode::BAD_REQUEST.to_string() + msg.as_str(),
            ),

            Error::Template(_e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR.to_string(),
            ),

            Error::Sqlx(_e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR.to_string(),
            ),
        };

        (status, message).into_response()
    }
}

struct Content(String);

impl<S> FromRequest<S> for Content
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("multipart/form-data") {
            read_multipart(req, state).await
        } else {
            read_body(req, state).await
        }
    }
}

async fn read_body<S>(req: Request, state: &S) -> Result<Content, Error>
where
    S: Send + Sync,
{
    let bytes = Bytes::from_request(req, state)
        .await
        .map_err(|_| Error::BadRequest("failed to read body".into()))?;

    let (text, _, malformed) = encoding_rs::UTF_8.decode(&bytes);
    if !malformed {
        return Ok(Content(text.into_owned()));
    }

    let (text, _, malformed) = encoding_rs::GBK.decode(&bytes);
    if !malformed {
        return Ok(Content(text.into_owned()));
    }

    Err(Error::BadRequest("unsupported character encoding".into()))
}

async fn read_multipart<S>(req: Request, state: &S) -> Result<Content, Error>
where
    S: Send + Sync,
{
    let mut multipart = Multipart::from_request(req, state)
        .await
        .map_err(|_| Error::BadRequest("invalid multipart body".into()))?;

    let mut result = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| Error::BadRequest("invalid multipart field".into()))?
    {
        let text = field
            .text()
            .await
            .map_err(|_| Error::BadRequest("invalid multipart data".into()))?;

        if !result.is_empty() {
            result.push('\n');
        }

        result.push_str(&text);
    }

    Ok(Content(result))
}

#[derive(RustEmbed)]
#[folder = "templates/assets/"]
struct Assets;

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct Note {
    id: String,
    content: String,
}

async fn redirect() -> impl IntoResponse {
    Redirect::temporary(&rand_string(4))
}

async fn reader(
    Path(id): Path<String>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Result<impl IntoResponse, Error> {
    let note = Note::read(&id).await?;

    const CLI: [&str; 2] = ["curl", "wget"];
    let is_cli = CLI.iter().any(|agent| user_agent.as_str().contains(agent));

    if is_cli {
        Ok((
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            note.content,
        )
            .into_response())
    } else {
        let txt = note.render()?;
        Ok(Html(txt).into_response())
    }
}

async fn raw(Path(id): Path<String>) -> Result<impl IntoResponse, Error> {
    let note = Note::read(&id).await?;

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        note.content,
    ))
}

async fn writer(
    Path(id): Path<String>,
    Content(content): Content,
) -> Result<impl IntoResponse, Error> {
    let note = Note { id, content };

    note.write().await?;

    Ok(StatusCode::OK)
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("fallback for path {}\n", uri.path()),
    )
}

async fn assets(Path(file): Path<String>) -> impl IntoResponse {
    match Assets::get(&file) {
        Some(obj) => {
            let content_type = match () {
                _ if file.ends_with(".js") => "text/javascript",
                _ if file.ends_with(".css") => "text/css",
                _ => "application/octet-stream",
            };

            let bytes = match obj.data {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(vec) => Bytes::from(vec),
            };

            const AGE: i64 = 60 * 60 * 24 * 30 * 6;
            let cache = format!("public, max-age={AGE}");

            let headers = [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, cache.as_str()),
            ];

            (headers, bytes).into_response()
        }

        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn favicon() -> impl IntoResponse {
    const AGE: i64 = 60 * 60 * 24 * 30 * 12;
    let cache = format!("public, max-age={AGE}");

    (
        [
            (header::CONTENT_TYPE, "image/x-icon"),
            (header::CACHE_CONTROL, cache.as_str()),
        ],
        vec![],
    )
        .into_response()
}

impl Note {
    async fn schema() -> Result<(), sqlx::Error> {
        let db = db().await?;

        let s = r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                content TEXT
            );
            "#;

        sqlx::query(s).execute(db).await?;
        Ok(())
    }

    async fn read(id: &str) -> Result<Self, Error> {
        let db = db().await?;

        let s = "SELECT content FROM notes WHERE id = $1";

        let content = sqlx::query_scalar(s)
            .bind(id)
            .fetch_optional(db)
            .await?
            .unwrap_or_default();

        Ok(Note {
            id: id.to_string(),
            content,
        })
    }

    async fn write(&self) -> Result<(), Error> {
        let db = db().await?;

        let s = r#"
            INSERT INTO notes (id, content) VALUES ($1, $2) ON CONFLICT(id) DO
            UPDATE SET content = excluded.content
            "#;

        sqlx::query(s)
            .bind(&self.id)
            .bind(&self.content)
            .execute(db)
            .await?;

        Ok(())
    }
}

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

fn router() -> Router {
    let ass = Router::new()
        .route("/assets/{file}", get(assets))
        .route("/favicon.ico", get(favicon));

    Router::new()
        .route("/", get(redirect))
        .route("/{id}", get(reader).post(writer).put(writer))
        .route("/d/{id}", get(raw))
        .merge(ass)
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(3 << 20))
        .layer(CorsLayer::permissive())
}

async fn init() -> Result<(), Box<dyn std::error::Error>> {
    Note::schema().await?;
    Ok(())
}
