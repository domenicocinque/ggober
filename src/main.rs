mod cli;
mod scanner;

use anyhow::Result;
use clap::Parser;
use console::Term;
use console::style;
use dialoguer::Confirm;
use std::fs;
use std::path::Path;

use cli::Cli;
use scanner::{Match, MatchList, ScanEvent, Scanner};

const PROGRESS_UPDATE_INTERVAL: usize = 100;

fn bytes_to_gigabytes(bytes: u64) -> f64 {
    bytes as f64 / 1_000_000_000.0
}

fn remove_path(path: &Path) -> Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn confirm_delete(mtch: &Match) -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt(format!(
            "Delete {} ({:.2} GB)?",
            mtch.path.display(),
            bytes_to_gigabytes(mtch.size)
        ))
        .default(false)
        .interact()?)
}

fn delete_matches(term: &Term, matches: &MatchList, auto_approve: bool) -> Result<()> {
    if matches.is_empty() {
        return Ok(());
    }

    let mut deleted = 0;
    let mut skipped = 0;

    for mtch in matches.iter() {
        if !auto_approve && !confirm_delete(mtch)? {
            skipped += 1;
            continue;
        }

        remove_path(&mtch.path)?;
        deleted += 1;
        term.write_line(&format!("Deleted {}", style(mtch.path.display()).red()))?;
    }

    term.write_line(&format!(
        "Deleted {} paths, skipped {}",
        style(deleted).red(),
        style(skipped).yellow()
    ))?;

    Ok(())
}

fn write_scan_progress(
    term: &Term,
    scanned_paths: usize,
    matches: &MatchList,
    status_path: Option<(&str, &Path)>,
) -> Result<()> {
    term.clear_line()?;
    if let Some((status, path)) = status_path {
        term.write_str(&format!(
            "\rScanning... {} scanned, {} matches, {:.2} GB; {} {}",
            style(scanned_paths).cyan(),
            style(matches.len()).cyan(),
            style(bytes_to_gigabytes(matches.total_size())).cyan(),
            status,
            path.display()
        ))?;
    } else {
        term.write_str(&format!(
            "\rScanning... {} scanned, {} matches, {:.2} GB",
            style(scanned_paths).cyan(),
            style(matches.len()).cyan(),
            style(bytes_to_gigabytes(matches.total_size())).cyan()
        ))?;
    }
    term.flush()?;

    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    let term = Term::stdout();

    if !cli.delete {
        term.write_line(&format!(
            "{}",
            style("Performing dry run. Use --delete to remove files").color256(245)
        ))?;
    }

    let mut matches = MatchList::new();
    let mut scanned_paths = 0;
    write_scan_progress(&term, scanned_paths, &matches, None)?;

    for event in Scanner::new(&cli.root, cli.max_depth, cli.profile) {
        match event? {
            ScanEvent::Visited => {
                scanned_paths += 1;
                if scanned_paths % PROGRESS_UPDATE_INTERVAL == 0 {
                    write_scan_progress(&term, scanned_paths, &matches, None)?;
                }
            }
            ScanEvent::Sizing(path) => {
                scanned_paths += 1;
                write_scan_progress(&term, scanned_paths, &matches, Some(("sizing", &path)))?;
            }
            ScanEvent::Match(mtch) => {
                let path = mtch.path.clone();
                matches.push(mtch);
                write_scan_progress(&term, scanned_paths, &matches, Some(("matched", &path)))?;
            }
        }
    }

    term.clear_line()?;

    term.write_line(&format!(
        "\rFound {} deletable paths totaling {:.2} GB",
        style(matches.len()).cyan(),
        style(bytes_to_gigabytes(matches.total_size())).cyan()
    ))?;

    if cli.delete {
        delete_matches(&term, &matches, cli.auto_approve)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    run(cli)?;

    Ok(())
}
