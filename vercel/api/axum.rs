use askama::Template;
use axum::extract::Path;
use axum::http::Uri;
use axum::response::{Html, IntoResponse, Response};
use axum::{
    Router,
    routing::{get, post},
};
use hyper::body::Bytes;
use hyper::{StatusCode, header};
use mime_guess::MimeGuess;
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

async fn home() -> impl IntoResponse {
    let note = Note {
        id: "1".to_string(),
        content: "This is a note.".to_string(),
    };
    let html = note.render().unwrap();
    Html(html)
}

async fn assets(Path(id): Path<String>) -> impl IntoResponse {
    match Assets::get(&id) {
        Some(file) => {
            let content_type = MimeGuess::from_path(&id).first_or_octet_stream();
            let cache_control = format!("public, max-age={}", 60 * 60 * 24 * 30 * 6); // 6 months

            let bytes = match file.data {
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

        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    format!("Axum fallback for path {}", uri.path())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let router = Router::new()
        .route("/", get(home))
        .route("/assets/{id}", get(assets))
        .fallback(fallback);

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);
    vercel_runtime::run(app).await
}
