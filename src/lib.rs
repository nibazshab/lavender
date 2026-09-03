pub mod app;
mod database;

use askama::Template;
use axum::body::Bytes;
use axum::extract::multipart::{MultipartError, MultipartRejection};
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, FromRequest, Multipart, Path, Request};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Router, routing::get};
use axum_extra::{TypedHeader, headers};
use const_format::concatcp;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use rust_embed::RustEmbed;
use thiserror::Error;
use tower_http::cors::CorsLayer;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
enum Error {
    #[error("{0}")]
    BadRequest(String),

    #[error(transparent)]
    Bytes(#[from] BytesRejection),

    #[error(transparent)]
    MultipartRejection(#[from] MultipartRejection),

    #[error(transparent)]
    Multipart(#[from] MultipartError),

    #[error("{0}")]
    Template(#[from] askama::Error),

    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),

            Error::Bytes(rejection) => rejection.into_response(),
            Error::MultipartRejection(rejection) => rejection.into_response(),
            Error::Multipart(rejection) => rejection.into_response(),

            _err @ (Error::Template(_) | Error::Sqlx(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

struct Content(String);

impl Content {
    async fn from_body<S>(req: Request, state: &S) -> Result<Self>
    where
        S: Send + Sync,
    {
        let bytes = Bytes::from_request(req, state).await?;

        let (text, _, malformed) = encoding_rs::UTF_8.decode(&bytes);
        if !malformed {
            return Ok(Self(text.into_owned()));
        }

        let (text, _, malformed) = encoding_rs::GBK.decode(&bytes);
        if !malformed {
            return Ok(Self(text.into_owned()));
        }

        Err(Error::BadRequest("not utf-8/gbk".into()))
    }

    async fn from_multipart<S>(req: Request, state: &S) -> Result<Self>
    where
        S: Send + Sync,
    {
        let mut multipart = Multipart::from_request(req, state).await?;
        let mut text = String::new();

        while let Some(field) = multipart.next_field().await? {
            let val = field.text().await?;
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&val);
        }

        Ok(Self(text))
    }
}

impl<S> FromRequest<S> for Content
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("multipart/form-data") {
            Self::from_multipart(req, state).await
        } else {
            Self::from_body(req, state).await
        }
    }
}

#[derive(RustEmbed)]
#[folder = "templates/assets/"]
struct Assets;

const MAX_AGE: i64 = 60 * 60 * 24 * 30 * 6;
const CACHE_CONTROL: &str = concatcp!("public, max-age=", MAX_AGE);

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct Note {
    id: String,
    content: String,
}

impl Note {
    async fn schema() -> Result<()> {
        let db = database::get().await?;

        let s = r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                content TEXT
            );
            "#;

        sqlx::query(s).execute(db).await?;
        Ok(())
    }

    async fn read(id: &str) -> Result<Self> {
        let db = database::get().await?;

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

    async fn write(&self) -> Result<()> {
        let db = database::get().await?;

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

async fn redirect() -> impl IntoResponse {
    Redirect::temporary(&rand_string(4))
}

async fn reader(
    Path(id): Path<String>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Result<impl IntoResponse> {
    let ua = user_agent.as_str().to_lowercase();
    let is_cli = ua.contains("curl") || ua.contains("wget");
    if is_cli {
        return content(Path(id)).await.map(|r| r.into_response());
    }

    let note = Note::read(&id).await?;
    let chars = note.render()?;

    Ok(Html(chars).into_response())
}

async fn content(Path(id): Path<String>) -> Result<impl IntoResponse> {
    let note = Note::read(&id).await?;

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        note.content,
    ))
}

async fn writer(Path(id): Path<String>, Content(content): Content) -> Result<impl IntoResponse> {
    let note = Note { id, content };
    note.write().await?;

    Ok(StatusCode::OK)
}

async fn assets(Path(file): Path<String>) -> impl IntoResponse {
    let Some(obj) = Assets::get(&file) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = match file.rsplit('.').next() {
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    };

    let bytes: Bytes = obj.data.into_owned().into();

    let headers = [
        (header::CONTENT_TYPE, content_type),
        (header::CACHE_CONTROL, CACHE_CONTROL),
    ];

    (headers, bytes).into_response()
}

async fn favicon() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/x-icon"),
            (header::CACHE_CONTROL, CACHE_CONTROL),
        ],
        vec![],
    )
        .into_response()
}

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

async fn router() -> Result<Router> {
    Note::schema().await?;

    let endpoints = Router::new()
        .route("/", get(redirect))
        .route("/{id}", get(reader).post(writer).put(writer))
        .route("/d/{id}", get(content));

    let resources = Router::new()
        .route("/assets/{file}", get(assets))
        .route("/favicon.ico", get(favicon));

    let router = Router::new()
        .merge(endpoints)
        .merge(resources)
        .layer(DefaultBodyLimit::max(3 << 20))
        .layer(CorsLayer::permissive());

    Ok(router)
}
