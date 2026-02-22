use axum::Router;
use sqlx::SqlitePool;

#[cfg(feature = "file")]
mod file;

pub fn inits() -> std::io::Result<()> {
    #[cfg(feature = "file")]
    file::init_os_dir()?;

    Ok(())
}

pub fn routers() -> Router<SqlitePool> {
    #[cfg(feature = "file")]
    return file::router();

    #[cfg(not(feature = "file"))]
    return Router::new();
}

pub fn schemas() -> &'static str {
    #[cfg(feature = "file")]
    return file::schema();

    #[cfg(not(feature = "file"))]
    return "";
}
