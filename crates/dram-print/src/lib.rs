//! `dram-print`: Cross-platform OS print handoff via PDF rendering.
//!
//! Pure Rust has no cross-platform native print dialog; `dram-print` renders the
//! screenplay to a temporary PDF using `dram-pdf` and hands it off to CUPS (`lp`/`lpr`)
//! on Linux/macOS or `ShellExecuteW` (`"print"`) on Windows, with a fallback to open
//! the PDF in the system's default viewer.

#![forbid(unsafe_code)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use dram_core::{Document, LayoutProfile};
use dram_pdf::{export_pdf_bytes, PdfError};
use thiserror::Error;
use uuid::Uuid;

/// Options for printing a screenplay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintOptions {
    /// Target printer name (`None` = system default).
    pub printer: Option<String>,
    /// Number of copies (`>= 1`).
    pub copies: u32,
    /// Optional page range (e.g., `"1-5,8"`).
    pub page_range: Option<String>,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            printer: None,
            copies: 1,
            page_range: None,
        }
    }
}

/// Errors produced during printing.
#[derive(Debug, Error)]
pub enum PrintError {
    /// Failed to render the document to PDF prior to printing.
    #[error("PDF rendering failed: {0}")]
    Pdf(#[from] PdfError),

    /// OS print command failed, returning the generated PDF path for viewer fallback.
    #[error("print command failed: {message}")]
    PrintCommand {
        /// Human-readable error message.
        message: String,
        /// Path to the rendered temporary PDF for fallback opening.
        pdf_path: PathBuf,
    },

    /// Failed to open PDF in default viewer.
    #[error("could not open PDF viewer: {0}")]
    ViewerFailed(String),
}

/// Lists available printers from the operating system (CUPS `lpstat -p` on Linux/macOS).
#[must_use]
pub fn list_printers() -> Vec<String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        if let Ok(output) = Command::new("lpstat").arg("-p").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return parse_lpstat_output(&stdout);
            }
        }
        Vec::new()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Parses printer names from `lpstat -p` output.
#[must_use]
pub fn parse_lpstat_output(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("printer ")?;
            rest.split_whitespace().next().map(ToString::to_string)
        })
        .collect()
}

/// Validates a CUPS/system printer name against flag-injection and control characters.
fn validate_printer_name(printer: &str) -> Result<&str, String> {
    let trimmed = printer.trim();
    if trimmed.is_empty() {
        return Err("printer name must not be empty".to_string());
    }
    if trimmed.starts_with('-') || trimmed.chars().any(|c| c.is_control()) {
        return Err(format!("invalid printer name `{trimmed}`"));
    }
    Ok(trimmed)
}

/// Validates a print page range string (e.g., `"1-5,8"`).
fn validate_page_range(range: &str) -> Result<String, String> {
    let compact: String = range.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty()
        || compact.starts_with('-')
        || !compact
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ',' | '-'))
    {
        return Err(format!("invalid page range `{}`", range.trim()));
    }
    Ok(compact)
}

/// Renders `doc` to a temporary PDF file with restricted permissions (`0600` on Unix).
pub fn render_temp_pdf(doc: &Document, profile: &LayoutProfile) -> Result<PathBuf, PrintError> {
    let bytes = export_pdf_bytes(doc, profile)?;
    let tmp_path = std::env::temp_dir().join(format!("dram_print_{}.pdf", Uuid::new_v4()));

    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(&tmp_path).map_err(PdfError::Io)?;
    file.write_all(&bytes).map_err(PdfError::Io)?;
    Ok(tmp_path)
}

/// Renders `doc` to PDF and submits it to the OS printing subsystem.
///
/// Returns the rendered temporary PDF path on success.
pub fn print_document(
    doc: &Document,
    profile: &LayoutProfile,
    options: &PrintOptions,
) -> Result<PathBuf, PrintError> {
    if let Some(range) = options
        .page_range
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Err(message) = validate_page_range(range) {
            return Err(PrintError::PrintCommand {
                message,
                pdf_path: PathBuf::new(),
            });
        }
    }
    let pdf_path = render_temp_pdf(doc, profile)?;
    submit_pdf_to_os(&pdf_path, options).map_err(|message| PrintError::PrintCommand {
        message,
        pdf_path: pdf_path.clone(),
    })?;
    Ok(pdf_path)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn submit_pdf_to_os(pdf_path: &Path, options: &PrintOptions) -> Result<(), String> {
    let mut cmd = Command::new("lp");
    let validated_printer = match options.printer.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(p) => Some(validate_printer_name(p)?),
        None => None,
    };
    if let Some(printer) = validated_printer {
        cmd.arg("-d").arg(printer);
    }
    let copies = options.copies.max(1);
    cmd.arg("-n").arg(copies.to_string());
    if let Some(range) = options
        .page_range
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let validated_range = validate_page_range(range)?;
        cmd.arg("-P").arg(validated_range);
    }
    cmd.arg("--").arg(pdf_path);

    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // Fallback attempt with `lpr` if `lp` failed
            let mut lpr = Command::new("lpr");
            if let Some(printer) = validated_printer {
                lpr.arg("-P").arg(printer);
            }
            lpr.arg(format!("-#{copies}")).arg("--").arg(pdf_path);
            if let Ok(lpr_out) = lpr.output() {
                if lpr_out.status.success() {
                    return Ok(());
                }
            }
            Err(if stderr.is_empty() {
                format!("lp exited with status {}", out.status)
            } else {
                stderr
            })
        }
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(target_os = "windows")]
fn submit_pdf_to_os(pdf_path: &Path, options: &PrintOptions) -> Result<(), String> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command"]);
    cmd.env("DRAM_PRINT_FILE", pdf_path);

    let script = if let Some(printer) = options.printer.as_deref().filter(|s| !s.trim().is_empty())
    {
        let validated = validate_printer_name(printer)?;
        cmd.env("DRAM_PRINTER", validated);
        "Start-Process -FilePath $env:DRAM_PRINT_FILE -Verb PrintTo -ArgumentList $env:DRAM_PRINTER"
    } else {
        "Start-Process -FilePath $env:DRAM_PRINT_FILE -Verb Print"
    };

    match cmd.arg(script).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if err.is_empty() {
                format!("print command exited with status {}", out.status)
            } else {
                err
            })
        }
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn submit_pdf_to_os(_pdf_path: &Path, _options: &PrintOptions) -> Result<(), String> {
    Err("unsupported operating system for direct printing".to_string())
}

/// Opens `pdf_path` in the operating system's default PDF viewer (fallback path).
pub fn open_pdf_in_default_viewer(pdf_path: &Path) -> Result<(), PrintError> {
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(pdf_path).status();

    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(pdf_path).status();

    #[cfg(target_os = "windows")]
    let status = Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(pdf_path)
        .status();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err(PrintError::ViewerFailed("unsupported OS".to_string()));

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(PrintError::ViewerFailed(format!("viewer exited with {s}"))),
        Err(e) => Err(PrintError::ViewerFailed(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dram_core::{PageSize, ScriptLanguage};

    #[test]
    fn lpstat_parser_extracts_printer_names() {
        let sample = "\
printer HP_LaserJet_Pro is idle.  enabled since Mon 16 Nov 2026
printer Canon_PIXMA disabled since Tue 10 Nov 2026 -
";
        assert_eq!(
            parse_lpstat_output(sample),
            vec!["HP_LaserJet_Pro", "Canon_PIXMA"]
        );
    }

    #[test]
    fn render_temp_pdf_creates_valid_file() {
        let doc = Document::new(ScriptLanguage::Tr);
        let path = render_temp_pdf(&doc, &LayoutProfile::for_page_size(PageSize::A4)).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn print_option_validation_rejects_flag_injection() {
        assert_eq!(validate_page_range(" 1 - 5, 8 ").unwrap(), "1-5,8");
        assert!(validate_page_range("-1").is_err());
        assert!(validate_page_range("1;rm -rf /").is_err());
        assert!(validate_printer_name("-o evil").is_err());
        assert_eq!(validate_printer_name("HP_LaserJet").unwrap(), "HP_LaserJet");
    }
}
