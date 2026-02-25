#[cfg(feature = "serverless")]
use app::vercel::app;

#[cfg(feature = "server")]
use app::server::app;

#[cfg(feature = "serverless")]
#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let app = app();

    vercel_runtime::run(app).await
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    if let Err(e) = app().await {
        eprintln!("{e}");

        std::process::exit(1);
    }
}
