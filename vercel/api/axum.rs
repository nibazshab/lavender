use askama::Template;
use axum::extract::Path;
use axum::http::Uri;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{
    Router,
    routing::{get, post},
};
use axum_extra::TypedHeader;
use axum_extra::headers::{self, CacheControl};
use hyper::body::Bytes;
use hyper::{StatusCode, header};
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tokio::time::Duration;
use tower::ServiceBuilder;
use vercel_runtime::Error;
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

async fn root() -> Redirect {
    Redirect::temporary(&rand_string(4))
}

async fn home(
    Path(id): Path<String>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> impl IntoResponse {
    let note = Note {
        id: id,
        content: "This is a note.".to_string(),
    };

    const CLI: [&str; 2] = ["curl", "wget"];
    let is_cli = CLI.iter().any(|agent| user_agent.as_str().contains(agent));

    if is_cli {
        (
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            note.content,
        )
            .into_response()
    } else {
        let html = note.render().unwrap();
        Html(html).into_response()
    }
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

            let cache_control = format!("public, max-age={}", 60 * 60 * 24 * 30 * 6); // 6 months

            let bytes = match obj.data {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(vec) => Bytes::from(vec),
            };

            (
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, cache_control.as_str()),
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
            (
                header::CACHE_CONTROL,
                format!("public, max-age={}", 60 * 60 * 24 * 30 * 12).as_str(),
            ),
        ],
        vec![],
    )
        .into_response()
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    format!("Axum fallback for path {}", uri.path())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let router = Router::new()
        .route("/", get(root))
        .route("/{id}", get(home))
        .route("/assets/{file}", get(assets))
        .route("/favicon.ico", get(favicon))
        .fallback(fallback);

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}

fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}
