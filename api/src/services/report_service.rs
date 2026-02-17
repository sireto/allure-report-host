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
use zip::ZipArchive;
use crate::helpers::allure_config::ensure_allure_config;

pub async fn upload_report(
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut project_name: Option<String> = None;
    let mut report_name: Option<String> = None;
    let mut report_type: String = "allure".to_string();
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
                                if id > max_id { max_id = id; }
                            }
                        }
                    }
                }
            }
            next_id = max_id + 1;
        }
        Err(e) => { eprintln!("Error reading directory for ID generation: {}", e); }
    }

    let report_id = next_id.to_string();

    let mut report_dir = parent_dir.clone();
    report_dir.push(&report_id);

    let extract_dir = if report_type == "allure" {
        report_dir.join("allure-results")
    } else {
        report_dir.clone()
    };

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
            let raw_path = match file.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => continue,
            };

            let stripped_path = raw_path.components()
                .skip_while(|c| {
                    if let std::path::Component::Normal(s) = c {
                        s.to_string_lossy() == "allure-results"
                    } else {
                        false
                    }
                })
                .collect::<PathBuf>();

            let relative_path = if stripped_path.as_os_str().is_empty() {
                continue;
            } else {
                stripped_path
            };

            let outpath = target_dir.join(&relative_path);

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
        Ok(Ok(count)) => { println!("Extracted {} entries from zip", count); },
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Zip extraction failed: {}", e) }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Extraction panic: {}", e) }))).into_response(),
    }

    if report_type == "allure" {
        let actual_input_dir = find_results_dir(&extract_dir).await;
        println!("Resolved allure-results input dir: {:?}", actual_input_dir);

        let config_result = ensure_allure_config(&parent_dir, report_name.as_ref().unwrap()).await;
        
        if let Err(e) = config_result {
            eprintln!("Warning: Failed to create allurerc.json: {}", e);
        }

        let parent_history = parent_dir.join("history.jsonl");
        let input_history = actual_input_dir.join("history.jsonl");
        if tokio::fs::metadata(&parent_history).await.is_ok() {
            println!("Found existing history.jsonl, copying to input dir");
            if let Err(e) = tokio::fs::copy(&parent_history, &input_history).await {
                eprintln!("Warning: Failed to copy history.jsonl to input dir: {}", e);
            }
        }
        
        let abs_input_dir = std::fs::canonicalize(&actual_input_dir)
        .unwrap_or_else(|_| actual_input_dir.clone());
        let abs_output_dir = std::fs::canonicalize(&report_dir)
            .unwrap_or_else(|_| report_dir.clone());
        let abs_parent_dir = std::fs::canonicalize(&parent_dir)
        .unwrap_or_else(|_| parent_dir.clone());

        let input_dir_str = abs_input_dir.to_string_lossy().to_string();
        let output_dir_str = abs_output_dir.to_string_lossy().to_string();
        let parent_dir_str = abs_parent_dir.to_string_lossy().to_string();

        println!("Allure input (absolute): {}", input_dir_str);
        println!("Allure output (absolute): {}", output_dir_str);
        println!("Allure cwd (config dir): {}", parent_dir_str);

        let gen_result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let output = Command::new("npx")
            .arg("allure")
            .arg("generate")
            .arg(&input_dir_str)
            .arg("-o")
            .arg(&output_dir_str)
            .arg("--cwd")
            .arg(&parent_dir_str)  // ← Allure looks for allurerc.json here
            .output()
            .map_err(|e| format!("Failed to run npx command: {}", e))?;

            let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            println!("Allure stdout: {}", stdout_str);
            println!("Allure stderr: {}", stderr_str);

            if !output.status.success() {
                return Err(format!("Allure generation failed: {}", stderr_str));
            }
            Ok(stdout_str)
        }).await;

        match gen_result {
            Ok(Ok(msg)) => { println!("Allure generation succeeded: {}", msg); },
            Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Generation panic: {}", e) }))).into_response(),
        }

        let generated_history = actual_input_dir.join("history.jsonl");
        let parent_history = parent_dir.join("history.jsonl");
        if tokio::fs::metadata(&generated_history).await.is_ok() {
            println!("history.jsonl was created/updated. Copying back to parent dir.");
            if let Err(e) = tokio::fs::copy(&generated_history, &parent_history).await {
                eprintln!("Failed to copy history.jsonl back: {}", e);
            }
        } else {
            let output_history = report_dir.join("history.jsonl");
            if tokio::fs::metadata(&output_history).await.is_ok() {
                println!("history.jsonl found in output dir. Copying to parent.");
                if let Err(e) = tokio::fs::copy(&output_history, &parent_history).await {
                    eprintln!("Failed to copy history.jsonl from output: {}", e);
                }
            } else {
                eprintln!("WARNING: history.jsonl was NOT created anywhere. Allure CLI may not support this config option.");
            }
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

async fn find_results_dir(dir: &PathBuf) -> PathBuf {
    let mut current = dir.clone();

    for _ in 0..3 {
        let mut has_json = false;
        let mut single_subdir: Option<PathBuf> = None;
        let mut subdir_count = 0;
    
        if let Ok(mut entries) = tokio::fs::read_dir(&current).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "json" || ext == "xml" || ext == "txt" {
                            has_json = true;
                            break;
                        }
                    }
                } else if path.is_dir() {
                    subdir_count += 1;
                    if single_subdir.is_none() {
                        single_subdir = Some(path);
                    }
                }
            }
        }
    
        if has_json {
            println!("Found result files at: {:?}", current);
            return current;
        }
    
        if subdir_count == 1 && single_subdir.is_some() {
            println!("Descending into single subfolder: {:?}", single_subdir.as_ref().unwrap());
            current = single_subdir.unwrap();
        } else {
            break;
        }
    }

    println!("Could not find result files, using original dir: {:?}", dir);
    current
}