use clap::{Parser, Subcommand};
use colored::Colorize;
use fs_extra::dir::create_all;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "cork",
    about = "A build tool for C projects",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates a new C project
    New { name: String },
    /// Builds the C project
    #[command(alias = "b")]
    Build {
        #[arg(long)]
        release: bool,
    },
    /// Builds and runs the C project
    #[command(alias = "r")]
    Run {
        #[arg(long)]
        release: bool,
    },
    /// Cleans the build directory
    Clean,
}

fn run_project(release: bool) -> Result<(), String> {
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

fn create_new_project(name: &str) -> Result<(), String> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!(
            "{}: destination `{name}` already exists",
            "error".red()
        ));
    }

    create_all(project_dir, false).map_err(|e| e.to_string())?;

    // Create directories
    create_all(project_dir.join("src"), true).map_err(|e| e.to_string())?;
    create_all(project_dir.join("include"), true).map_err(|e| e.to_string())?;
    create_all(project_dir.join("tests"), true).map_err(|e| e.to_string())?;

    // Write src/main.c
    let main_c = r#"#include <stdio.h>
#include "../include/headers.h"

int main() {
    printf("Hello, Cork!\n");
    return 0;
}
"#;
    fs::write(project_dir.join("src/main.c"), main_c).map_err(|e| e.to_string())?;

    // Write include/headers.h
    let headers_h = r#"#ifndef HEADERS_H
#define HEADERS_H

void some_function(void);

#endif // HEADERS_H
"#;
    fs::write(project_dir.join("include/headers.h"), headers_h).map_err(|e| e.to_string())?;

    // Write tests/test_main.c
    let test_main_c = r#"#include <stdio.h>
#include "../include/headers.h"

int main() {
    printf("Running tests\n");
    return 0;
}
"#;
    fs::write(project_dir.join("tests/test_main.c"), test_main_c).map_err(|e| e.to_string())?;

    // Write Cork.toml
    let cork_toml = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
"#
    );
    fs::write(project_dir.join("Cork.toml"), cork_toml).map_err(|e| e.to_string())?;

    // Write .gitignore
    let gitignore_content = r#"build/
"#;
    fs::write(project_dir.join(".gitignore"), gitignore_content).map_err(|e| e.to_string())?;

    // Initialize Git repository
    let status = Command::new("git")
        .current_dir(project_dir)
        .arg("init")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to initialize git repository: {e}"))?;

    if !status.success() {
        return Err("Failed to initialize Git repository".to_string());
    }

    println!("   {} project `{name}`", "Creating".green());

    Ok(())
}

fn build_project(release: bool) -> Result<PathBuf, String> {
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

    // Create directories
    create_all(&obj_dir, true).map_err(|e| format!("Failed to create obj directory: {e}"))?;

    // Collect all .c files in src directory
    let source_files: Vec<_> = fs::read_dir(src_dir)
        .map_err(|e| format!("Failed to read src directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("c"))
        .map(|entry| entry.path())
        .collect();

    if source_files.is_empty() {
        return Err("No source files found in src directory!".to_string());
    }

    // Collect all .h files in include directory (for dependency checking)
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

        // Check if source or any header is newer than the object file
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
            cmd.arg("-c") // Compile only, no linking
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
            needs_link = true; // New object file means we need to link
        }
    }

    // Link if necessary
    if needs_link {
        let mut cmd = Command::new("gcc");
        cmd.arg("-o").arg(&output_executable);
        for obj in &objects_to_link {
            cmd.arg(obj);
        }

        cmd.output().map_err(|e| format!("Failed to link: {e}"))?;

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to execute gcc: {e}"))?;

        if !status.success() {
            return Err("Compilation failed".to_string());
        }
    }

    Ok(output_executable)
}

fn clean_project() -> Result<(), String> {
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

    // Convert size to MiB
    let size_in_mib = total_size as f64 / (1024.0 * 1024.0);
    println!("Removed {file_count} files, {size_in_mib:.1}MiB total");

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name } => create_new_project(&name),
        Commands::Build { release } => build_project(release).map(|_| ()),
        Commands::Run { release } => run_project(release),
        Commands::Clean => clean_project(),
    };

    if let Err(e) = result {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn test_create_new_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "test_project";
        let project_path = temp_dir.path().join(test_project_name);

        let result = create_new_project(&project_path.to_string_lossy());
        assert!(result.is_ok(), "Failed to create new project: {:?}", result);

        assert!(project_path.exists(), "Project directory was not created");
        assert!(project_path.join("src/main.c").exists());
        assert!(project_path.join("include/headers.h").exists());
        assert!(project_path.join("tests/test_main.c").exists());
        assert!(project_path.join("Cork.toml").exists());
        assert!(project_path.join(".gitignore").exists());
    }

    #[test]
    fn test_build_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "build_test";
        let project_path = temp_dir.path().join(test_project_name);

        create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for build test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        let result = build_project(false); // Debug build
        assert!(result.is_ok(), "Build failed: {:?}", result);

        assert!(
            Path::new("build/debug/project").exists(),
            "Executable not created"
        );
        assert!(
            Path::new("build/debug/obj/main.o").exists(),
            "Object file not created"
        );
    }

    #[test]
    fn test_build_only_if_changed() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "change_test";
        let project_path = temp_dir.path().join(test_project_name);

        create_new_project(&project_path.to_string_lossy()).expect("Failed to create project");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        // First build
        build_project(false).expect("Initial build failed");
        let initial_obj_time = fs::metadata("build/debug/obj/main.o")
            .and_then(|m| m.modified())
            .expect("Failed to get initial obj time");
        let initial_exe_time = fs::metadata("build/debug/project")
            .and_then(|m| m.modified())
            .expect("Failed to get initial exe time");

        // Second build (no changes)
        build_project(false).expect("Second build failed");
        let second_obj_time = fs::metadata("build/debug/obj/main.o")
            .and_then(|m| m.modified())
            .expect("Failed to get second obj time");
        let second_exe_time = fs::metadata("build/debug/project")
            .and_then(|m| m.modified())
            .expect("Failed to get second exe time");
        assert_eq!(
            initial_obj_time, second_obj_time,
            "Object should not recompile"
        );
        assert_eq!(
            initial_exe_time, second_exe_time,
            "Executable should not relink"
        );

        // Modify a source file
        let main_c_path = Path::new("src/main.c");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(main_c_path)
            .expect("Failed to open main.c");
        use std::io::Write;
        writeln!(file, "\n// Modified").expect("Failed to modify main.c");

        // Third build (after change)
        build_project(false).expect("Third build failed");
        let third_obj_time = fs::metadata("build/debug/obj/main.o")
            .and_then(|m| m.modified())
            .expect("Failed to get third obj time");
        let third_exe_time = fs::metadata("build/debug/project")
            .and_then(|m| m.modified())
            .expect("Failed to get third exe time");
        assert!(third_obj_time > second_obj_time, "Object should recompile");
        assert!(third_exe_time > second_exe_time, "Executable should relink");
    }

    #[test]
    fn test_run_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "run_test";
        let project_path = temp_dir.path().join(test_project_name);

        create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for run test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        // Build the project first
        build_project(false).expect("Failed to build project for run test");

        let result = run_project(false); // Debug run
        assert!(result.is_ok(), "Run failed: {:?}", result);
    }

    #[test]
    fn test_clean_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "clean_test";
        let project_path = temp_dir.path().join(test_project_name);

        create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for clean test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        // Build to create some artifacts
        build_project(false).expect("Failed to build project for clean test");

        assert!(
            Path::new("build").exists(),
            "Build directory should exist before cleaning"
        );

        clean_project().expect("Clean failed");

        assert!(
            !Path::new("build").exists(),
            "Build directory should not exist after cleaning"
        );
    }
}
