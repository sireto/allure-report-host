use std::path::Path;
use std::process::Command;

/// Runs `npx allure generate` with the given input/output directories.
/// `config_dir` is used as the working directory so allurerc.json is found.
pub fn generate_report(
    input_dir: &Path,
    output_dir: &Path,
    config_dir: &Path,
) -> Result<String, String> {
    let input_str = input_dir.to_string_lossy().to_string();
    let output_str = output_dir.to_string_lossy().to_string();
    let config_str = config_dir.to_string_lossy().to_string();

    println!("Allure input (absolute): {}", input_str);
    println!("Allure output (absolute): {}", output_str);
    println!("Allure cwd (config dir): {}", config_str);

    let output = Command::new("npx")
        .arg("allure")
        .arg("generate")
        .arg(&input_str)
        .arg("-o")
        .arg(&output_str)
        .arg("--cwd")
        .arg(&config_str)
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
}

/// Copies history.jsonl from parent_dir into the input directory,
/// and after generation copies it back from wherever allure wrote it.
pub async fn sync_history(parent_dir: &Path, input_dir: &Path, _output_dir: &Path) {
    let parent_history = parent_dir.join("history.jsonl");
    let input_history = input_dir.join("history.jsonl");

    // Copy existing history into input dir before generation
    if tokio::fs::metadata(&parent_history).await.is_ok() {
        println!("Found existing history.jsonl, copying to input dir");
        if let Err(e) = tokio::fs::copy(&parent_history, &input_history).await {
            eprintln!("Warning: Failed to copy history.jsonl to input dir: {}", e);
        }
    }
}

pub async fn collect_history(parent_dir: &Path, input_dir: &Path, output_dir: &Path) {
    let parent_history = parent_dir.join("history.jsonl");

    // Check input dir first (allure may update in-place)
    let generated_history = input_dir.join("history.jsonl");
    if tokio::fs::metadata(&generated_history).await.is_ok() {
        println!("history.jsonl was created/updated. Copying back to parent dir.");
        if let Err(e) = tokio::fs::copy(&generated_history, &parent_history).await {
            eprintln!("Failed to copy history.jsonl back: {}", e);
        }
        return;
    }

    // Check output dir as fallback
    let output_history = output_dir.join("history.jsonl");
    if tokio::fs::metadata(&output_history).await.is_ok() {
        println!("history.jsonl found in output dir. Copying to parent.");
        if let Err(e) = tokio::fs::copy(&output_history, &parent_history).await {
            eprintln!("Failed to copy history.jsonl from output: {}", e);
        }
        return;
    }

    eprintln!(
        "WARNING: history.jsonl was NOT created anywhere. Allure CLI may not support this config option."
    );
}
