use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::{IntoIter, WalkDir};

use crate::cli::Profile;

fn is_python(name: &str) -> bool {
    matches!(
        name,
        ".venv"
            | "venv"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | ".ipynb_checkpoints"
            | ".coverage"
    )
}

fn is_rust(name: &str) -> bool {
    name == "target"
}

fn is_js(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "dist" | "build" | ".next" | ".svelte-kit"
    )
}

fn should_remove(path: &Path, profile: Profile) -> bool {
    let Some(name) = path.file_name().and_then(|x| x.to_str()) else {
        return false;
    };

    match profile {
        Profile::All => is_python(name) || is_rust(name) || is_js(name),
        Profile::Python => is_python(name),
        Profile::Rust => is_rust(name),
        Profile::Js => is_js(name),
    }
}

fn path_size(path: &Path) -> Result<u64> {
    let metadata = path.metadata()?;

    if !metadata.is_dir() {
        return Ok(metadata.len());
    }

    let mut size = 0;
    for entry in WalkDir::new(path) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_dir() {
            size += metadata.len();
        }
    }

    Ok(size)
}

pub struct Match {
    pub path: PathBuf,
    pub size: u64,
}

pub struct MatchList {
    matches: Vec<Match>,
}

impl MatchList {
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
        }
    }

    pub fn push(&mut self, mtch: Match) {
        self.matches.push(mtch);
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Match> {
        self.matches.iter()
    }

    pub fn total_size(&self) -> u64 {
        self.matches.iter().map(|mtch| mtch.size).sum()
    }
}

pub struct Scanner {
    walker: IntoIter,
    profile: Profile,
    pending_size: Option<PathBuf>,
}

impl Scanner {
    pub fn new(root: &Path, max_depth: Option<usize>, profile: Profile) -> Self {
        let mut walker = WalkDir::new(root);

        if let Some(max_depth) = max_depth {
            walker = walker.max_depth(max_depth);
        }

        Self {
            walker: walker.into_iter(),
            profile,
            pending_size: None,
        }
    }
}

pub enum ScanEvent {
    Visited,
    Sizing(PathBuf),
    Match(Match),
}

impl Iterator for Scanner {
    type Item = Result<ScanEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        // Handle any pending size calculation
        if let Some(path) = self.pending_size.take() {
            let size = match path_size(&path) {
                Ok(size) => size,
                Err(error) => return Some(Err(error)),
            };

            return Some(Ok(ScanEvent::Match(Match { path, size })));
        }

        let entry = match self.walker.next()? {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error.into())),
        };

        let path = entry.path();
        if !should_remove(path, self.profile) {
            return Some(Ok(ScanEvent::Visited));
        }

        let path = path.to_path_buf();
        if entry.file_type().is_dir() {
            self.walker.skip_current_dir();
        }

        self.pending_size = Some(path.clone());

        Some(Ok(ScanEvent::Sizing(path)))
    }
}

#[allow(dead_code)]
pub fn scan(root: &Path, max_depth: Option<usize>, profile: Profile) -> Result<MatchList> {
    let mut matches = MatchList::new();

    for event in Scanner::new(root, max_depth, profile) {
        if let ScanEvent::Match(mtch) = event? {
            matches.push(mtch);
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn relative_paths(matches: &MatchList, root: &Path) -> Vec<String> {
        let mut paths = matches
            .iter()
            .map(|mtch| {
                mtch.path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    #[test]
    fn scan_reports_top_level_match_size_without_nested_matches() {
        let temp = TempDir::new().unwrap();
        write_file(
            &temp.path().join("node_modules/pkg/__pycache__/cache.pyc"),
            "abc",
        );
        write_file(&temp.path().join("node_modules/pkg/file.js"), "de");

        let matches = scan(temp.path(), None, Profile::All).unwrap();

        assert_eq!(relative_paths(&matches, temp.path()), vec!["node_modules"]);
        assert_eq!(matches.total_size(), 5);
    }

    #[test]
    fn scan_handles_file_targets() {
        let temp = TempDir::new().unwrap();
        write_file(&temp.path().join(".coverage"), "coverage");

        let matches = scan(temp.path(), None, Profile::Python).unwrap();

        assert_eq!(relative_paths(&matches, temp.path()), vec![".coverage"]);
        assert_eq!(matches.total_size(), 8);
    }

    #[test]
    fn scan_respects_profile_and_max_depth() {
        let temp = TempDir::new().unwrap();
        write_file(&temp.path().join("project/target/debug/app"), "rust");
        write_file(&temp.path().join("project/node_modules/pkg/index.js"), "js");

        let rust_matches = scan(temp.path(), None, Profile::Rust).unwrap();
        assert_eq!(
            relative_paths(&rust_matches, temp.path()),
            vec!["project/target"]
        );

        let shallow_matches = scan(temp.path(), Some(1), Profile::Rust).unwrap();
        assert!(shallow_matches.is_empty());
    }
}
