//! Fade In (`.fadein`) Open Screenplay Format (OSF) ZIP+XML reader and writer.
//!
//! Verified structure: ZIP archive containing `document.xml` with
//! `<document type="Open Screenplay Format document">`, `<paragraphs>`, `<para>`,
//! `<style basestyle="..."/>`, `<text>`, and `<titlepage>`.

use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use dram_core::{Document, Element, ElementType, InlineStyle, ScriptLanguage, TextRun};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::FormatError;

const DOCUMENT_XML_ENTRY: &str = "document.xml";

/// Loads a `.fadein` file from `path` into a [`Document`].
pub fn load_fadein(path: &Path, script_language: ScriptLanguage) -> Result<Document, FormatError> {
    let file = File::open(path)?;
    read_fadein_from_reader(file, script_language)
}

/// Reads a `.fadein` ZIP archive from any `Read + Seek` source.
pub fn read_fadein_from_reader<R: Read + Seek>(
    reader: R,
    script_language: ScriptLanguage,
) -> Result<Document, FormatError> {
    let mut archive = ZipArchive::new(reader)?;
    let mut entry = match archive.by_name(DOCUMENT_XML_ENTRY) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => {
            return Err(FormatError::MissingEntry(DOCUMENT_XML_ENTRY.to_string()));
        }
        Err(e) => return Err(FormatError::Zip(e)),
    };
    let mut xml = String::new();
    entry.read_to_string(&mut xml)?;
    parse_osf_xml(&xml, script_language)
}

/// Writes `doc` as a `.fadein` ZIP archive to `path`.
pub fn save_fadein(path: &Path, doc: &Document) -> Result<(), FormatError> {
    let file = File::create(path)?;
    write_fadein_to_writer(file, doc)
}

/// Writes `doc` as a `.fadein` ZIP archive into any `Write + Seek` sink.
pub fn write_fadein_to_writer<W: Write + Seek>(
    writer: W,
    doc: &Document,
) -> Result<(), FormatError> {
    let xml = serialize_osf_xml(doc)?;
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(DOCUMENT_XML_ENTRY, options)?;
    zip.write_all(xml.as_bytes())?;
    zip.finish()?;
    Ok(())
}

/// Parses Open Screenplay Format XML (`document.xml`) into a [`Document`].
pub fn parse_osf_xml(xml: &str, script_language: ScriptLanguage) -> Result<Document, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut doc = Document::new(script_language);
    doc.elements.clear();

    let mut in_paragraphs = false;
    let mut in_para = false;
    let mut in_text = false;
    let mut in_titlepage = false;

    let mut current_kind = ElementType::Action;
    let mut current_runs: Vec<TextRun> = Vec::new();
    let mut current_style = InlineStyle::PLAIN;
    let mut current_title_key: Option<String> = None;
    let mut current_title_val = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "paragraphs" => in_paragraphs = true,
                    "titlepage" => in_titlepage = true,
                    "para" if in_paragraphs => {
                        in_para = true;
                        current_kind = ElementType::Action;
                        current_runs.clear();
                        current_style = InlineStyle::PLAIN;
                    }
                    "style" if in_para => {
                        if let Some(style_name) =
                            attr_value(&e, &["basestyle", "baseStyleName", "basestylename"])
                        {
                            current_kind = osf_style_to_element_type(&style_name);
                        }
                    }
                    "text" if in_para => {
                        in_text = true;
                        current_style = InlineStyle::PLAIN;
                        if attr_is_true(&e, "bold") {
                            current_style.bold = true;
                        }
                        if attr_is_true(&e, "italic") {
                            current_style.italic = true;
                        }
                        if attr_is_true(&e, "underline") {
                            current_style.underline = true;
                        }
                    }
                    "b" if in_text => current_style.bold = true,
                    "i" if in_text => current_style.italic = true,
                    "u" if in_text => current_style.underline = true,
                    other if in_titlepage => {
                        current_title_key = Some(other.to_string());
                        current_title_val.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "style" && in_para {
                    if let Some(style_name) =
                        attr_value(&e, &["basestyle", "baseStyleName", "basestylename"])
                    {
                        current_kind = osf_style_to_element_type(&style_name);
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| FormatError::Xml(err.to_string()))?
                    .into_owned();
                if in_text && !text.is_empty() {
                    push_styled_run(&mut current_runs, text, current_style);
                } else if in_titlepage && current_title_key.is_some() {
                    current_title_val.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "paragraphs" => in_paragraphs = false,
                    "titlepage" => in_titlepage = false,
                    "para" if in_para => {
                        in_para = false;
                        let mut el = Element::new(current_kind, "");
                        el.runs = std::mem::take(&mut current_runs);
                        doc.elements.push(el);
                    }
                    "text" if in_text => {
                        in_text = false;
                        current_style = InlineStyle::PLAIN;
                    }
                    "b" if in_text => current_style.bold = false,
                    "i" if in_text => current_style.italic = false,
                    "u" if in_text => current_style.underline = false,
                    other if in_titlepage && current_title_key.as_deref() == Some(other) => {
                        assign_osf_title_field(&mut doc, other, current_title_val.trim());
                        current_title_key = None;
                        current_title_val.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(FormatError::Xml(err.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if doc.elements.is_empty() {
        doc.elements
            .push(Element::new(ElementType::SceneHeading, ""));
    }
    Ok(doc)
}

/// Serializes a [`Document`] into Open Screenplay Format XML (`document.xml`).
pub fn serialize_osf_xml(doc: &Document) -> Result<String, FormatError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    let mut root = BytesStart::new("document");
    root.push_attribute(("type", "Open Screenplay Format document"));
    root.push_attribute(("version", "40"));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    // <paragraphs>
    writer
        .write_event(Event::Start(BytesStart::new("paragraphs")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    for el in &doc.elements {
        writer
            .write_event(Event::Start(BytesStart::new("para")))
            .map_err(|e| FormatError::Xml(e.to_string()))?;

        let mut style_tag = BytesStart::new("style");
        style_tag.push_attribute(("basestyle", element_type_to_osf_style(el.kind)));
        writer
            .write_event(Event::Empty(style_tag))
            .map_err(|e| FormatError::Xml(e.to_string()))?;

        if el.runs.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("text")))
                .map_err(|e| FormatError::Xml(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("text")))
                .map_err(|e| FormatError::Xml(e.to_string()))?;
        } else {
            for run in &el.runs {
                let mut text_tag = BytesStart::new("text");
                if run.style.bold {
                    text_tag.push_attribute(("bold", "1"));
                }
                if run.style.italic {
                    text_tag.push_attribute(("italic", "1"));
                }
                if run.style.underline {
                    text_tag.push_attribute(("underline", "1"));
                }
                writer
                    .write_event(Event::Start(text_tag))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(&run.text)))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new("text")))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
            }
        }

        writer
            .write_event(Event::End(BytesEnd::new("para")))
            .map_err(|e| FormatError::Xml(e.to_string()))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("paragraphs")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    // <titlepage>
    writer
        .write_event(Event::Start(BytesStart::new("titlepage")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;
    write_title_tag(&mut writer, "title", &doc.title_page.title)?;
    write_title_tag(&mut writer, "credit", &doc.title_page.credit)?;
    write_title_tag(&mut writer, "author", &doc.title_page.author)?;
    write_title_tag(&mut writer, "source", &doc.title_page.source)?;
    write_title_tag(&mut writer, "draftdate", &doc.title_page.draft_date)?;
    write_title_tag(&mut writer, "contact", &doc.title_page.contact)?;
    write_title_tag(&mut writer, "copyright", &doc.title_page.copyright)?;
    writer
        .write_event(Event::End(BytesEnd::new("titlepage")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    writer
        .write_event(Event::End(BytesEnd::new("document")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|e| FormatError::Xml(e.to_string()))
}

fn write_title_tag<W: Write>(
    writer: &mut Writer<W>,
    tag: &str,
    value: &str,
) -> Result<(), FormatError> {
    if value.trim().is_empty() {
        return Ok(());
    }
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| FormatError::Xml(e.to_string()))?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|e| FormatError::Xml(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| FormatError::Xml(e.to_string()))?;
    Ok(())
}

fn attr_value(e: &BytesStart<'_>, keys: &[&str]) -> Option<String> {
    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref());
        if keys
            .iter()
            .any(|candidate| k.eq_ignore_ascii_case(candidate))
        {
            if let Ok(v) = attr.unescape_value() {
                return Some(v.into_owned());
            }
        }
    }
    None
}

fn attr_is_true(e: &BytesStart<'_>, key: &str) -> bool {
    attr_value(e, &[key]).is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn push_styled_run(runs: &mut Vec<TextRun>, text: String, style: InlineStyle) {
    if let Some(last) = runs.last_mut() {
        if last.style == style {
            last.text.push_str(&text);
            return;
        }
    }
    runs.push(TextRun { text, style });
}

fn assign_osf_title_field(doc: &mut Document, tag: &str, val: &str) {
    match tag.to_ascii_lowercase().as_str() {
        "title" => doc.title_page.title = val.to_string(),
        "credit" => doc.title_page.credit = val.to_string(),
        "author" | "authors" => doc.title_page.author = val.to_string(),
        "source" => doc.title_page.source = val.to_string(),
        "draftdate" | "draft_date" => doc.title_page.draft_date = val.to_string(),
        "contact" => doc.title_page.contact = val.to_string(),
        "copyright" => doc.title_page.copyright = val.to_string(),
        _ => {}
    }
}

fn osf_style_to_element_type(style: &str) -> ElementType {
    match style.trim().to_ascii_lowercase().as_str() {
        "scene heading" => ElementType::SceneHeading,
        "action" => ElementType::Action,
        "character" => ElementType::Character,
        "parenthetical" => ElementType::Parenthetical,
        "dialogue" => ElementType::Dialogue,
        "transition" => ElementType::Transition,
        "shot" => ElementType::Shot,
        "centered" => ElementType::Centered,
        "lyrics" => ElementType::Lyrics,
        "synopsis" => ElementType::Synopsis,
        "note" => ElementType::Note,
        _ => ElementType::General,
    }
}

const fn element_type_to_osf_style(kind: ElementType) -> &'static str {
    match kind {
        ElementType::SceneHeading => "Scene Heading",
        ElementType::Action => "Action",
        ElementType::Character => "Character",
        ElementType::Parenthetical => "Parenthetical",
        ElementType::Dialogue => "Dialogue",
        ElementType::Transition => "Transition",
        ElementType::Shot => "Shot",
        ElementType::Centered => "Centered",
        ElementType::Lyrics => "Lyrics",
        ElementType::Synopsis => "Synopsis",
        ElementType::Note => "Note",
        ElementType::General | ElementType::PageBreak | ElementType::Section(_) => "General",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fadein_round_trip_preserves_elements_and_turkish() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.title_page.title = "İSTANBUL GÜNLÜĞÜ".to_string();
        doc.title_page.author = "Çiğdem Şöför".to_string();
        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"),
            Element::new(
                ElementType::Action,
                "İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ.",
            ),
            Element::new(ElementType::Character, "ÇİĞDEM"),
            Element::new(ElementType::Parenthetical, "(gülümseyerek)"),
            Element::new(ElementType::Dialogue, "Günaydın!"),
            Element::new(ElementType::Transition, "KESME."),
        ];

        let mut buf = Cursor::new(Vec::new());
        write_fadein_to_writer(&mut buf, &doc).unwrap();
        buf.set_position(0);

        let loaded = read_fadein_from_reader(buf, ScriptLanguage::Tr).unwrap();
        assert_eq!(loaded.title_page.title, doc.title_page.title);
        assert_eq!(loaded.title_page.author, doc.title_page.author);
        assert_eq!(loaded.elements.len(), doc.elements.len());
        for (a, b) in doc.elements.iter().zip(loaded.elements.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.plain_text(), b.plain_text());
        }
    }
}
