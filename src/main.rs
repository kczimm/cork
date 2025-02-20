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
    Build,
    /// Builds and runs the C project
    Run,
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

    println!("Created new project: {}", name);
    Ok(())
}

fn build_project() -> Result<(), String> {
    let src_dir = Path::new("src");
    let include_dir = Path::new("include");

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

    let output_executable = "project";

    let mut cmd = Command::new("gcc");
    cmd.arg("-o").arg(output_executable);

    for file in source_files {
        cmd.arg(file);
    }

    // Add include directory
    cmd.arg("-I").arg(include_dir);

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to execute gcc: {}", e))?;

    if !status.success() {
        return Err("Compilation failed".to_string());
    }

    println!("Build successful!");
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            if let Err(e) = create_new_project(&name) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Build => {
            if let Err(e) = build_project() {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Run => {
            println!("Running project...");
        }
    }
}
