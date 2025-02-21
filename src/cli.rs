use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cork",
    about = "A build tool for C projects",
    version = "0.1.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
