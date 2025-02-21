use fs_extra::dir::create_all;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_project(release: bool) -> Result<PathBuf, String> {
    let cork_toml_path = Path::new("Cork.toml");

    if !cork_toml_path.exists() {
        return Err(format!(
            "error: could not find `Cork.toml` in `{}`",
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    }

    let src_dir = Path::new("src");
    let include_dir = Path::new("include");
    let build_dir = Path::new("build");
    let build_subdir = if release { "release" } else { "debug" };
    let obj_dir = build_dir.join(build_subdir).join("obj");
    let output_executable = build_dir.join(build_subdir).join("project");

    create_all(&obj_dir, true).map_err(|e| format!("Failed to create obj directory: {e}"))?;

    let source_files: Vec<_> = fs::read_dir(src_dir)
        .map_err(|e| format!("Failed to read src directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("c"))
        .map(|entry| entry.path())
        .collect();

    if source_files.is_empty() {
        return Err("No source files found in src directory!".to_string());
    }

    let header_files: Vec<_> = fs::read_dir(include_dir)
        .map_err(|e| format!("Failed to read include directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("h"))
        .map(|entry| entry.path())
        .collect();

    let mut objects_to_link = Vec::new();
    let mut needs_link = !output_executable.exists();

    for src_file in &source_files {
        let obj_file = obj_dir.join(
            src_file
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".c", ".o"),
        );
        objects_to_link.push(obj_file.clone());

        let src_time = src_file.metadata().and_then(|m| m.modified()).ok();
        let obj_time = obj_file.metadata().and_then(|m| m.modified()).ok();

        let needs_compile = src_time.map_or(true, |st| {
            obj_time.map_or(true, |ot| {
                st > ot
                    || header_files.iter().any(|h| {
                        h.metadata()
                            .and_then(|m| m.modified())
                            .map(|ht| ht > ot)
                            .unwrap_or(true)
                    })
            })
        });

        if needs_compile {
            let mut cmd = Command::new("gcc");
            cmd.arg("-c")
                .arg(src_file)
                .arg("-o")
                .arg(&obj_file)
                .arg("-I")
                .arg(include_dir);

            if release {
                cmd.arg("-O3");
            }

            let output = cmd
                .output()
                .map_err(|e| format!("Failed to compile {src_file:?}: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(stderr.to_string());
            }
            needs_link = true;
        }
    }

    if needs_link {
        let mut cmd = Command::new("gcc");
        cmd.arg("-o").arg(&output_executable);
        for obj in &objects_to_link {
            cmd.arg(obj);
        }

        let output = cmd.output().map_err(|e| format!("Failed to link: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Linking failed:\n{stderr}"));
        }
    }

    Ok(output_executable)
}

pub fn run_project(release: bool) -> Result<(), String> {
    let executable_path = build_project(release)?;

    let status = Command::new(executable_path)
        .status()
        .map_err(|e| format!("Failed to run the project: {e}"))?;

    if !status.success() {
        return Err(format!(
            "Project execution failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}
