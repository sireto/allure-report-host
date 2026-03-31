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
                                        if let Ok(file_name) = raw_entry.file_name().into_string()
                                            && let Ok(id) = file_name.parse::<u32>()
                                        {
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
                            } else {
                                // Collect ALL numeric run directories for this report
                                if let Ok(report_ids) = std::fs::read_dir(&report_path) {
                                    let mut runs: Vec<serde_json::Value> = Vec::new();

                                    for id_entry in report_ids.flatten() {
                                        let file_name_os = id_entry.file_name();
                                        let Some(file_name) = file_name_os.to_str() else {
                                            continue;
                                        };
                                        // Skip non-numeric dirs (e.g. "latest" symlink, history.jsonl)
                                        let Ok(id) = file_name.parse::<u32>() else {
                                            continue;
                                        };

                                        let url = format!(
                                            "/{}/{}/{}/{}/index.html",
                                            project_name, branch_name, report_name, id
                                        );

                                        // Read metadata.json for this run if it exists
                                        let metadata: Option<serde_json::Value> =
                                            std::fs::read_to_string(
                                                report_path.join(file_name).join("metadata.json"),
                                            )
                                            .ok()
                                            .and_then(|s| serde_json::from_str(&s).ok());

                                        let run_id = metadata
                                            .as_ref()
                                            .and_then(|m| m["run_id"].as_str())
                                            .map(|s| s.to_string());
                                        let created_at = metadata
                                            .as_ref()
                                            .and_then(|m| m["created_at"].as_u64());

                                        runs.push(json!({
                                            "id": id,
                                            "run_id": run_id,
                                            "created_at": created_at,
                                            "path": url,
                                        }));
                                    }

                                    if !runs.is_empty() {
                                        // Sort runs by id descending (newest first)
                                        runs.sort_by(|a, b| {
                                            let id_b = b["id"].as_u64().unwrap_or(0);
                                            let id_a = a["id"].as_u64().unwrap_or(0);
                                            id_b.cmp(&id_a)
                                        });

                                        let latest_url = format!(
                                            "/{}/{}/{}/latest/index.html",
                                            project_name, branch_name, report_name
                                        );

                                        reports.push(json!({
                                            "name": report_name,
                                            "latest_path": latest_url,
                                            "type": "allure",
                                            "runs": runs
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
