use axum::Router;
use crate::routes::report_route;

pub fn create_api_router() -> Router {
    Router::new()
        .merge(report_route::router())
}