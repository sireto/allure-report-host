use axum::{
    extract::{Multipart},
    response::IntoResponse,
};
use utoipa::ToSchema;
use crate::services::report_service;

#[derive(ToSchema)]
pub struct FileUploadRequest {
    #[schema(example = "my-project")]
    project_name: String,
    #[schema(example = "daily-test")]
    report_name: String,
    #[schema(default = "allure", example = "allure")]
    r#type: Option<String>, 
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

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
pub async fn upload_report(
    multipart: Multipart,
) -> impl IntoResponse {
    report_service::upload_report(multipart).await
}