use colored::Colorize;
use fs_extra::dir::create_all;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::project::CorkConfig;

pub fn build_project(release: bool) -> Result<PathBuf, String> {
    let cork_toml_path = Path::new("Cork.toml");

    if !cork_toml_path.exists() {
        return Err(format!(
            "{}: could not find `Cork.toml` in `{}`",
            "error".red(),
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    }

    // Parse Cork.toml
    let config_content =
        fs::read_to_string(cork_toml_path).map_err(|e| format!("Failed to read Cork.toml: {e}"))?;
    let config: CorkConfig =
        toml::from_str(&config_content).map_err(|e| format!("Failed to parse Cork.toml: {e}"))?;

    let src_dir = Path::new("src");
    let private_include_dir = Path::new("src/include");
    let public_include_dir = Path::new("include");
    let build_dir = Path::new("build");
    let build_subdir = if release { "release" } else { "debug" };
    let obj_dir = build_dir.join(build_subdir).join("obj");
    let output_executable = build_dir.join(build_subdir).join("project");

    create_all(&obj_dir, true).map_err(|e| format!("Failed to create obj directory: {e}"))?;

    // Collect include dirs (start with project's own)
    let mut include_dirs = vec![
        public_include_dir.to_owned(),
        private_include_dir.to_owned(),
    ];
    let mut all_objects_to_link = Vec::new();

    // Build dependencies
    for (dep_name, dep) in &config.dependencies {
        let dep_path = Path::new(&dep.path);
        let dep_cork_toml = dep_path.join("Cork.toml");
        if !dep_cork_toml.exists() {
            return Err(format!(
                "error: dependency `{dep_name}` missing Cork.toml at `{}`",
                dep_path.display()
            ));
        }

        let dep_src_dir = dep_path.join("src");
        let dep_public_include_dir = dep_path.join("include");
        let dep_obj_dir = dep_path.join("build").join(build_subdir).join("obj");

        create_all(&dep_obj_dir, true)
            .map_err(|e| format!("Failed to create dependency obj directory: {e}"))?;

        let dep_source_files: Vec<_> = fs::read_dir(&dep_src_dir)
            .map_err(|e| format!("Failed to read dependency src directory: {e}"))?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("c"))
            .map(|entry| entry.path())
            .collect();

        let dep_header_files: Vec<_> = fs::read_dir(&dep_public_include_dir)
            .map_err(|e| format!("Failed to read dependency include directory: {e}"))?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("h"))
            .map(|entry| entry.path())
            .collect();

        for dep_src_file in &dep_source_files {
            let dep_obj_file = dep_obj_dir.join(
                dep_src_file
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .replace(".c", ".o"),
            );
            all_objects_to_link.push(dep_obj_file.clone());

            let src_time = dep_src_file.metadata().and_then(|m| m.modified()).ok();
            let obj_time = dep_obj_file.metadata().and_then(|m| m.modified()).ok();

            let needs_compile = src_time.map_or(true, |st| {
                obj_time.map_or(true, |ot| {
                    st > ot
                        || dep_header_files.iter().any(|h| {
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
                    .arg(dep_src_file)
                    .arg("-o")
                    .arg(&dep_obj_file)
                    .arg("-I")
                    .arg(&dep_public_include_dir); // Dependency’s public headers

                if release {
                    cmd.arg("-O3");
                }

                let output = cmd
                    .output()
                    .map_err(|e| format!("Failed to compile dependency {dep_src_file:?}: {e}"))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!(
                        "Compilation failed for {dep_src_file:?}:\n{stderr}"
                    ));
                }
            }
        }
        include_dirs.push(dep_public_include_dir); // Add dependency’s public headers to include path
    }

    // Build main project
    let source_files: Vec<_> = fs::read_dir(src_dir)
        .map_err(|e| format!("Failed to read src directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("c"))
        .map(|entry| entry.path())
        .collect();

    if source_files.is_empty() {
        return Err("No source files found in src directory!".to_string());
    }

    let public_headers: Vec<_> = fs::read_dir(public_include_dir)
        .map_err(|e| format!("Failed to read include directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("h"))
        .map(|entry| entry.path())
        .collect();

    let private_headers: Vec<_> = fs::read_dir(private_include_dir)
        .map_err(|e| format!("Failed to read src/include directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("h"))
        .map(|entry| entry.path())
        .collect();

    let mut needs_link = !output_executable.exists();

    for src_file in &source_files {
        let obj_file = obj_dir.join(
            src_file
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".c", ".o"),
        );
        all_objects_to_link.push(obj_file.clone());

        let src_time = src_file.metadata().and_then(|m| m.modified()).ok();
        let obj_time = obj_file.metadata().and_then(|m| m.modified()).ok();

        let needs_compile = src_time.map_or(true, |st| {
            obj_time.map_or(true, |ot| {
                st > ot
                    || public_headers
                        .iter()
                        .chain(private_headers.iter())
                        .any(|h| {
                            h.metadata()
                                .and_then(|m| m.modified())
                                .map(|ht| ht > ot)
                                .unwrap_or(true)
                        })
            })
        });

        if needs_compile {
            let mut cmd = Command::new("gcc");
            cmd.arg("-c").arg(src_file).arg("-o").arg(&obj_file);
            for inc in &include_dirs {
                cmd.arg("-I").arg(inc); // Include all public headers (own + dependencies)
            }

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
        for obj in &all_objects_to_link {
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
