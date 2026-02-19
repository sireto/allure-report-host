use crate::handlers::report_handler;
use axum::{Router, routing::post};

pub fn router() -> Router {
    Router::new().route("/reports/upload", post(report_handler::upload_report))
}
