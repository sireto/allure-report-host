use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Allure,
    Raw,
    Other,
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
