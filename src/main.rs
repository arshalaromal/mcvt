use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use sysinfo::System;

mod batch;
mod executor;
mod router;
mod runners;
mod templates;

/// MCVT: Multi-Format Conversion Engine
///
/// A unified, multi-threaded routing interface for Video, Audio, Images, and Documents.
/// Acts as a smart layer over FFmpeg, ImageMagick, and Pandoc, utilizing Magic Byte
/// detection and context-aware optimization templates.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "MCVT",
    version = "0.1.0",
    author = "Arshal Aromal",
    about = "Multi-Format Conversion Engine",
    long_about = "MCVT acts as a smart layer over FFmpeg, ImageMagick, and Pandoc. It utilizes Magic Byte detection for accurate routing, applies context-aware optimization templates, and supports highly concurrent, recursive directory batch processing."
)]
pub struct Cli {
    #[arg(help = "The source file or directory path to convert")]
    pub input: String,

    #[arg(help = "The target file or directory path")]
    pub output: String,

    // ---------------------------------------------------------
    // Batch Processing Overrides
    // ---------------------------------------------------------
    #[arg(
        long,
        help = "REQUIRED FOR BATCHING: Target output extension (e.g., mkv)"
    )]
    pub batch_ext: Option<String>,

    #[arg(long, help = "OPTIONAL: Filter batch input by extension (e.g., mp4)")]
    pub batch_in: Option<String>,

    #[arg(
        short = 'R',
        long,
        help = "OPTIONAL: Recursively process and mirror sub-directories"
    )]
    pub recursive: bool,

    // ---------------------------------------------------------
    // Core Engine Overrides
    // ---------------------------------------------------------
    #[arg(
        long,
        help = "Override routing by forcing domains (e.g., image:document)"
    )]
    pub force: Option<String>,

    #[arg(
        long,
        default_value_t = false,
        help = "Disable Magic Bytes; strictly trust file extensions"
    )]
    pub no_guess: bool,

    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Print raw underlying tool output for debugging"
    )]
    pub verbose: bool,

    // ---------------------------------------------------------
    // Tool-Specific Executable Paths & Arguments
    // ---------------------------------------------------------
    #[arg(long, help = "Provide a custom system path for the FFmpeg binary")]
    pub ffmpeg_path: Option<String>,

    #[arg(long, help = "Provide a custom system path for the ImageMagick binary")]
    pub magick_path: Option<String>,

    #[arg(long, help = "Provide a custom system path for the Pandoc binary")]
    pub pandoc_path: Option<String>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw input flags into FFmpeg")]
    pub ffmpeg_in: Option<Vec<String>>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw output flags into FFmpeg")]
    pub ffmpeg_out: Option<Vec<String>>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw input flags into ImageMagick")]
    pub magick_in: Option<Vec<String>>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw output flags into ImageMagick")]
    pub magick_out: Option<Vec<String>>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw input flags into Pandoc")]
    pub pandoc_in: Option<Vec<String>>,

    #[arg(long, num_args = 1.., allow_hyphen_values = true, help = "Inject raw output flags into Pandoc")]
    pub pandoc_out: Option<Vec<String>>,
}

fn main() {
    // ---------------------------------------------------------
    // Global Interrupt Handler (Zombie Process Assassin)
    // ---------------------------------------------------------
    // Intercepts SIGINT (Ctrl+C) to prevent orphaned C-binaries
    // from consuming OS resources in the background after MCVT exits.
    ctrlc::set_handler(move || {
        eprintln!(
            "{}",
            "\n[!] Ctrl+C Pressed! Quitting Background Processes...".yellow()
        );

        let mut sys = System::new();
        sys.refresh_all();

        if let Ok(pid) = sysinfo::get_current_pid() {
            for (_pid, process) in sys.processes() {
                if process.parent() == Some(pid) {
                    process.kill();
                }
            }
        }

        eprintln!("{}", "[✓] Shutdown Safely.".green());
        std::process::exit(1);
    })
    .expect(&"[✘] Error setting Ctrl-C handler".red().to_string());

    // ---------------------------------------------------------
    // Execution Routing
    // ---------------------------------------------------------
    let cli = Cli::parse();
    let input_path = Path::new(&cli.input);

    if input_path.is_dir() {
        // Route to the Rayon thread-pool engine for directory traversal
        println!(
            "{}",
            "[✓] Directory detected. Engaging Batch Mode...".green()
        );
        if let Err(e) = batch::process_directory(cli) {
            eprintln!("{}", format!("[✘] Batch Error: {}", e).red());
            std::process::exit(1);
        }
    } else {
        // ---------------------------------------------------------
        // Single-File Execution & Safety Checks
        // ---------------------------------------------------------
        let in_path = Path::new(&cli.input);
        let out_path = Path::new(&cli.output);

        // Safety: Prevent backend binaries from truncating source files to 0 bytes
        // by verifying that canonicalized input and output paths do not overlap.
        if in_path
            .canonicalize()
            .unwrap_or_else(|_| in_path.to_path_buf())
            == out_path
                .canonicalize()
                .unwrap_or_else(|_| out_path.to_path_buf())
        {
            eprintln!(
                "{}",
                "[✘] Error: Input and Output paths are identical.".red()
            );
            std::process::exit(1);
        }

        // Dynamically create the target output directory if it does not exist
        if let Some(parent) = Path::new(&cli.output).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                    eprintln!(
                        "{}",
                        format!("[!] Warning: Failed to create output dir: {}", e).yellow()
                    )
                });
            }
        }

        // Initialize progress UI
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .expect("Invalid progress bar template"),
        );
        pb.set_message(format!("Converting: {} -> {}", cli.input, cli.output));

        // Resolve domains and trigger the backend executor
        match router::resolve_domains(&cli, &cli.input, &cli.output) {
            Ok((in_domain, out_domain)) => {
                if let Err(e) = executor::run(&cli, &cli.input, &cli.output, in_domain, out_domain)
                {
                    pb.finish_and_clear();
                    eprintln!("{}", format!("[✘] Execution Failed: {}", e).red());
                    std::process::exit(1);
                } else {
                    pb.finish_with_message("[✓] Conversion successful!".green().to_string());
                }
            }
            Err(e) => {
                pb.finish_and_clear();
                eprintln!("{}", format!("[✘] Routing Error: {}", e).red());
                std::process::exit(1);
            }
        }
    }
}
