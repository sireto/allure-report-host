use crate::routes::report_route;
use axum::Router;

pub fn create_api_router() -> Router {
    Router::new().merge(report_route::router())
}
