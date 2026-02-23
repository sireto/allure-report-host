use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Allure,
    Raw,
}

#[derive(ToSchema)]
#[allow(dead_code)]
pub struct FileUploadRequest {
    #[schema(example = "my-project")]
    project_name: String,
    #[schema(example = "qa")]
    branch: String,
    #[schema(example = "daily-test")]
    report_name: String,
    #[schema(default = "allure", example = "allure")]
    r#type: Option<ReportType>,
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct CreateReportRequest {
    pub branch: String,
    pub report_name: String,
    pub report_type: ReportType,
    /// Auto-generated if not provided
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ReportResponse {
    pub run_id: String,
    pub message: String,
    pub status: String,
}
