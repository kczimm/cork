use clap::{Parser, Subcommand};
use fs_extra::dir::create_all;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    let build_dir = Path::new("build");
    let build_subdir = if release { "release" } else { "debug" };
    let executable_path = build_dir.join(build_subdir).join("project");

    if !executable_path.exists() {
        build_project(release)?; // Build the project if not built
    }

    let status = Command::new(executable_path)
        .status()
        .map_err(|e| format!("Failed to run the project: {}", e))?;

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
        return Err(format!("Directory '{}' already exists!", name));
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
name = "{}"
version = "0.1.0"
"#,
        name
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
        .status()
        .map_err(|e| format!("Failed to initialize git repository: {}", e))?;

    if !status.success() {
        return Err("Failed to initialize Git repository".to_string());
    }

    println!("Created new project: {}", name);
    Ok(())
}

fn build_project(release: bool) -> Result<(), String> {
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

    // Create build directories if they don't exist
    create_all(build_dir.join(build_subdir), true)
        .map_err(|e| format!("Failed to create build directory: {}", e))?;

    // Collect all .c files in src directory
    let source_files: Vec<_> = fs::read_dir(src_dir)
        .map_err(|e| format!("Failed to read src directory: {}", e))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("c"))
        .map(|entry| entry.path())
        .collect();

    if source_files.is_empty() {
        return Err("No source files found in src directory!".to_string());
    }

    let output_executable = build_dir.join(build_subdir).join("project");

    let mut cmd = Command::new("gcc");
    cmd.arg("-o").arg(&output_executable);

    for file in source_files {
        cmd.arg(file);
    }

    // Add include directory
    cmd.arg("-I").arg(include_dir);

    // Add optimization flag for release builds
    if release {
        cmd.arg("-O3");
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to execute gcc: {}", e))?;

    if !status.success() {
        return Err("Compilation failed".to_string());
    }

    Ok(())
}

fn clean_project() -> Result<(), String> {
    let build_dir = Path::new("build");

    if build_dir.exists() {
        fs::remove_dir_all(build_dir)
            .map_err(|e| format!("Failed to clean build directory: {}", e))?;
        println!("Cleaned build directory.");
    } else {
        println!("Build directory does not exist. Nothing to clean.");
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name } => create_new_project(&name),
        Commands::Build { release } => build_project(release),
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
