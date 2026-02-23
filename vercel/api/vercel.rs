use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;

use app::router;

#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let router = router();

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}
