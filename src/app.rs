use crate::router;

use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;

#[tokio::main]
pub async fn main() -> Result<(), vercel_runtime::Error> {
    let router = router().await.map_err(|e| e.to_string())?;

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}
