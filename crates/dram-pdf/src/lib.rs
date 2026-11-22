//! `dram-pdf`: Pure-Rust screenplay PDF generator using `krilla` and embedded Courier Prime (SIL OFL).

#![forbid(unsafe_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use dram_core::{
    paginate, Document, InlineStyle, LayoutProfile, LineAlign, PaginatedLine, TextRun, TitlePage,
};
use krilla::geom::{PathBuilder, Point};
use krilla::page::PageSettings;
use krilla::paint::Stroke;
use krilla::text::{Font, TextDirection};
use krilla::Document as KrillaDocument;
use thiserror::Error;
use ttf_parser::Face;

const FONT_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-Bold.ttf");
const FONT_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-Italic.ttf");
const FONT_BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-BoldItalic.ttf");

/// All Turkish-specific characters that must be present in every embedded font face.
pub const REQUIRED_TURKISH_CHARS: &str = "İıĞğŞşÇçÖöÜü";

/// Errors produced during PDF generation.
#[derive(Debug, Error)]
pub enum PdfError {
    /// Failed to load or parse an embedded font.
    #[error("failed to load embedded font `{0}`")]
    FontLoad(&'static str),

    /// An embedded font is missing a required Turkish glyph.
    #[error("embedded font `{font}` is missing glyph for '{ch}'")]
    MissingGlyph {
        /// Font variant name.
        font: &'static str,
        /// Missing character.
        ch: char,
    },

    /// `krilla` failed to construct or finish the PDF document.
    #[error("PDF generation failed: {0}")]
    Krilla(String),

    /// Filesystem write error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

struct FontFamily {
    regular: Font,
    bold: Font,
    italic: Font,
    bold_italic: Font,
}

static CACHED_FONTS: OnceLock<Result<FontFamily, PdfError>> = OnceLock::new();
static GLYPHS_VERIFIED: OnceLock<Result<(), PdfError>> = OnceLock::new();
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl FontFamily {
    fn cached() -> Result<&'static Self, PdfError> {
        match CACHED_FONTS.get_or_init(Self::load) {
            Ok(fonts) => Ok(fonts),
            Err(err) => Err(match err {
                PdfError::FontLoad(name) => PdfError::FontLoad(name),
                PdfError::MissingGlyph { font, ch } => PdfError::MissingGlyph { font, ch: *ch },
                PdfError::Krilla(m) => PdfError::Krilla(m.clone()),
                PdfError::Io(e) => PdfError::Krilla(e.to_string()),
            }),
        }
    }

    fn load() -> Result<Self, PdfError> {
        Ok(Self {
            regular: Font::new(FONT_REGULAR.to_vec().into(), 0)
                .ok_or(PdfError::FontLoad("CourierPrime-Regular"))?,
            bold: Font::new(FONT_BOLD.to_vec().into(), 0)
                .ok_or(PdfError::FontLoad("CourierPrime-Bold"))?,
            italic: Font::new(FONT_ITALIC.to_vec().into(), 0)
                .ok_or(PdfError::FontLoad("CourierPrime-Italic"))?,
            bold_italic: Font::new(FONT_BOLD_ITALIC.to_vec().into(), 0)
                .ok_or(PdfError::FontLoad("CourierPrime-BoldItalic"))?,
        })
    }

    fn for_style(&self, style: InlineStyle) -> &Font {
        match (style.bold, style.italic) {
            (false, false) => &self.regular,
            (true, false) => &self.bold,
            (false, true) => &self.italic,
            (true, true) => &self.bold_italic,
        }
    }
}

/// Verifies that all four embedded Courier Prime font faces contain valid glyphs
/// for every Turkish character in [`REQUIRED_TURKISH_CHARS`].
pub fn verify_turkish_glyphs() -> Result<(), PdfError> {
    match GLYPHS_VERIFIED.get_or_init(verify_turkish_glyphs_uncached) {
        Ok(()) => Ok(()),
        Err(PdfError::FontLoad(name)) => Err(PdfError::FontLoad(name)),
        Err(PdfError::MissingGlyph { font, ch }) => Err(PdfError::MissingGlyph { font, ch: *ch }),
        Err(PdfError::Krilla(m)) => Err(PdfError::Krilla(m.clone())),
        Err(PdfError::Io(e)) => Err(PdfError::Krilla(e.to_string())),
    }
}

fn verify_turkish_glyphs_uncached() -> Result<(), PdfError> {
    let faces = [
        ("CourierPrime-Regular", FONT_REGULAR),
        ("CourierPrime-Bold", FONT_BOLD),
        ("CourierPrime-Italic", FONT_ITALIC),
        ("CourierPrime-BoldItalic", FONT_BOLD_ITALIC),
    ];

    for (name, bytes) in faces {
        let face = Face::parse(bytes, 0).map_err(|_| PdfError::FontLoad(name))?;
        for ch in REQUIRED_TURKISH_CHARS.chars() {
            match face.glyph_index(ch) {
                Some(gid) if gid.0 != 0 => {}
                _ => return Err(PdfError::MissingGlyph { font: name, ch }),
            }
        }
    }
    Ok(())
}

/// Renders `doc` to a PDF byte buffer using `profile` and [`dram_core::paginate`].
pub fn export_pdf_bytes(doc: &Document, profile: &LayoutProfile) -> Result<Vec<u8>, PdfError> {
    verify_turkish_glyphs()?;
    let fonts = FontFamily::cached()?;
    let (page_w, page_h) = profile.page_dimensions_pt();
    let page_settings = PageSettings::from_wh(page_w, page_h)
        .ok_or_else(|| PdfError::Krilla("invalid page dimensions".to_string()))?;

    let mut pdf = KrillaDocument::new();

    // 1. Optional separate Title Page
    if has_title_page_content(&doc.title_page) {
        render_title_page(&mut pdf, page_settings.clone(), doc, profile, fonts);
    }

    // 2. Paginated script pages
    let paginated = paginate(doc, profile);
    for page_data in &paginated.pages {
        let mut page = pdf.start_page_with(page_settings.clone());
        let mut surface = page.surface();

        if page_data.show_page_number {
            let num_text = format!("{}.", page_data.page_number);
            let num_w = num_text.chars().count() as f32 * profile.char_width_pt;
            let num_x = page_w - profile.margin_right_in * 72.0 - num_w;
            draw_text_ltr(
                &mut surface,
                num_x,
                36.0,
                &fonts.regular,
                profile.font_size_pt,
                &num_text,
            );
        }

        for line in &page_data.lines {
            draw_paginated_line(&mut surface, line, profile, page_w, fonts);
        }

        surface.finish();
        page.finish();
    }

    pdf.finish()
        .map_err(|err| PdfError::Krilla(format!("{err:?}")))
}

/// Renders `doc` to a PDF file at `path` atomically.
pub fn export_pdf_file(
    path: &Path,
    doc: &Document,
    profile: &LayoutProfile,
) -> Result<(), PdfError> {
    let bytes = export_pdf_bytes(doc, profile)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("script.pdf");
    let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = parent.join(format!(".{file_name}.{}.{seq}.tmp", std::process::id()));

    let res = (|| -> Result<(), PdfError> {
        let mut f = File::create(&tmp_path)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    res
}

fn has_title_page_content(tp: &TitlePage) -> bool {
    !tp.title.trim().is_empty()
        || !tp.author.trim().is_empty()
        || !tp.credit.trim().is_empty()
        || !tp.source.trim().is_empty()
        || !tp.draft_date.trim().is_empty()
        || !tp.contact.trim().is_empty()
        || !tp.copyright.trim().is_empty()
        || !tp.extra.is_empty()
}

fn render_title_page(
    pdf: &mut KrillaDocument,
    page_settings: PageSettings,
    doc: &Document,
    profile: &LayoutProfile,
    fonts: &FontFamily,
) {
    let (page_w, page_h) = profile.page_dimensions_pt();
    let mut page = pdf.start_page_with(page_settings);
    let mut surface = page.surface();

    let tp = &doc.title_page;
    let center_y = page_h * 0.38;
    let mut y = center_y;

    if !tp.title.trim().is_empty() {
        for line in tp.title.lines() {
            draw_centered_text(&mut surface, line.trim(), y, page_w, &fonts.bold, profile);
            y += profile.line_height_pt * 1.4;
        }
        y += profile.line_height_pt;
    }

    let credit = if tp.credit.trim().is_empty() && !tp.author.trim().is_empty() {
        doc.settings.dictionary.label_written_by.as_str()
    } else {
        tp.credit.trim()
    };
    if !credit.is_empty() {
        draw_centered_text(&mut surface, credit, y, page_w, &fonts.regular, profile);
        y += profile.line_height_pt * 1.4;
    }

    if !tp.author.trim().is_empty() {
        for line in tp.author.lines() {
            draw_centered_text(
                &mut surface,
                line.trim(),
                y,
                page_w,
                &fonts.regular,
                profile,
            );
            y += profile.line_height_pt * 1.2;
        }
        y += profile.line_height_pt;
    }

    if !tp.source.trim().is_empty() {
        for line in tp.source.lines() {
            draw_centered_text(&mut surface, line.trim(), y, page_w, &fonts.italic, profile);
            y += profile.line_height_pt * 1.2;
        }
    }

    // Bottom-left: Contact & Copyright
    let left_x = profile.margin_left_in * 72.0;
    let mut bottom_left_lines: Vec<String> = Vec::new();
    for line in tp.contact.lines() {
        if !line.trim().is_empty() {
            bottom_left_lines.push(line.trim().to_string());
        }
    }
    for line in tp.copyright.lines() {
        if !line.trim().is_empty() {
            bottom_left_lines.push(line.trim().to_string());
        }
    }
    let bottom_base_y = page_h - profile.margin_bottom_in * 72.0;
    let mut bl_y =
        bottom_base_y - (bottom_left_lines.len().saturating_sub(1) as f32) * profile.line_height_pt;
    for line in &bottom_left_lines {
        draw_text_ltr(
            &mut surface,
            left_x,
            bl_y,
            &fonts.regular,
            profile.font_size_pt,
            line,
        );
        bl_y += profile.line_height_pt;
    }

    // Bottom-right: Draft date
    if !tp.draft_date.trim().is_empty() {
        let draft_text = format!(
            "{}: {}",
            doc.settings.dictionary.label_draft,
            tp.draft_date.trim()
        );
        let text_w = draft_text.chars().count() as f32 * profile.char_width_pt;
        let right_x = page_w - profile.margin_right_in * 72.0 - text_w;
        draw_text_ltr(
            &mut surface,
            right_x,
            bottom_base_y,
            &fonts.regular,
            profile.font_size_pt,
            &draft_text,
        );
    }

    surface.finish();
    page.finish();
}

fn draw_centered_text(
    surface: &mut krilla::surface::Surface<'_>,
    text: &str,
    y: f32,
    page_w: f32,
    font: &Font,
    profile: &LayoutProfile,
) {
    let text_w = text.chars().count() as f32 * profile.char_width_pt;
    let x = ((page_w - text_w) * 0.5).max(36.0);
    draw_text_ltr(surface, x, y, font, profile.font_size_pt, text);
}

fn draw_paginated_line(
    surface: &mut krilla::surface::Surface<'_>,
    line: &PaginatedLine,
    profile: &LayoutProfile,
    page_w: f32,
    fonts: &FontFamily,
) {
    // Optional scene numbers on left and right margins
    if let Some(sn) = &line.scene_number {
        draw_text_ltr(
            surface,
            54.0,
            line.y_pt,
            &fonts.bold,
            profile.font_size_pt,
            sn,
        );
        let sn_w = sn.chars().count() as f32 * profile.char_width_pt;
        draw_text_ltr(
            surface,
            page_w - 54.0 - sn_w,
            line.y_pt,
            &fonts.bold,
            profile.font_size_pt,
            sn,
        );
    }

    let total_chars: usize = line.runs.iter().map(|r| r.text.chars().count()).sum();
    let total_w = total_chars as f32 * profile.char_width_pt;

    let mut cur_x = match line.align {
        LineAlign::Left => line.x_pt,
        LineAlign::Center => line.x_pt + ((line.right_x_pt - line.x_pt - total_w) * 0.5).max(0.0),
        LineAlign::Right => (line.right_x_pt - total_w).max(line.x_pt),
    };

    for run in &line.runs {
        if run.text.is_empty() {
            continue;
        }
        draw_run(surface, run, cur_x, line.y_pt, profile, fonts);
        cur_x += run.text.chars().count() as f32 * profile.char_width_pt;
    }

    if line.has_revision {
        draw_text_ltr(
            surface,
            page_w - 36.0,
            line.y_pt,
            &fonts.bold,
            profile.font_size_pt,
            "*",
        );
    }
}

fn draw_run(
    surface: &mut krilla::surface::Surface<'_>,
    run: &TextRun,
    x: f32,
    y: f32,
    profile: &LayoutProfile,
    fonts: &FontFamily,
) {
    let font = fonts.for_style(run.style);
    draw_text_ltr(surface, x, y, font, profile.font_size_pt, &run.text);

    if run.style.underline {
        let w = run.text.chars().count() as f32 * profile.char_width_pt;
        let underline_y = y + 2.0;
        let mut pb = PathBuilder::new();
        pb.move_to(x, underline_y);
        pb.line_to(x + w, underline_y);
        if let Some(path) = pb.finish() {
            surface.set_stroke(Some(Stroke {
                width: 0.75,
                ..Default::default()
            }));
            surface.draw_path(&path);
            surface.set_stroke(None);
        }
    }
}

fn draw_text_ltr(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    font: &Font,
    font_size: f32,
    text: &str,
) {
    surface.draw_text(
        Point::from_xy(x, y),
        font.clone(),
        font_size,
        text,
        false,
        TextDirection::LeftToRight,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use dram_core::{Element, ElementType, PageSize, ScriptLanguage};

    #[test]
    fn courier_prime_supports_all_turkish_glyphs() {
        verify_turkish_glyphs().expect("Courier Prime must support all Turkish characters");
    }

    #[test]
    fn pdf_export_produces_valid_pdf_with_title_page_and_turkish_text() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.title_page.title = "İSTANBUL GÜNLÜĞÜ".to_string();
        doc.title_page.author = "Çiğdem Şöför".to_string();
        doc.title_page.draft_date = "16 Kasım 2026".to_string();
        doc.settings.show_scene_numbers = true;

        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"),
            Element::new(
                ElementType::Action,
                "İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ.",
            ),
            Element::new(ElementType::Character, "ÇİĞDEM"),
            Element::new(ElementType::Dialogue, "Merhaba İstanbul!"),
        ];
        doc.renumber_scenes();

        let bytes = export_pdf_bytes(&doc, &LayoutProfile::for_page_size(PageSize::A4)).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() > 2000);
    }
}
