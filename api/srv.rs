#[cfg(feature = "serverless")]
mod vercel;

#[cfg(not(feature = "serverless"))]
mod server;

#[cfg(feature = "serverless")]
#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let app = vercel::app();

    vercel_runtime::run(app).await
}

#[cfg(not(feature = "serverless"))]
#[tokio::main]
async fn main() {
    if let Err(e) = server::app().await {
        eprintln!("{e}");

        std::process::exit(1);
    }
}
