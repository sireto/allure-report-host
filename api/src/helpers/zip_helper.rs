use std::path::PathBuf;
use std::io::Cursor;
use zip::ZipArchive;

/// Extracts a zip archive, stripping a common top-level directory if all entries share one.
pub fn extract_zip(zip_bytes: Vec<u8>, target_dir: PathBuf) -> Result<usize, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;

    // Detect common top-level directory
    let common_prefix = detect_common_prefix(&mut archive)?;
    println!("Zip common prefix: {:?}", common_prefix);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let raw_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        // Strip only the first component if it matches the common prefix
        let stripped_path = if let Some(ref prefix) = common_prefix {
            if let Some(first_component) = raw_path.components().next() {
                if let std::path::Component::Normal(s) = first_component {
                    if s.to_string_lossy() == *prefix {
                        // Remove only the first component
                        raw_path.components().skip(1).collect::<PathBuf>()
                    } else {
                        raw_path.clone()
                    }
                } else {
                    raw_path.clone()
                }
            } else {
                raw_path.clone()
            }
        } else {
            raw_path.clone()
        };

        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let outpath = target_dir.join(&stripped_path);

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    Ok(archive.len())
}

fn detect_common_prefix(archive: &mut ZipArchive<Cursor<Vec<u8>>>) -> Result<Option<String>, String> {
    let mut prefix: Option<String> = None;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        
        // Get the first path component only
        let first_component = name.split('/').next().unwrap_or("").to_string();

        // Skip empty components (from trailing slashes or root)
        if first_component.is_empty() {
            continue;
        }

        match &prefix {
            None => prefix = Some(first_component),
            Some(p) => {
                if *p != first_component {
                    return Ok(None);
                }
            }
        }
    }

    Ok(prefix)
}