//! Batch processing engine for MCVT.
//! Handles parallel directory traversal, path reconstruction, and multithreaded task execution.

use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::executor;
use crate::router;
use crate::Cli;

/// Orchestrates the batch processing workflow across a given directory.
/// Pre-calculates output paths single-threadedly before delegating to the Rayon thread pool.
pub fn process_directory(cli: Cli) -> Result<(), String> {
    let input_dir = Path::new(&cli.input);
    let output_dir = Path::new(&cli.output);

    // Extract target extension
    let target_ext = cli
        .batch_ext
        .as_ref()
        .map(|s| s.trim_start_matches('.'))
        .ok_or_else(|| {
            "[!] You must provide --batch-ext (e.g., mkv)"
                .yellow()
                .to_string()
        })?;

    // Extract input filter extension (if provided)
    let filter_in_ext = cli
        .batch_in
        .as_deref()
        .map(|s| s.trim_start_matches('.').to_lowercase());

    // Initialize root output directory
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| {
            format!("[✘] Failed to create output directory: {}", e)
                .red()
                .to_string()
        })?;
    }

    let mut files_to_process: Vec<(PathBuf, PathBuf)> = Vec::new();

    // ==========================================
    // THE RECURSIVE ENGINE
    // ==========================================
    let walker = WalkDir::new(input_dir);

    // Evaluate `-R` flag for sub-directory traversal depth
    let iterator = if cli.recursive {
        walker.into_iter()
    } else {
        walker.max_depth(1).into_iter()
    };

    println!("Scanning directory structure...");

    for entry in iterator.filter_map(Result::ok) {
        let path = entry.path();

        if path.is_file() {
            let file_name_display = path.file_name().unwrap_or_default().to_string_lossy();

            // Ignore hidden system files
            if file_name_display.starts_with('.') {
                continue;
            }

            // Apply input extension filter
            if let Some(ref ext) = filter_in_ext {
                let file_ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if file_ext != *ext {
                    continue;
                }
            }

            // Calculate relative path structure to mirror directory tree
            let rel_path = path.strip_prefix(input_dir).unwrap_or(path);
            let mut output_file_path = output_dir.join(rel_path);
            output_file_path.set_extension(target_ext);

            // Dynamically pre-create nested subdirectories
            if let Some(parent) = output_file_path.parent() {
                if !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            // Safety check: Prevent truncating source files to 0 bytes
            if path == output_file_path {
                println!(
                    "{}",
                    format!(
                        "[!] Skipping [{}]: Cannot overwrite self.",
                        file_name_display
                    )
                    .yellow()
                );
                continue;
            }

            if output_file_path.exists() {
                println!(
                    "{}",
                    format!(
                        "[!] Warning: Overwriting existing file: [{}]",
                        file_name_display
                    )
                    .yellow()
                );
            }

            // Stage validated paths for thread distribution
            files_to_process.push((path.to_path_buf(), output_file_path));
        }
    }

    if files_to_process.is_empty() {
        return Err("[✘] No valid files found in the directory after filtering."
            .red()
            .to_string());
    }

    let total_files = files_to_process.len() as u64;
    println!("Starting parallel processing on {} files...", total_files);

    let pb = ProgressBar::new(total_files);
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} files ({eta})")
        .expect("Invalid progress bar template")
        .progress_chars("#>-"));

    // ==========================================
    // PARALLEL THREAD POOL EXECUTION
    // ==========================================
    files_to_process
        .par_iter()
        .for_each(|(input_file_path, output_file_path)| {
            let file_name_display = input_file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let in_str = input_file_path.to_string_lossy();
            let out_str = output_file_path.to_string_lossy();

            match router::resolve_domains(&cli, &in_str, &out_str) {
                Ok((in_domain, out_domain)) => {
                    if let Err(e) = executor::run(&cli, &in_str, &out_str, in_domain, out_domain) {
                        pb.println(
                            format!("[✘] Failed [{}]: {}", file_name_display, e)
                                .red()
                                .to_string(),
                        );
                    } else {
                        pb.println(
                            format!("[✓] Success [{}]", file_name_display)
                                .green()
                                .to_string(),
                        );
                    }
                }
                Err(e) => pb.println(
                    format!("[✘] Routing Error [{}]: {}", file_name_display, e)
                        .red()
                        .to_string(),
                ),
            }
            pb.inc(1);
        });

    pb.finish_with_message("[✓] Batch processing complete!".green().to_string());
    Ok(())
}
