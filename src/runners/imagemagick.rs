use colored::Colorize;
use std::process::Command;

/// Executes ImageMagick (`magick`) commands for image and PDF rasterization.
///
/// ImageMagick utilizes a strict positional architecture where reading constraints
/// must precede the input file, and processing filters must precede the output.
/// Sequence: `magick [INPUT_ARGS] <INPUT_FILE> [OUTPUT_ARGS] <OUTPUT_FILE>`
pub struct ImageMagickRunner {
    pub binary_path: String,
    pub input_path: String,
    pub output_path: String,
    pub input_args: Vec<String>,
    pub output_args: Vec<String>,
    pub verbose: bool,
}

impl ImageMagickRunner {
    /// Instantiates a new ImageMagick execution pipeline.
    pub fn new(input: &str, output: &str) -> Self {
        Self {
            binary_path: "magick".to_string(),
            input_path: input.to_string(),
            output_path: output.to_string(),
            input_args: Vec::new(),
            output_args: Vec::new(),
            verbose: false,
        }
    }

    /// Spawns the ImageMagick child process, truncating error logs for UI clarity.
    pub fn run(&self) -> Result<(), String> {
        let mut cmd = Command::new(&self.binary_path);

        // 1. Inject INPUT args (e.g., -density for PDF rasterization)
        for arg in &self.input_args {
            cmd.arg(arg);
        }

        // 2. The Input File
        cmd.arg(&self.input_path);

        // 3. Inject OUTPUT args (e.g., -resize, -quality)
        for arg in &self.output_args {
            cmd.arg(arg);
        }

        // 4. The Output File
        cmd.arg(&self.output_path);

        let result = cmd.output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let raw_error = String::from_utf8_lossy(&output.stderr);

                    let error_msg = if self.verbose {
                        raw_error.to_string()
                    } else {
                        // Muzzle to 3 lines if not verbose
                        let lines: Vec<&str> = raw_error.trim().lines().collect();
                        if lines.len() > 3 {
                            format!("...\n{}", lines[lines.len() - 3..].join("\n"))
                        } else {
                            raw_error.trim().to_string()
                        }
                    };
                    Err(format!("[✘] {} Error:\n{}", self.binary_path, error_msg)
                        .red()
                        .to_string())
                }
            }
            Err(e) => Err(format!("[✘] Failed to execute: {}", e).red().to_string()),
        }
    }
}
