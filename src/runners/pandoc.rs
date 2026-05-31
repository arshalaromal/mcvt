use colored::Colorize;
use std::process::Command;

/// Executes Pandoc commands for document translation.
///
/// Pandoc reads inputs positionally without flags, but strictly requires
/// the `-o` flag before defining the target destination.
/// Sequence: `pandoc [INPUT_ARGS] <INPUT_FILE> [OUTPUT_ARGS] -o <OUTPUT_FILE>`
pub struct PandocRunner {
    pub binary_path: String,
    pub input_path: String,
    pub output_path: String,
    pub input_args: Vec<String>,
    pub output_args: Vec<String>,
    pub verbose: bool,
}

impl PandocRunner {
    /// Instantiates a new Pandoc execution pipeline.
    pub fn new(input: &str, output: &str) -> Self {
        Self {
            binary_path: "pandoc".to_string(),
            input_path: input.to_string(),
            output_path: output.to_string(),
            input_args: Vec::new(),
            output_args: Vec::new(),
            verbose: false,
        }
    }

    /// Spawns the Pandoc child process. Captures external LaTeX engine errors
    /// if PDF compilation fails during markdown/docx conversions.
    pub fn run(&self) -> Result<(), String> {
        let mut cmd = Command::new(&self.binary_path);

        // 1. Input args (e.g., -f docx)
        for arg in &self.input_args {
            cmd.arg(arg);
        }

        // 2. Input file
        cmd.arg(&self.input_path);

        // 3. Output args (e.g., --pdf-engine=xelatex)
        for arg in &self.output_args {
            cmd.arg(arg);
        }

        // 4. Output file (Pandoc requires the explicit -o flag)
        cmd.arg("-o").arg(&self.output_path);

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
            Err(e) => Err(format!(
                "[✘] Failed to execute '{}'. Is Pandoc installed? Error: {}",
                self.binary_path, e
            )
            .red()
            .to_string()),
        }
    }
}
