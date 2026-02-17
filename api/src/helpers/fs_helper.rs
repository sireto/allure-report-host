use std::path::PathBuf;

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

/// Finds the next sequential ID by scanning existing numbered directories.
pub async fn next_sequential_id(parent_dir: &PathBuf) -> u32 {
    let mut max_id: u32 = 0;

    match tokio::fs::read_dir(parent_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
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
        }
        Err(e) => {
            eprintln!("Error reading directory for ID generation: {}", e);
        }
    }

    max_id + 1
}