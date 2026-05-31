/// Returns standard optimization defaults for FFmpeg based on the target output extension.
///
/// These defaults are designed to ensure maximum compatibility and prevent pipeline
/// crashes. For `.mp4` and `.mkv` files, it enforces H.264 encoding, applies a near-lossless
/// Constant Rate Factor (CRF) of 16, and injects a mandatory filter chain to de-interlace
/// source material and force even-pixel dimensions (required by many hardware decoders).
pub fn ffmpeg_output_defaults(output_file: &str) -> Vec<String> {
    let ext = output_file.to_lowercase();

    if ext.ends_with(".mp4") || ext.ends_with(".mkv") {
        vec![
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "medium".into(),
            "-crf".into(),
            "16".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            // THE FILTER CHAIN:
            // 1. Auto-deinterlace to fix motion blur.
            // 2. Force even dimensions to prevent hardware playback crashes.
            "-vf".into(),
            "bwdif=deint=interlaced,scale=trunc(iw/2)*2:trunc(ih/2)*2".into(),
            "-movflags".into(),
            "+faststart".into(),
        ]
    } else if ext.ends_with(".avi") {
        vec!["-qscale:v".into(), "2".into()]
    } else {
        vec![] // Pass-through empty arguments for unknown formats
    }
}

/// Returns a highly optimized, 2-pass complex filter chain for Video-to-GIF conversion.
///
/// Bypasses standard encoding to generate a custom color palette (`palettegen`) from the
/// source video, and immediately applies it (`paletteuse`) to render a high-quality,
/// dithered animation locked at 15 FPS to optimize file size.
pub fn gif_pipeline_filter() -> Vec<String> {
    vec![
        "-filter_complex".into(),
        "[0:v] fps=15,scale=w=640:h=-1,split [a][b];[a] palettegen [p];[b][p] paletteuse".into(),
    ]
}

/// Returns input arguments for ImageMagick when reading PDF documents.
///
/// Documents require a high rasterization density before being processed into images.
/// If omitted, ImageMagick defaults to 72 DPI, resulting in heavily pixelated/unreadable output.
pub fn magick_pdf_input(input_file: &str) -> Vec<String> {
    if input_file.to_lowercase().ends_with(".pdf") {
        vec!["-density".into(), "300".into()]
    } else {
        vec![]
    }
}

/// Returns output arguments for ImageMagick to prevent memory bloat on large documents.
///
/// When converting raw, high-resolution photography into PDFs, the resulting document
/// can easily exceed several gigabytes. This template enforces a maximum boundary
/// of 1200x1200px while maintaining maximum image quality.
pub fn magick_pdf_output(output_file: &str) -> Vec<String> {
    if output_file.to_lowercase().ends_with(".pdf") {
        vec![
            "-resize".into(),
            "1200x1200>".into(),
            "-quality".into(),
            "100".into(),
        ]
    } else {
        vec![]
    }
}

/// Returns input arguments for FFmpeg when converting a static image into a video.
///
/// Standard media players will often crash or instantly close when playing a 1-frame video.
/// This template loops the static image (`-loop 1`) and enforces a 5-second duration (`-t 5`).
pub fn ffmpeg_image_to_video_input(input_file: &str) -> Vec<String> {
    if input_file.to_lowercase().ends_with(".gif") {
        vec![] // GIFs are natively animated; do nothing.
    } else {
        vec![
            "-loop".into(),
            "1".into(),
            "-t".into(),
            "5".into(), // Lock the static video duration to 5 seconds
        ]
    }
}

// ---------------------------------------------------------
// TESTS
// ---------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_mp4_defaults() {
        let args = ffmpeg_output_defaults("vacation.mp4");
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"yuv420p".to_string()));
    }

    #[test]
    fn test_magick_pdf_constraints() {
        let out_args = magick_pdf_output("document.pdf");
        assert!(out_args.contains(&"-resize".to_string()));

        // Non-PDF targets should return an empty vector
        let empty_args = magick_pdf_output("image.jpg");
        assert!(empty_args.is_empty());
    }
}
