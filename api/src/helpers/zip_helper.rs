use std::path::PathBuf;
use std::io::Cursor;
use zip::ZipArchive;

/// Extracts a zip archive, stripping a common top-level directory if all entries share one.
pub fn extract_zip(zip_bytes: Vec<u8>, target_dir: PathBuf) -> Result<usize, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;

    // Detect common top-level directory
    let common_prefix = detect_common_prefix(&mut archive)?;
    let strip_prefix = common_prefix.unwrap_or_default();
    println!("Zip strip prefix: '{}'", strip_prefix);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let raw_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        let stripped_path = raw_path
            .components()
            .skip_while(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_string_lossy() == strip_prefix
                } else {
                    false
                }
            })
            .collect::<PathBuf>();

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
        let first_component = name.split('/').next().unwrap_or("").to_string();

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