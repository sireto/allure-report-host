use axum::{routing::post, Router};
use crate::handlers::report_handler;

pub fn router() -> Router {
    Router::new()
        .route("/reports/upload", post(report_handler::upload_report))
}