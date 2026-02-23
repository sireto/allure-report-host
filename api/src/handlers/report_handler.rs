use crate::models::report::FileUploadRequest;
use crate::services::report_service;
use axum::{extract::Multipart, response::IntoResponse};

#[utoipa::path(
    post,
    path = "/api/reports/upload",
    tag = "reports",
    request_body(content = FileUploadRequest, content_type = "multipart/form-data", description = "Report files to upload"),
    responses(
        (status = 200, description = "Files uploaded successfully"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn upload_report(multipart: Multipart) -> impl IntoResponse {
    report_service::upload_report(multipart).await
}
