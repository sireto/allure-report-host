use std::path::{Path, PathBuf};
use tokio::fs;

/// Recursively finds the directory containing allure result JSON files.
/// Handles cases where zip contains nested folders like allure-results/allure-results/*.json
pub async fn find_results_dir(dir: &PathBuf) -> PathBuf {
    let mut current = dir.clone();

    for _ in 0..3 {
        let mut has_json = false;
        let mut single_subdir: Option<PathBuf> = None;
        let mut subdir_count = 0;

        if let Ok(mut entries) = tokio::fs::read_dir(&current).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension()
                        && (ext == "json" || ext == "xml" || ext == "txt")
                    {
                        has_json = true;
                        break;
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

        if subdir_count == 1 {
            if let Some(subdir) = single_subdir {
                println!("Descending into single subfolder: {:?}", &subdir);
                current = subdir;
            } else {
                unreachable!("subdir_count == 1 but single_subdir is None");
            }
        } else {
            break;
        }
    }

    println!("Could not find result files, using original dir: {:?}", dir);
    current
}

/// Finds the next sequential ID by scanning existing numbered directories.
pub async fn next_sequential_id(parent_dir: &PathBuf) -> u32 {
    let mut max_id: u32 = 0;

    match tokio::fs::read_dir(parent_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(ft) = entry.file_type().await
                    && ft.is_dir()
                    && let Some(name) = entry.file_name().to_str()
                    && let Ok(id) = name.parse::<u32>()
                    && id > max_id
                {
                    max_id = id;
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading directory for ID generation: {}", e);
        }
    }

    max_id + 1
}

/// Moves all contents from source directory to destination directory
pub async fn move_directory_contents(source: &Path, dest: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(source)
        .await
        .map_err(|e| format!("Failed to read source directory: {}", e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read entry: {}", e))?
    {
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;
        let dest_path = dest.join(file_name);

        if dest_path.exists() {
            if dest_path.is_dir() {
                fs::remove_dir_all(&dest_path).await.map_err(|e| {
                    format!("Failed to remove existing directory {:?}: {}", dest_path, e)
                })?;
            } else {
                fs::remove_file(&dest_path).await.map_err(|e| {
                    format!("Failed to remove existing file {:?}: {}", dest_path, e)
                })?;
            }
        }

        fs::rename(&source_path, &dest_path)
            .await
            .map_err(|e| format!("Failed to move {:?} to {:?}: {}", source_path, dest_path, e))?;

        println!("Moved {:?} to {:?}", source_path, dest_path);
    }

    Ok(())
}

/// Validates a single path segment to prevent traversal and unsafe chars.
pub fn validate_path_segment(input: &str, field: &str) -> Result<String, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(format!("{} cannot be empty or whitespace", field));
    }

    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("{} must not contain path separators", field));
    }

    if trimmed.contains("..") {
        return Err(format!("{} must not contain '..'", field));
    }

    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';
    if !trimmed.chars().all(allowed) {
        return Err(format!(
            "{} contains invalid characters. Allowed: a-z, A-Z, 0-9, '.', '_', '-'",
            field
        ));
    }

    Ok(trimmed.to_string())
}

/// Atomically allocates the next sequential ID by creating the directory.
/// Retries on conflict to avoid race conditions.
pub async fn allocate_next_id_dir(parent_dir: &PathBuf) -> Result<(u32, PathBuf), String> {
    loop {
        let next_id = next_sequential_id(parent_dir).await;
        let dir = parent_dir.join(next_id.to_string());

        match fs::create_dir(&dir).await {
            Ok(_) => return Ok((next_id, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Another upload won the race; retry
                continue;
            }
            Err(e) => return Err(format!("Failed to create report directory: {}", e)),
        }
    }
}
