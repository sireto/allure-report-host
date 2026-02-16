use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::io::Cursor;
use serde_json::json;
use uuid::Uuid;
use zip::ZipArchive;

pub async fn upload_report(
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut project_name: Option<String> = None;
    let mut report_name: Option<String> = None;
    let mut report_id: Option<String> = None;
    let mut report_type: String = "allure".to_string(); // Default to 'allure'
    let mut zip_data: Option<Vec<u8>> = None;
    
    let base_path = env::var("DATA_DIR").unwrap_or_else(|_| "../data".to_string());

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or_default().to_string();

        match field_name.as_str() {
            "project_name" => {
                if let Ok(val) = field.text().await { if !val.is_empty() { project_name = Some(val); } }
            }
            "report_name" => {
                if let Ok(val) = field.text().await { if !val.is_empty() { report_name = Some(val); } }
            }
            "report_id" => {
                if let Ok(val) = field.text().await { if !val.is_empty() { report_id = Some(val); } }
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
                            Ok(data) => { zip_data = Some(data.to_vec()); }
                            Err(e) => { eprintln!("Failed to read file bytes: {}", e); }
                        }
                    }
                }
            }
        }
    }

    if project_name.is_none() || report_name.is_none() {
         return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Missing required metadata fields (project_name, report_name)." }))).into_response();
    }
    if zip_data.is_none() {
         return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No ZIP file uploaded." }))).into_response();
    }
    if report_id.is_none() {
        report_id = Some(Uuid::new_v4().to_string());
    }

    let mut parent_dir = PathBuf::from(&base_path);
    parent_dir.push("reports");
    parent_dir.push(project_name.as_ref().unwrap());
    parent_dir.push(report_name.as_ref().unwrap());

    if !parent_dir.exists() {
        if let Err(e) = tokio::fs::create_dir_all(&parent_dir).await {
             return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Failed to create parent directory: {}", e) }))).into_response();
        }
    }

    // Read directory to find max ID
    let mut next_id = 1;

    match tokio::fs::read_dir(&parent_dir).await {
        Ok(mut entries) => {
            let mut max_id = 0;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            if let Ok(id) = name.parse::<u32>() {
                                if id > max_id {
                                    max_id = id;
                                }
                            }
                        }
                    }
                }
            }
            next_id = max_id + 1;
        }
        Err(e) => {
             eprintln!("Error reading directory for ID generation: {}", e);
        }
    }

    let report_id = next_id.to_string();

    let mut report_dir = parent_dir.clone();    // parent --> data directory
    report_dir.push(&report_id);

    // If 'allure', we need a temporary place to extract the raw JSONs before generating index.html to report_dir
    // If 'raw', we extract directly to report_dir
    let extract_dir = if report_type == "allure" {
        report_dir.join("allure-results")
    } else {
        report_dir.clone()
    };

    // Create directory
    if let Err(e) = tokio::fs::create_dir_all(&extract_dir).await {
         return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Failed to create directory: {}", e) }))).into_response();
    }

    let zip_bytes = zip_data.unwrap();
    let target_dir = extract_dir.clone();
    
    let extract_task = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let reader = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
                }
                let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
        Ok(archive.len())
    }).await;

    match extract_task {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Zip extraction failed: {}", e) }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Extraction panic: {}", e) }))).into_response(),
    }

    if report_type == "allure" {
        let output_dir_str = report_dir.to_string_lossy().to_string(); // .../uuid/
        let input_dir_str = extract_dir.to_string_lossy().to_string(); // .../uuid/allure-results

        let gen_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            // Updated command to use 'npx'
            // We use 'npx' 'allure-commandline' 'generate' ...
            let output = Command::new("npx")
                .arg("allure-commandline")
                .arg("generate")
                .arg(&input_dir_str)
                .arg("-o")
                .arg(&output_dir_str)
                .arg("--clean")
                .output()
                .map_err(|e| format!("Failed to run npx command: {}", e))?;

            if !output.status.success() {
                return Err(format!("Allure generation failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            Ok(())
        }).await;

        match gen_result {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Generation panic: {}", e) }))).into_response(),
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "message": format!("Report uploaded successfully (Type: {})", report_type),
            "project_name": project_name,
            "report_name": report_name,
            "report_id": report_id,
            "url": format!("/reports/{}/{}/{}/index.html", project_name.unwrap(), report_name.unwrap(), report_id)
        })),
    ).into_response()
}