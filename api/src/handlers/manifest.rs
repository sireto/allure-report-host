use axum::{Json, response::IntoResponse};
use serde_json::json;

/// Get manifest of all projects, branches, and reports
pub async fn get_manifest() -> impl IntoResponse {
    use std::collections::BTreeMap;

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "../data".to_string());
    let data_path = std::path::PathBuf::from(&data_dir);

    let mut projects: BTreeMap<String, BTreeMap<String, Vec<serde_json::Value>>> = BTreeMap::new();

    if let Ok(project_dirs) = std::fs::read_dir(&data_path) {
        for project_entry in project_dirs.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let project_name = project_entry.file_name().to_string_lossy().to_string();
            let mut branches: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();

            if let Ok(branch_dirs) = std::fs::read_dir(&project_path) {
                for branch_entry in branch_dirs.flatten() {
                    let branch_path = branch_entry.path();
                    if !branch_path.is_dir() {
                        continue;
                    }

                    let branch_name = branch_entry.file_name().to_string_lossy().to_string();
                    let mut reports = vec![];

                    if let Ok(report_dirs) = std::fs::read_dir(&branch_path) {
                        for report_entry in report_dirs.flatten() {
                            let report_path = report_entry.path();
                            if !report_path.is_dir() {
                                continue;
                            }

                            let report_name =
                                report_entry.file_name().to_string_lossy().to_string();

                            // Skip numeric directories
                            if report_name.parse::<u32>().is_ok() {
                                continue;
                            }

                            // Skip 'raw' directory, look inside it for reports
                            if report_name == "raw" {
                                if let Ok(raw_reports) = std::fs::read_dir(&report_path) {
                                    for raw_entry in raw_reports.flatten() {
                                        let _raw_path = raw_entry.path();
                                        if let Ok(file_name) = raw_entry.file_name().into_string() {
                                            if let Ok(id) = file_name.parse::<u32>() {
                                                let url = format!(
                                                    "/{}/{}/{}/raw/{}/index.html",
                                                    project_name, branch_name, report_name, id
                                                );
                                                reports.push(json!({
                                                    "name": format!("{} (Raw)", report_name),
                                                    "id": id,
                                                    "path": url,
                                                    "type": "raw"
                                                }));
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Find latest numeric report ID
                                if let Ok(report_ids) = std::fs::read_dir(&report_path) {
                                    let mut max_id: u32 = 0;
                                    for id_entry in report_ids.flatten() {
                                        if let Ok(file_name) = id_entry.file_name().into_string() {
                                            if let Ok(id) = file_name.parse::<u32>() {
                                                if id > max_id {
                                                    max_id = id;
                                                }
                                            }
                                        }
                                    }

                                    if max_id > 0 {
                                        let url = format!(
                                            "/{}/{}/{}/{}/index.html",
                                            project_name, branch_name, report_name, max_id
                                        );
                                        reports.push(json!({
                                            "name": report_name,
                                            "id": max_id,
                                            "path": url,
                                            "type": "allure"
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    if !reports.is_empty() {
                        branches.insert(branch_name, reports);
                    }
                }
            }

            if !branches.is_empty() {
                projects.insert(project_name, branches);
            }
        }
    }

    Json(projects)
}
