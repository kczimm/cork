use colored::Colorize;
use fs_extra::dir::create_all;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Deserialize)]
pub struct CorkConfig {
    pub project: ProjectConfig,
    #[serde(default)] // Empty map if no dependencies section
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct Dependency {
    pub path: String, // For now, only local paths; can extend to Git later
}

pub fn create_new_project(name: &str) -> Result<(), String> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!(
            "{}: destination `{name}` already exists",
            "error".red()
        ));
    }

    create_all(project_dir, false).map_err(|e| e.to_string())?;
    create_all(project_dir.join("src"), true).map_err(|e| e.to_string())?;
    create_all(project_dir.join("src/include"), true).map_err(|e| e.to_string())?;
    create_all(project_dir.join("include"), true).map_err(|e| e.to_string())?;
    create_all(project_dir.join("tests"), true).map_err(|e| e.to_string())?;

    let main_c = r#"#include <stdio.h>
#include "headers.h"

int main() {
    printf("Hello, Cork!\n");
    return 0;
}
"#;
    fs::write(project_dir.join("src/main.c"), main_c).map_err(|e| e.to_string())?;

    let headers_h = r#"#ifndef HEADERS_H
#define HEADERS_H

void some_function(void);

#endif // HEADERS_H
"#;
    fs::write(project_dir.join("include/headers.h"), headers_h).map_err(|e| e.to_string())?;

    let test_main_c = r#"#include <stdio.h>
#include "headers.h"

int main() {
    printf("Running tests\n");
    return 0;
}
"#;
    fs::write(project_dir.join("tests/test_main.c"), test_main_c).map_err(|e| e.to_string())?;

    let cork_toml = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"

[dependencies]
"#
    );
    fs::write(project_dir.join("Cork.toml"), &cork_toml).map_err(|e| e.to_string())?;

    let gitignore_content = r#"build/
"#;
    fs::write(project_dir.join(".gitignore"), gitignore_content).map_err(|e| e.to_string())?;

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
