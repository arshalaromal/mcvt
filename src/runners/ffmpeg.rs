use colored::Colorize;
use std::process::Command;

/// Executes FFmpeg commands with context-aware argument injection.
///
/// This runner manages the complex argument positioning required by FFmpeg.
/// It constructs the exact sequence:
/// `ffmpeg [GLOBAL/INPUT ARGS] -i <INPUT_FILE> [OUTPUT ARGS] <OUTPUT_FILE>`
pub struct FfmpegRunner {
    pub binary_path: String,
    pub input_args: Vec<String>,
    pub output_args: Vec<String>,
    pub input_file: String,
    pub output_file: String,
    pub verbose: bool,
}

impl FfmpegRunner {
    /// Instantiates a new FFmpeg execution pipeline with safe defaults.
    pub fn new(input_file: &str, output_file: &str) -> Self {
        Self {
            binary_path: "ffmpeg".to_string(),
            input_args: Vec::new(),
            output_args: Vec::new(),
            input_file: input_file.to_string(),
            output_file: output_file.to_string(),
            verbose: false,
        }
    }

    /// Spawns the FFmpeg child process, capturing and formatting standard error.
    ///
    /// By default, FFmpeg is highly verbose. This method suppresses the banner
    /// and info logs unless `verbose` is explicitly enabled. It automatically
    /// applies the `-y` flag to allow the MCVT router to manage overwrites safely.
    pub fn run(&self) -> Result<(), String> {
        let mut cmd = Command::new(&self.binary_path);

        // VERBOSE SWITCH & GLOBAL FLAGS
        if !self.verbose {
            cmd.arg("-hide_banner");
            cmd.arg("-loglevel").arg("error");
        }

        // Force overwrite (Overwriting safety is handled upstream in batch.rs/main.rs)
        cmd.arg("-y");

        // 1. Input parameters (must precede the -i flag)
        for arg in &self.input_args {
            cmd.arg(arg);
        }

        // 2. Input file definition
        cmd.arg("-i").arg(&self.input_file);

        // 3. Output parameters (must precede the output file)
        for arg in &self.output_args {
            cmd.arg(arg);
        }

        // 4. Target output file
        cmd.arg(&self.output_file);

        let result = cmd.output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let raw_error = String::from_utf8_lossy(&output.stderr);

                    // Muzzle the error block to 3 lines unless verbose is requested
                    let error_msg = if self.verbose {
                        raw_error.to_string()
                    } else {
                        let error_lines: Vec<&str> = raw_error.trim().lines().collect();
                        if error_lines.len() > 3 {
                            let last_lines = &error_lines[error_lines.len() - 3..];
                            format!("...\n{}", last_lines.join("\n"))
                        } else {
                            raw_error.trim().to_string()
                        }
                    };

                    Err(format!("[✘] Ffmpeg Error:\n{}", error_msg)
                        .red()
                        .to_string())
                }
            }
            Err(e) => Err(
                format!("[✘] Failed to execute '{}'. Error: {}", self.binary_path, e)
                    .red()
                    .to_string(),
            ),
        }
    }
}
