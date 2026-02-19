use std::path::PathBuf;
use serde_json::json;

pub async fn ensure_allure_config(parent_dir: &PathBuf, _report_name: &str) -> Result<PathBuf, String> {
    let config_path = parent_dir.join("allurerc.json");

    let abs_parent = std::fs::canonicalize(parent_dir)
        .unwrap_or_else(|_| parent_dir.clone());
    let history_abs = abs_parent.join("history.jsonl");

    let config_content = json!({
        "historyPath": history_abs.to_string_lossy(),
        "appendHistory": true
    });

    let config_json = serde_json::to_string_pretty(&config_content)
        .map_err(|e| format!("Failed to serialize allure config: {}", e))?;

    tokio::fs::write(&config_path, config_json)
        .await
        .map_err(|e| format!("Failed to write allurerc.json: {}", e))?;

    println!("Wrote allurerc.json at {:?} with historyPath={:?}", config_path, history_abs);
    Ok(config_path)
}