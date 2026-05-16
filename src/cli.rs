use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "ggober")]
#[command(about = "Clean removable build and cache artifacts from code folders")]
pub struct Cli {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Actually delete files. Without this, sweep only performs a dry run.
    #[arg(long)]
    pub delete: bool,

    /// Do not ask for confirmation when deleting
    #[arg(long, visible_alias = "yes")]
    pub auto_approve: bool,

    /// Cleaning profile to use
    #[arg(long, value_enum, default_value = "all")]
    pub profile: Profile,

    /// Maximum directory depth to scan
    #[arg(long)]
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Profile {
    All,
    Python,
    Rust,
    Js,
}
