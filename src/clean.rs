use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn clean_project() -> Result<(), String> {
    let build_dir = Path::new("build");

    if !build_dir.exists() {
        println!("Build directory does not exist. Nothing to clean.");
        return Ok(());
    }

    let mut total_size: u64 = 0;
    let mut file_count: u64 = 0;

    for entry in WalkDir::new(build_dir) {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_file() {
            match fs::metadata(path) {
                Ok(metadata) => {
                    total_size += metadata.len();
                    file_count += 1;
                }
                Err(e) => eprintln!("Failed to get metadata for {}: {e}", path.display()),
            }
        }
    }

    fs::remove_dir_all(build_dir).map_err(|e| format!("Failed to clean build directory: {e}"))?;

    let size_in_mib = total_size as f64 / (1024.0 * 1024.0);
    println!("Removed {file_count} files, {size_in_mib:.1}MiB total");

    Ok(())
}
