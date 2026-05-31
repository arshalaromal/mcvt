use crate::Cli;
use colored::Colorize;
use file_format::FileFormat;
use mime_guess::from_path;
use std::path::Path;

/// Represents the broad category of the media file.
/// Used by the executor to determine which backend binary (FFmpeg, ImageMagick, Pandoc)
/// should handle the translation pathway.
#[derive(Debug, Clone, PartialEq)]
pub enum Domain {
    VideoAudio,
    Image,
    Document,
}

/// Evaluates the input and output paths to resolve their respective operational domains.
///
/// # Routing Logic
/// 1. **Force Override:** If the user supplies the `--force` flag, automated routing is bypassed.
/// 2. **Output Resolution:** Resolves the target format via its file extension.
/// 3. **Input Resolution:** Reads the binary header ("Magic Bytes") of the input file to
///    prevent routing errors caused by spoofed or missing extensions. Falls back to extension
///    parsing if the header is generic (e.g., zip/xml) or if the `--no-guess` flag is provided.
pub fn resolve_domains(
    cli: &Cli,
    current_input: &str,
    current_output: &str,
) -> Result<(Domain, Domain), String> {
    // Step 1: Check for manual user override
    if let Some(force_string) = &cli.force {
        return parse_force_string(force_string);
    }

    // Step 2: Determine output domain strictly from the requested extension
    let out_domain = get_domain_from_extension(current_output).ok_or_else(|| {
        format!(
            "[✘] Cannot determine the output format for '{}'. Please use --force.",
            current_output
        )
        .red()
        .to_string()
    })?;

    // Step 3: Determine input domain using Magic Bytes (or fallback to extension)
    let in_domain;
    if cli.no_guess {
        in_domain = get_domain_from_extension(current_input).ok_or_else(|| {
            format!(
                "[✘] Failed to identify '{}' from extension. Please use --force.",
                current_input
            )
            .red()
            .to_string()
        })?;
    } else {
        match get_domain_from_magic_bytes(current_input) {
            Some(domain) => {
                in_domain = domain;
            }
            None => {
                in_domain = get_domain_from_extension(current_input)
                    .ok_or_else(|| format!("[✘] Could not identify '{}' from magic bytes OR extension. Please use --force.", current_input).red().to_string())?;
            }
        }
    }

    Ok((in_domain, out_domain))
}

// ---------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------

/// Parses the `--force` flag syntax (`input_domain:output_domain`).
fn parse_force_string(force_str: &str) -> Result<(Domain, Domain), String> {
    let parts: Vec<&str> = force_str.split(':').collect();

    if parts.len() != 2 {
        return Err(
            "[!] Invalid --force format. Use input:output (e.g., image:document)"
                .yellow()
                .to_string(),
        );
    }

    let parse_domain = |s: &str| match s.to_lowercase().as_str() {
        "video" | "audio" | "videoaudio" => Ok(Domain::VideoAudio),
        "image" => Ok(Domain::Image),
        "document" => Ok(Domain::Document),
        _ => Err(format!("[✘] Unknown domain '{}' in --force flag.", s)
            .red()
            .to_string()),
    };

    let in_domain = parse_domain(parts[0])?;
    let out_domain = parse_domain(parts[1])?;

    Ok((in_domain, out_domain))
}

/// Inspects the file's binary header to determine its true MIME type.
///
/// Note: Formats like `.docx` or `.epub` are technically just zipped XML archives.
/// If the magic byte reader returns a generic archive MIME type, this function returns `None`
/// to force the router to fall back to standard extension parsing.
fn get_domain_from_magic_bytes(file_path: &str) -> Option<Domain> {
    let format = FileFormat::from_file(file_path).ok()?;
    let mime = format.media_type();

    if mime == "text/xml" || mime == "application/zip" || mime == "application/octet-stream" {
        return None;
    }

    match_mime_to_domain(mime)
}

/// Determines the file's MIME type based purely on its string extension.
fn get_domain_from_extension(file_path: &str) -> Option<Domain> {
    let guessed_mime = from_path(Path::new(file_path)).first()?;
    match_mime_to_domain(guessed_mime.as_ref())
}

/// Maps standard MIME types to their corresponding internal routing Domain.
fn match_mime_to_domain(mime: &str) -> Option<Domain> {
    if mime.starts_with("image/") {
        return Some(Domain::Image);
    }
    if mime.starts_with("video/") || mime.starts_with("audio/") {
        return Some(Domain::VideoAudio);
    }
    if mime.starts_with("text/") || mime.starts_with("application/vnd") {
        return Some(Domain::Document);
    }

    // Handle edge-case document and media types
    match mime {
        "application/pdf" | "application/msword" | "application/epub+zip" | "application/rtf" => {
            Some(Domain::Document)
        }
        "application/mp4" | "application/ogg" | "application/mxf" => Some(Domain::VideoAudio),
        _ => None,
    }
}

// ---------------------------------------------------------
// TESTS
// ---------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_string_parsing() {
        assert_eq!(
            parse_force_string("image:video"),
            Ok((Domain::Image, Domain::VideoAudio))
        );
        assert_eq!(
            parse_force_string("DOCUMENT:IMAGE"),
            Ok((Domain::Document, Domain::Image))
        );
        assert!(parse_force_string("image_to_video").is_err());
        assert!(parse_force_string("magic:video").is_err());
    }

    #[test]
    fn test_mime_matching() {
        assert_eq!(match_mime_to_domain("image/png"), Some(Domain::Image));
        assert_eq!(match_mime_to_domain("video/mp4"), Some(Domain::VideoAudio));
        assert_eq!(
            match_mime_to_domain("application/pdf"),
            Some(Domain::Document)
        );
        assert_eq!(match_mime_to_domain("application/zip"), None);
    }
}
