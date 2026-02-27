#[cfg(feature = "serverless")]
use app::vercel::app;

#[cfg(feature = "server")]
use app::server::app;

#[cfg(feature = "serverless")]
#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    app().await
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app().await
}
