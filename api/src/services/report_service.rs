use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::env;
use std::path::PathBuf;
use serde_json::json;

use crate::helpers::allure_config::ensure_allure_config;
use crate::helpers::allure_generator::{collect_history, generate_report, sync_history};
use crate::helpers::fs_helper::{find_results_dir, next_sequential_id, move_directory_contents};
use crate::helpers::zip_helper::extract_zip;

const MAX_ZIP_SIZE_BYTES: u64 = 500 * 1024 * 1024; // 500MB
const MAX_ZIP_SIZE_MB: u64 = MAX_ZIP_SIZE_BYTES / (1024 * 1024);

pub async fn upload_report(
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut project_name: Option<String> = None;
    let mut branch: Option<String> = None;
    let mut report_name: Option<String> = None;
    let mut report_type: String = "allure".to_string();
    let mut zip_data: Option<Vec<u8>> = None;
    let mut zip_size: u64 = 0;

    let base_path = env::var("DATA_DIR").unwrap_or_else(|_| "../data".to_string());

    // Parse multipart fields
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or_default().to_string();

        match field_name.as_str() {
            "project_name" => {
                if let Ok(val) = field.text().await {
                    if !val.is_empty() { project_name = Some(val); }
                }
            }
            "branch" => {
                if let Ok(val) = field.text().await {
                    if !val.is_empty() { branch = Some(val); }
                }
            }
            "report_name" => {
                if let Ok(val) = field.text().await {
                    if !val.is_empty() { report_name = Some(val); }
                }
            }
            "type" | "report_type" => {
                if let Ok(val) = field.text().await {
                    if !val.is_empty() { report_type = val.to_lowercase(); }
                }
            }
            _ => {
                if let Some(file_name) = field.file_name() {
                    if field_name == "file" || file_name.ends_with(".zip") {
                        match field.bytes().await {
                            Ok(data) => {
                                zip_size = data.len() as u64;
                                
                                // Check file size before storing
                                if zip_size > MAX_ZIP_SIZE_BYTES {
                                    let size_mb = zip_size / (1024 * 1024);
                                    return (
                                        StatusCode::PAYLOAD_TOO_LARGE,
                                        Json(json!({
                                            "error": format!(
                                                "ZIP file size exceeds maximum limit of {}MB (received: {}MB)",
                                                MAX_ZIP_SIZE_MB, size_mb
                                            ),
                                            "max_size_bytes": MAX_ZIP_SIZE_BYTES,
                                            "max_size_mb": MAX_ZIP_SIZE_MB,
                                            "received_bytes": zip_size,
                                            "received_mb": size_mb,
                                            "field": "file"
                                        }))
                                    ).into_response();
                                }
                                
                                zip_data = Some(data.to_vec());
                            }
                            Err(e) => {
                                eprintln!("Failed to read file bytes: {}", e);
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!({
                                        "error": format!("Failed to read uploaded file: {}", e),
                                        "field": "file"
                                    }))
                                ).into_response();
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate required fields
    if project_name.is_none() || report_name.is_none() || branch.is_none() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": "Missing required metadata fields (project_name, report_name, branch)."
        }))).into_response();
    }
    if zip_data.is_none() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": "No ZIP file uploaded."
        }))).into_response();
    }

    // Build directory paths based on report type
    let mut parent_dir = PathBuf::from(&base_path);
    parent_dir.push(project_name.as_ref().unwrap());
    parent_dir.push(branch.as_ref().unwrap());
    parent_dir.push(report_name.as_ref().unwrap());

    // For raw reports, create a separate "raw" subdirectory
    if report_type == "raw" {
        parent_dir.push("raw");
    }

    if let Err(e) = tokio::fs::create_dir_all(&parent_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": format!("Failed to create parent directory: {}", e)
        }))).into_response();
    }

    let next_id = next_sequential_id(&parent_dir).await;
    let report_id = next_id.to_string();

    let report_dir = parent_dir.join(&report_id);
    let extract_dir = if report_type == "allure" {
        report_dir.join("allure-results")
    } else {
        report_dir.clone()
    };

    if let Err(e) = tokio::fs::create_dir_all(&extract_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": format!("Failed to create directory: {}", e)
        }))).into_response();
    }

    // Extract zip
    let zip_bytes = zip_data.unwrap();
    let target_dir = extract_dir.clone();

    let extract_result = tokio::task::spawn_blocking(move || extract_zip(zip_bytes, target_dir)).await;

    match extract_result {
        Ok(Ok(count)) => { println!("Extracted {} entries from zip", count); }
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": format!("Zip extraction failed: {}", e)
        }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": format!("Extraction panic: {}", e)
        }))).into_response(),
    }

    // Generate allure report only for allure type
    if report_type == "allure" {
        let actual_input_dir = find_results_dir(&extract_dir).await;
        println!("Resolved allure-results input dir: {:?}", actual_input_dir);

        // Write allure config
        if let Err(e) = ensure_allure_config(&parent_dir, report_name.as_ref().unwrap()).await {
            eprintln!("Warning: Failed to create allurerc.json: {}", e);
        }

        // Sync history before generation
        sync_history(&parent_dir, &actual_input_dir, &report_dir).await;

        // Canonicalize paths for the command
        let abs_input = std::fs::canonicalize(&actual_input_dir)
            .unwrap_or_else(|_| actual_input_dir.clone());
        let abs_output = std::fs::canonicalize(&report_dir)
            .unwrap_or_else(|_| report_dir.clone());
        let abs_parent = std::fs::canonicalize(&parent_dir)
            .unwrap_or_else(|_| parent_dir.clone());

        let gen_result = tokio::task::spawn_blocking(move || {
            generate_report(&abs_input, &abs_output, &abs_parent)
        }).await;

        match gen_result {
            Ok(Ok(msg)) => { println!("Allure generation succeeded: {}", msg); }
            Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": format!("Generation panic: {}", e)
            }))).into_response(),
        }

        // Move awesome directory contents to report_dir root
        let awesome_dir = report_dir.join("awesome");
        if awesome_dir.exists() {
            println!("Found awesome directory at {:?}", awesome_dir);
            if let Err(e) = move_directory_contents(&awesome_dir, &report_dir).await {
                eprintln!("Warning: Failed to move awesome directory contents: {}", e);
            }
        } else {
            eprintln!("Warning: awesome directory not found at {:?}", awesome_dir);
        }

        // Clean up awesome directory if it still exists
        if awesome_dir.exists() {
            if let Err(e) = tokio::fs::remove_dir(&awesome_dir).await {
                eprintln!("Warning: Failed to remove awesome directory: {}", e);
            }

            // Clean up allure-results directory if it still exists
            if let Err(e) = tokio::fs::remove_dir_all(&extract_dir).await {
                eprintln!("Warning: Failed to remove allure-results directory: {}", e);
            }
        }

        // Collect history after generation
        collect_history(&parent_dir, &actual_input_dir, &report_dir).await;
    }

    // Build URL based on report type
    let url = if report_type == "raw" {
        format!("/{}/{}/{}/raw/{}/index.html",
            project_name.as_ref().unwrap(),
            branch.as_ref().unwrap(),
            report_name.as_ref().unwrap(),
            report_id)
    } else {
        format!("/{}/{}/{}/{}/index.html",
            project_name.as_ref().unwrap(),
            branch.as_ref().unwrap(),
            report_name.as_ref().unwrap(),
            report_id)
    };

    (
        StatusCode::OK,
        Json(json!({
            "message": format!("Report uploaded successfully (Type: {})", report_type),
            "project_name": project_name,
            "branch": branch,
            "report_name": report_name,
            "report_id": report_id,
            "report_type": report_type,
            "url": url
        })),
    ).into_response()
}