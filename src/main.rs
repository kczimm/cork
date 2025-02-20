use clap::{Parser, Subcommand};
use fs_extra::dir::create_all;
use std::fs;
use std::path::Path;

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

    // Create project directory
    create_all(project_dir, false).map_err(|e| e.to_string())?;

    // Write main.c
    let main_c = r#"#include <stdio.h>

int main() {
    printf("Hello, Cork!\n");
    return 0;
}
"#;
    fs::write(project_dir.join("main.c"), main_c).map_err(|e| e.to_string())?;

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            if let Err(e) = create_new_project(&name) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Build => {
            println!("Building project...");
        }
        Commands::Run => {
            println!("Running project...");
        }
    }
}
