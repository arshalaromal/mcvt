//! Central execution orchestrator for MCVT.
//! Routes domain pairs to their specific tool runners and handles system dependency validation.

use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

use colored::Colorize;

use crate::router::Domain;
use crate::runners::{ffmpeg::FfmpegRunner, imagemagick::ImageMagickRunner, pandoc::PandocRunner};
use crate::templates;
use crate::Cli;

/// Thread-safe global cache for validated system binaries.
/// Prevents OS scheduler thread exhaustion during massive parallel batch executions
/// by ensuring each underlying tool is only verified exactly once per application lifecycle.
static VERIFIED_BINS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

/// Directs the verified input/output domains to the appropriate execution pipeline.
pub fn run(
    cli: &Cli,
    current_input: &str,
    current_output: &str,
    in_domain: Domain,
    out_domain: Domain,
) -> Result<(), String> {
    match (in_domain, out_domain) {
        // Standard Same-Domain Processing
        (Domain::VideoAudio, Domain::VideoAudio) => run_ffmpeg(cli, current_input, current_output),
        (Domain::Image, Domain::Image) => run_imagemagick(cli, current_input, current_output),
        (Domain::Document, Domain::Document) => run_pandoc(cli, current_input, current_output),

        // Cross-Domain: Video to Image (e.g., Video to GIF pipeline)
        (Domain::VideoAudio, Domain::Image) => {
            if current_output.to_lowercase().ends_with(".gif") {
                run_video_to_gif_pipeline(cli, current_input, current_output)
            } else {
                run_ffmpeg(cli, current_input, current_output)
            }
        }

        // Cross-Domain: Image to Video (e.g., Static image loop)
        (Domain::Image, Domain::VideoAudio) => {
            let binary = cli.ffmpeg_path.as_deref().unwrap_or("ffmpeg").to_string();
            check_dependency(&binary)?;

            let mut runner = FfmpegRunner::new(current_input, current_output);
            runner.binary_path = binary;
            runner.input_args = cli
                .ffmpeg_in
                .clone()
                .unwrap_or_else(|| templates::ffmpeg_image_to_video_input(current_input));
            runner.output_args = cli
                .ffmpeg_out
                .clone()
                .unwrap_or_else(|| templates::ffmpeg_output_defaults(current_output));

            runner.run()
        }

        // Cross-Domain: Image to Document (e.g., Rasterized Image to PDF)
        (Domain::Image, Domain::Document) => {
            if current_output.to_lowercase().ends_with(".pdf") {
                run_imagemagick(cli, current_input, current_output)
            } else {
                Err(
                    "[✘] Unsupported: Cannot convert an Image into a Text Document (like .docx)."
                        .red()
                        .to_string(),
                )
            }
        }

        // Cross-Domain: Document to Image (e.g., PDF to Image sequence)
        (Domain::Document, Domain::Image) => {
            if current_input.to_lowercase().ends_with(".pdf") {
                run_doc_to_image(cli, current_input, current_output)
            } else {
                Err(
                    "[✘] Unsupported: Can only convert Documents to Images if the input is a PDF."
                        .red()
                        .to_string(),
                )
            }
        }

        // Hard Boundaries (Impossible Conversions)
        (Domain::VideoAudio, Domain::Document) => {
            Err("[✘] Impossible: Cannot convert Video/Audio to Document."
                .red()
                .to_string())
        }
        (Domain::Document, Domain::VideoAudio) => {
            Err("[✘] Impossible: Cannot convert Document to Video/Audio."
                .red()
                .to_string())
        }
    }
}

// ---------------------------------------------------------
// Tool-Specific Handlers
// ---------------------------------------------------------

fn run_ffmpeg(cli: &Cli, current_input: &str, current_output: &str) -> Result<(), String> {
    let binary = cli.ffmpeg_path.as_deref().unwrap_or("ffmpeg").to_string();
    check_dependency(&binary)?;

    let mut runner = FfmpegRunner::new(current_input, current_output);
    runner.binary_path = binary;
    runner.verbose = cli.verbose;
    runner.input_args = cli.ffmpeg_in.clone().unwrap_or_default();
    runner.output_args = cli
        .ffmpeg_out
        .clone()
        .unwrap_or_else(|| templates::ffmpeg_output_defaults(current_output));

    runner.run()
}

fn run_imagemagick(cli: &Cli, current_input: &str, current_output: &str) -> Result<(), String> {
    let binary = cli.magick_path.as_deref().unwrap_or("magick").to_string();
    check_dependency(&binary)?;

    let mut runner = ImageMagickRunner::new(current_input, current_output);
    runner.binary_path = binary;
    runner.verbose = cli.verbose;
    runner.input_args = cli.magick_in.clone().unwrap_or_default();
    runner.output_args = cli
        .magick_out
        .clone()
        .unwrap_or_else(|| templates::magick_pdf_output(current_output));

    runner.run()
}

fn run_pandoc(cli: &Cli, current_input: &str, current_output: &str) -> Result<(), String> {
    let binary = cli.pandoc_path.as_deref().unwrap_or("pandoc").to_string();
    check_dependency(&binary)?;

    let mut runner = PandocRunner::new(current_input, current_output);
    runner.binary_path = binary;
    runner.verbose = cli.verbose;
    runner.input_args = cli.pandoc_in.clone().unwrap_or_default();
    runner.output_args = cli.pandoc_out.clone().unwrap_or_default();

    runner.run()
}

// ---------------------------------------------------------
// Complex Pipeline Handlers
// ---------------------------------------------------------

fn run_video_to_gif_pipeline(
    cli: &Cli,
    current_input: &str,
    current_output: &str,
) -> Result<(), String> {
    let ffmpeg_bin = cli.ffmpeg_path.as_deref().unwrap_or("ffmpeg").to_string();
    check_dependency(&ffmpeg_bin)?;

    let mut ffmpeg = FfmpegRunner::new(current_input, current_output);
    ffmpeg.binary_path = ffmpeg_bin;
    ffmpeg.verbose = cli.verbose;
    ffmpeg.input_args = cli.ffmpeg_in.clone().unwrap_or_default();
    ffmpeg.output_args = cli
        .ffmpeg_out
        .clone()
        .unwrap_or_else(|| templates::gif_pipeline_filter());

    ffmpeg.run()
}

fn run_doc_to_image(cli: &Cli, current_input: &str, current_output: &str) -> Result<(), String> {
    let mut runner = ImageMagickRunner::new(current_input, current_output);
    runner.binary_path = cli.magick_path.as_deref().unwrap_or("magick").to_string();
    runner.verbose = cli.verbose;
    runner.input_args = cli
        .magick_in
        .clone()
        .unwrap_or_else(|| templates::magick_pdf_input(current_input));
    runner.output_args = cli
        .magick_out
        .clone()
        .unwrap_or_else(|| templates::magick_pdf_output(current_output));

    runner.run()
}

// ---------------------------------------------------------
// Dependency Checking
// ---------------------------------------------------------

/// Validates that required external system binaries exist before attempting execution.
/// Utilizes a thread-safe `RwLock` cache to ensure $O(1)$ lookups after the initial verification.
fn check_dependency(binary_name: &str) -> Result<(), String> {
    let cache = VERIFIED_BINS.get_or_init(|| RwLock::new(HashSet::new()));

    // 1. Concurrent Read Phase (Non-blocking for verified binaries)
    {
        let set = cache.read().unwrap();
        if set.contains(binary_name) {
            return Ok(());
        }
    }

    // 2. System Verification Phase (Occurs once per missing binary)
    let path = std::path::Path::new(binary_name);
    if path.is_absolute() {
        if !path.exists() {
            return Err(
                format!("[✘] Custom binary not found at path: {}", binary_name)
                    .red()
                    .to_string(),
            );
        }
    } else {
        let status = std::process::Command::new(binary_name)
            .arg("--version")
            .output();
        if status.is_err() {
            return Err(format!(
                "[✘] Missing System Dependency: '{}' is not installed.",
                binary_name
            )
            .red()
            .to_string());
        }
    }

    // 3. Exclusive Write Phase
    let mut set = cache.write().unwrap();
    set.insert(binary_name.to_string());
    Ok(())
}
