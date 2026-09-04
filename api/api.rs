#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let router = app::router().await;

    let app = tower::ServiceBuilder::new()
        .layer(vercel_runtime::axum::VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}
