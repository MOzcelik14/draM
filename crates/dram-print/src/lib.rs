//! `dram-print`: Cross-platform OS print handoff via PDF rendering.
//!
//! Pure Rust has no cross-platform native print dialog; `dram-print` renders the
//! screenplay to a temporary PDF using `dram-pdf` and hands it off to CUPS (`lp`/`lpr`)
//! on Linux/macOS or `ShellExecuteW` (`"print"`) on Windows, with a fallback to open
//! the PDF in the system's default viewer.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use dram_core::{Document, LayoutProfile};
use dram_pdf::{export_pdf_file, PdfError};
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

/// Renders `doc` to a temporary PDF file.
pub fn render_temp_pdf(doc: &Document, profile: &LayoutProfile) -> Result<PathBuf, PrintError> {
    let tmp_path = std::env::temp_dir().join(format!("dram_print_{}.pdf", Uuid::new_v4()));
    export_pdf_file(&tmp_path, doc, profile)?;
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
    if let Some(printer) = options.printer.as_deref().filter(|s| !s.trim().is_empty()) {
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
        cmd.arg("-P").arg(range);
    }
    cmd.arg(pdf_path);

    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // Fallback attempt with `lpr` if `lp` failed
            let mut lpr = Command::new("lpr");
            if let Some(printer) = options.printer.as_deref().filter(|s| !s.trim().is_empty()) {
                lpr.arg("-P").arg(printer);
            }
            lpr.arg(format!("-#{copies}")).arg(pdf_path);
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
fn submit_pdf_to_os(pdf_path: &Path, _options: &PrintOptions) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb: Vec<u16> = "print\0".encode_utf16().collect();
    let file: Vec<u16> = pdf_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let res = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if res.0 as isize <= 32 {
        Err(format!("ShellExecuteW failed with code {:?}", res.0))
    } else {
        Ok(())
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
}
