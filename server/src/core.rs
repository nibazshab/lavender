use askama::Template;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_extra::{TypedHeader, headers};
use mime_guess::MimeGuess;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::borrow::Cow;
use tracing::info;

use crate::mode::{Assets, Error, Note};
use crate::utils;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/{id}", get(path_get).post(path_post))
        .route("/-/{id}", get(path_raw_get))
        .route("/", get(root_get).post(root_post))
        .route("/assets/{file}", get(assets))
        .route("/favicon.ico", get(favicon))
}

async fn path_get(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Result<impl IntoResponse, Error> {
    let note = Note::read(id, &pool).await?;

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

async fn path_raw_get(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, Error> {
    if id.len() > 64 {
        return Ok(Redirect::temporary(&utils::rand_string(4)).into_response());
    }

    let note = Note::read(id, &pool).await?;

    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        note.content,
    )
        .into_response())
}

async fn root_get() -> Redirect {
    Redirect::temporary(&utils::rand_string(4))
}

async fn path_post(
    Path(id): Path<String>,
    State(pool): State<SqlitePool>,
    bytes: Bytes,
) -> Result<impl IntoResponse, Error> {
    if id.len() > 64 {
        return Err(Error::BadRequest(
            "Invalid, expecting id length < 64.".to_string(),
        ));
    }

    let note = parsing(id.clone(), &bytes).await?;
    note.write(&pool).await?;

    info!("{id}");
    Ok(StatusCode::OK.into_response())
}

async fn root_post(
    State(pool): State<SqlitePool>,
    TypedHeader(host): TypedHeader<headers::Host>,
    bytes: Bytes,
) -> Result<impl IntoResponse, Error> {
    let id = utils::rand_string(5);

    let note = parsing(id.clone(), &bytes).await?;
    note.write(&pool).await?;

    info!("{id}");
    Ok((StatusCode::OK, format!("{host}/-/{id}")).into_response())
}

async fn parsing(id: String, bytes: &Bytes) -> Result<Note, Error> {
    #[derive(Deserialize)]
    struct Payload {
        t: String,
    }

    let t = 'a: {
        if let Ok(form) = serde_urlencoded::from_bytes::<Payload>(bytes) {
            break 'a form;
        }

        if let Ok(t) = std::str::from_utf8(bytes) {
            break 'a Payload { t: t.to_string() };
        }

        return Err(Error::BadRequest(
            "Invalid, expecting text utf-8.".to_string(),
        ));
    };

    Ok(Note { id, content: t.t })
}

async fn favicon() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/x-icon")], vec![]).into_response()
}

async fn assets(Path(id): Path<String>) -> impl IntoResponse {
    match Assets::get(&id) {
        Some(obj) => release_assets(&id, obj.data),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn release_assets(filename: &str, data: Cow<'static, [u8]>) -> Response {
    let content_type = MimeGuess::from_path(filename).first_or_octet_stream();
    let cache_control = format!("public, max-age={}", 60 * 60 * 24 * 7);

    let bytes = match data {
        Cow::Borrowed(slice) => Bytes::from_static(slice),
        Cow::Owned(vec) => Bytes::from(vec),
    };

    (
        [
            (header::CONTENT_TYPE, content_type.as_ref()),
            (header::CACHE_CONTROL, cache_control.as_str()),
        ],
        bytes,
    )
        .into_response()
}
