use clap::Parser;

mod build;
mod clean;
mod cli;
mod project;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name } => project::create_new_project(&name),
        Commands::Build { release } => build::build_project(release).map(|_| ()),
        Commands::Run { release } => build::run_project(release),
        Commands::Clean => clean::clean_project(),
    };

    if let Err(e) = result {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write, path::Path};
    use tempfile::tempdir;

    #[test]
    fn test_create_new_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "test_project";
        let project_path = temp_dir.path().join(test_project_name);

        let result = project::create_new_project(&project_path.to_string_lossy());
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

        project::create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for build test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        let result = build::build_project(false);
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

        project::create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        build::build_project(false).expect("Initial build failed");
        let initial_obj_time = fs::metadata("build/debug/obj/main.o")
            .and_then(|m| m.modified())
            .expect("Failed to get initial obj time");
        let initial_exe_time = fs::metadata("build/debug/project")
            .and_then(|m| m.modified())
            .expect("Failed to get initial exe time");

        build::build_project(false).expect("Second build failed");
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

        let main_c_path = Path::new("src/main.c");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(main_c_path)
            .expect("Failed to open main.c");
        writeln!(file, "\n// Modified").expect("Failed to modify main.c");

        build::build_project(false).expect("Third build failed");
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

        project::create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for run test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        build::build_project(false).expect("Failed to build project for run test");

        let result = build::run_project(false);
        assert!(result.is_ok(), "Run failed: {:?}", result);
    }

    #[test]
    fn test_clean_project() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_project_name = "clean_test";
        let project_path = temp_dir.path().join(test_project_name);

        project::create_new_project(&project_path.to_string_lossy())
            .expect("Failed to create project for clean test");
        std::env::set_current_dir(&project_path).expect("Failed to change to project directory");

        build::build_project(false).expect("Failed to build project for clean test");

        assert!(
            Path::new("build").exists(),
            "Build directory should exist before cleaning"
        );

        clean::clean_project().expect("Clean failed");

        assert!(
            !Path::new("build").exists(),
            "Build directory should not exist after cleaning"
        );
    }
}
