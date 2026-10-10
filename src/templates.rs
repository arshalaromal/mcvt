use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

static DEFAULT_TOML_STR: &str = include_str!("default_templates.toml");
static LOADED_CONFIG: OnceLock<TemplateConfig> = OnceLock::new();

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TemplateConfig {
    #[serde(default)]
    pub ffmpeg: FfmpegConfig,
    #[serde(default)]
    pub imagemagick: ImageMagickConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FfmpegConfig {
    #[serde(default)]
    pub gif_pipeline: Vec<String>,
    #[serde(default)]
    pub image_to_video: Vec<String>,
    #[serde(default)]
    pub output_defaults: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ImageMagickConfig {
    #[serde(default)]
    pub pdf_input: Vec<String>,
    #[serde(default)]
    pub pdf_output: Vec<String>,
}

impl TemplateConfig {
    /// Loads template configuration according to resolution order:
    /// 1. Custom CLI path (if provided and exists)
    /// 2. User config path (~/.config/mcvt/templates.toml or OS equivalent)
    /// 3. Embedded default configuration (`src/default_templates.toml`)
    pub fn load(cli_config_path: Option<&str>) -> Self {
        // 1. Try explicit CLI config path
        if let Some(path_str) = cli_config_path {
            let path = Path::new(path_str);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(cfg) = toml::from_str::<TemplateConfig>(&content) {
                        return cfg;
                    }
                }
            }
        }

        // 2. Try user config directory
        if let Some(mut user_config_dir) = dirs::config_dir() {
            user_config_dir.push("mcvt");
            user_config_dir.push("templates.toml");
            if user_config_dir.exists() {
                if let Ok(content) = fs::read_to_string(&user_config_dir) {
                    if let Ok(cfg) = toml::from_str::<TemplateConfig>(&content) {
                        return cfg;
                    }
                }
            }
        }

        // 3. Fallback to embedded default TOML
        Self::load_embedded()
    }

    pub fn load_embedded() -> Self {
        toml::from_str(DEFAULT_TOML_STR).unwrap_or_default()
    }

    #[cfg(test)]
    pub fn parse_toml_str(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

/// Initializes global template config based on CLI flag (called once at startup).
pub fn init_global_config(cli_config_path: Option<&str>) {
    let cfg = TemplateConfig::load(cli_config_path);
    let _ = LOADED_CONFIG.set(cfg);
}

/// Helper to access global config, falling back to default loaded instance if not explicitly initialized.
pub fn get_config() -> &'static TemplateConfig {
    LOADED_CONFIG.get_or_init(|| TemplateConfig::load(None))
}

/// Returns standard optimization defaults for FFmpeg based on the target output extension.
pub fn ffmpeg_output_defaults(output_file: &str) -> Vec<String> {
    let config = get_config();
    let ext_lower = output_file.to_lowercase();
    let extension = Path::new(&ext_lower)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if let Some(args) = config.ffmpeg.output_defaults.get(extension) {
        args.clone()
    } else {
        vec![]
    }
}

/// Returns a highly optimized, 2-pass complex filter chain for Video-to-GIF conversion.
pub fn gif_pipeline_filter() -> Vec<String> {
    get_config().ffmpeg.gif_pipeline.clone()
}

/// Returns input arguments for ImageMagick when reading PDF documents.
pub fn magick_pdf_input(input_file: &str) -> Vec<String> {
    if input_file.to_lowercase().ends_with(".pdf") {
        get_config().imagemagick.pdf_input.clone()
    } else {
        vec![]
    }
}

/// Returns output arguments for ImageMagick to prevent memory bloat on large documents.
pub fn magick_pdf_output(output_file: &str) -> Vec<String> {
    if output_file.to_lowercase().ends_with(".pdf") {
        get_config().imagemagick.pdf_output.clone()
    } else {
        vec![]
    }
}

/// Returns input arguments for FFmpeg when converting a static image into a video.
pub fn ffmpeg_image_to_video_input(input_file: &str) -> Vec<String> {
    if input_file.to_lowercase().ends_with(".gif") {
        vec![] // GIFs are natively animated; do nothing.
    } else {
        get_config().ffmpeg.image_to_video.clone()
    }
}

// ---------------------------------------------------------
// TESTS
// ---------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_default_templates() {
        let config = TemplateConfig::load_embedded();
        assert!(config.ffmpeg.output_defaults.contains_key("mp4"));
        assert!(config.ffmpeg.output_defaults.contains_key("mkv"));
        assert_eq!(
            config.ffmpeg.output_defaults.get("avi").unwrap(),
            &vec!["-qscale:v".to_string(), "2".to_string()]
        );
        assert!(!config.ffmpeg.gif_pipeline.is_empty());
        assert!(!config.imagemagick.pdf_input.is_empty());
        assert!(!config.imagemagick.pdf_output.is_empty());
    }

    #[test]
    fn test_custom_toml_parse() {
        let custom_toml = r#"
            [ffmpeg]
            gif_pipeline = ["-filter_complex", "custom_filter"]
            image_to_video = ["-loop", "1", "-t", "10"]

            [ffmpeg.output_defaults]
            mp4 = ["-c:v", "libx265"]

            [imagemagick]
            pdf_input = ["-density", "150"]
            pdf_output = ["-quality", "80"]
        "#;

        let cfg = TemplateConfig::parse_toml_str(custom_toml).unwrap();
        assert_eq!(cfg.ffmpeg.gif_pipeline, vec!["-filter_complex", "custom_filter"]);
        assert_eq!(cfg.ffmpeg.image_to_video, vec!["-loop", "1", "-t", "10"]);
        assert_eq!(cfg.ffmpeg.output_defaults.get("mp4").unwrap(), &vec!["-c:v", "libx265"]);
        assert_eq!(cfg.imagemagick.pdf_input, vec!["-density", "150"]);
        assert_eq!(cfg.imagemagick.pdf_output, vec!["-quality", "80"]);
    }
}
