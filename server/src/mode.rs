use askama::Template;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::error;

#[derive(Debug, Template)]
#[template(path = "index.html")]
pub struct Note {
    pub id: String,
    pub content: String,
}

#[derive(rust_embed::RustEmbed)]
#[folder = "templates/assets/"]
pub struct Assets;

pub enum Error {
    BadRequest(String),
    Sqlx(sqlx::Error),
    Template(askama::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Sqlx(e) => {
                error!("{e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            Error::Template(e) => {
                error!("{e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
        };

        (status, message).into_response()
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Sqlx(err)
    }
}

impl From<askama::Error> for Error {
    fn from(err: askama::Error) -> Self {
        Error::Template(err)
    }
}
