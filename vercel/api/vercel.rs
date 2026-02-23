use axum::Router;
use tower::ServiceBuilder;
use vercel_runtime::axum::{VercelLayer, VercelService};

use app::router;

pub fn app() -> VercelService<Router> {
    let router = router();

    ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router)
}
