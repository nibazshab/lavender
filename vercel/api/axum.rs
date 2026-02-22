use askama::Template;
use axum::extract::{DefaultBodyLimit, FromRequest, Path, Request};
use axum::http::Uri;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Router, routing::get};
use axum_extra::TypedHeader;
use axum_extra::headers;
use hyper::body::Bytes;
use hyper::{StatusCode, header};
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use rust_embed::RustEmbed;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::borrow::Cow;
use std::time::Duration;
use tokio::sync::OnceCell;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use vercel_runtime::axum::VercelLayer;

#[derive(RustEmbed)]
#[folder = "templates/assets/"]
struct Assets;

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct Note {
    id: String,
    content: String,
}

#[derive(Deserialize)]
struct NoteForm {
    t: String,
}

struct NoteContent(String);

impl<S> FromRequest<S> for NoteContent
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|_| Error::BadRequest("Failed to read body".into()))?;

        let con = serde_urlencoded::from_bytes::<NoteForm>(&bytes)
            .map(|f| f.t)
            .or_else(|_| {
                std::str::from_utf8(&bytes)
                    .map(|s| s.to_string())
                    .map_err(|_| ())
            })
            .map_err(|_| Error::BadRequest("Invalid input".into()))?;

        Ok(NoteContent(con))
    }
}

enum Error {
    BadRequest(String),
    Template(askama::Error),
    Sqlx(sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),

            Error::Template(e) => {
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
        };

        (status, message).into_response()
    }
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

static POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn redirect() -> impl IntoResponse {
    Redirect::temporary(&rand_string(4))
}

async fn home(
    Path(id): Path<String>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Result<impl IntoResponse, Error> {
    let note = Note::read(id).await?;

    const CLI: [&str; 2] = ["curl", "wget"];
    let is_cli = CLI.iter().any(|agent| user_agent.as_str().contains(agent));

    if is_cli {
        Ok((
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            note.content,
        )
            .into_response())
    } else {
        let html = note.render()?;
        Ok(Html(html).into_response())
    }
}

async fn raw(Path(id): Path<String>) -> Result<impl IntoResponse, Error> {
    let note = Note::read(id).await?;

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        note.content,
    ))
}

async fn assets(Path(file): Path<String>) -> impl IntoResponse {
    match Assets::get(&file) {
        Some(obj) => {
            let content_type = if file.ends_with(".js") {
                "text/javascript"
            } else if file.ends_with(".css") {
                "text/css"
            } else {
                "application/octet-stream"
            };

            let bytes = match obj.data {
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

        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn favicon() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/x-icon"),
            (header::CACHE_CONTROL, "public, max-age=31104000"), // 60 * 60 * 24 * 30 * 12, 1 year
        ],
        vec![],
    )
}

async fn update_data(
    Path(id): Path<String>,
    NoteContent(content): NoteContent,
) -> Result<impl IntoResponse, Error> {
    let note = Note { id, content };

    note.write().await?;

    Ok(StatusCode::OK)
}

async fn random_data(
    TypedHeader(host): TypedHeader<headers::Host>,
    NoteContent(content): NoteContent,
) -> Result<impl IntoResponse, Error> {
    let id = rand_string(5);
    let note = Note {
        id: id.clone(),
        content,
    };

    note.write().await?;

    Ok((StatusCode::OK, format!("{host}/-/{id}")))
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    format!("Axum fallback for path {}", uri.path())
}

#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let router = Router::new()
        .route("/", get(redirect).post(random_data))
        .route("/{id}", get(home).post(update_data))
        .route("/d/{id}", get(raw))
        .route("/assets/{file}", get(assets))
        .route("/favicon.ico", get(favicon))
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(5 << 20)) // 5 MB
        .layer(CorsLayer::permissive());

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}

impl Note {
    async fn write(&self) -> Result<(), sqlx::Error> {
        let pool = pool().await;

        sqlx::query(
            r#"
            INSERT INTO notes (id, content) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET
                content = excluded.content
            "#,
        )
        .bind(&self.id)
        .bind(&self.content)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn read(id: String) -> Result<Self, sqlx::Error> {
        let pool = pool().await;

        let content: String = sqlx::query_scalar("SELECT content FROM notes WHERE id = $1")
            .bind(id.clone())
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();

        Ok(Note { id, content })
    }
}

async fn pool() -> &'static PgPool {
    POOL.get_or_init(|| async {
        let db_str = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/postgres".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .min_connections(0)
            .idle_timeout(Duration::from_secs(30))
            .connect(&db_str)
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                content TEXT
            );
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    })
    .await
}

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}
