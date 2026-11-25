//! Final Draft (`.fdx`) XML reader and writer.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use dram_core::{Document, Element, ElementType, InlineStyle, ScriptLanguage, TextRun};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::dram_archive::{read_bounded, write_atomic_bytes};
use crate::error::FormatError;

const MAX_XML_BYTES: u64 = 32 * 1024 * 1024;

/// Loads a Final Draft `.fdx` file from `path`.
pub fn load_fdx(path: &Path, script_language: ScriptLanguage) -> Result<Document, FormatError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let raw = read_bounded(reader, "fdx", MAX_XML_BYTES)?;
    let xml = String::from_utf8(raw).map_err(|e| FormatError::Xml(e.to_string()))?;
    parse_fdx_xml(&xml, script_language)
}

/// Saves `doc` as a Final Draft `.fdx` file at `path`.
pub fn save_fdx(path: &Path, doc: &Document) -> Result<(), FormatError> {
    let xml = serialize_fdx_xml(doc)?;
    write_atomic_bytes(path, xml.as_bytes())
}

/// Parses Final Draft `.fdx` XML into a [`Document`].
pub fn parse_fdx_xml(xml: &str, script_language: ScriptLanguage) -> Result<Document, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut doc = Document::new(script_language);
    doc.elements.clear();

    let mut in_content = false;
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut current_kind = ElementType::Action;
    let mut current_runs: Vec<TextRun> = Vec::new();
    let mut current_style = InlineStyle::PLAIN;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "Content" => in_content = true,
                    "Paragraph" if in_content => {
                        in_paragraph = true;
                        current_kind = attr_value(&e, "Type")
                            .map_or(ElementType::Action, |t| fdx_type_to_element(&t));
                        current_runs.clear();
                    }
                    "Text" if in_paragraph => {
                        in_text = true;
                        current_style = attr_value(&e, "Style")
                            .map_or(InlineStyle::PLAIN, |s| parse_fdx_style(&s));
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_text => {
                let text = e
                    .xml10_content()
                    .map_err(|err| FormatError::Xml(err.to_string()))?
                    .into_owned();
                if !text.is_empty() {
                    push_run(&mut current_runs, text, current_style);
                }
            }
            Ok(Event::GeneralRef(e)) if in_text => {
                let text = if let Some(ch) = e
                    .resolve_char_ref()
                    .map_err(|err| FormatError::Xml(err.to_string()))?
                {
                    ch.to_string()
                } else {
                    let name = e
                        .decode()
                        .map_err(|err| FormatError::Xml(err.to_string()))?;
                    quick_xml::escape::resolve_predefined_entity(&name)
                        .unwrap_or(&name)
                        .to_string()
                };
                if !text.is_empty() {
                    push_run(&mut current_runs, text, current_style);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "Content" => in_content = false,
                    "Paragraph" if in_paragraph => {
                        in_paragraph = false;
                        let mut el = Element::new(current_kind, "");
                        el.runs = std::mem::take(&mut current_runs);
                        doc.elements.push(el);
                    }
                    "Text" if in_text => {
                        in_text = false;
                        current_style = InlineStyle::PLAIN;
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

/// Serializes a [`Document`] into Final Draft `.fdx` XML.
pub fn serialize_fdx_xml(doc: &Document) -> Result<String, FormatError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    let mut root = BytesStart::new("FinalDraft");
    root.push_attribute(("DocumentType", "Script"));
    root.push_attribute(("Template", "No"));
    root.push_attribute(("Version", "5"));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    writer
        .write_event(Event::Start(BytesStart::new("Content")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    for el in &doc.elements {
        let mut para = BytesStart::new("Paragraph");
        para.push_attribute(("Type", element_to_fdx_type(el.kind)));
        writer
            .write_event(Event::Start(para))
            .map_err(|e| FormatError::Xml(e.to_string()))?;

        if el.runs.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("Text")))
                .map_err(|e| FormatError::Xml(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("Text")))
                .map_err(|e| FormatError::Xml(e.to_string()))?;
        } else {
            for run in &el.runs {
                let mut text_el = BytesStart::new("Text");
                let style_str = format_fdx_style(run.style);
                if !style_str.is_empty() {
                    text_el.push_attribute(("Style", style_str.as_str()));
                }
                writer
                    .write_event(Event::Start(text_el))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(&run.text)))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new("Text")))
                    .map_err(|e| FormatError::Xml(e.to_string()))?;
            }
        }

        writer
            .write_event(Event::End(BytesEnd::new("Paragraph")))
            .map_err(|e| FormatError::Xml(e.to_string()))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Content")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new("FinalDraft")))
        .map_err(|e| FormatError::Xml(e.to_string()))?;

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|e| FormatError::Xml(e.to_string()))
}

fn attr_value(e: &BytesStart<'_>, key: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref());
        if k.eq_ignore_ascii_case(key) {
            if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                return Some(v.into_owned());
            }
        }
    }
    None
}

fn push_run(runs: &mut Vec<TextRun>, text: String, style: InlineStyle) {
    if let Some(last) = runs.last_mut() {
        if last.style == style {
            last.text.push_str(&text);
            return;
        }
    }
    runs.push(TextRun { text, style });
}

fn parse_fdx_style(style: &str) -> InlineStyle {
    let mut out = InlineStyle::PLAIN;
    for part in style.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "bold" => out.bold = true,
            "italic" => out.italic = true,
            "underline" => out.underline = true,
            _ => {}
        }
    }
    out
}

fn format_fdx_style(style: InlineStyle) -> String {
    let mut parts = Vec::new();
    if style.bold {
        parts.push("Bold");
    }
    if style.italic {
        parts.push("Italic");
    }
    if style.underline {
        parts.push("Underline");
    }
    parts.join("+")
}

fn fdx_type_to_element(t: &str) -> ElementType {
    match t.trim().to_ascii_lowercase().as_str() {
        "scene heading" => ElementType::SceneHeading,
        "action" => ElementType::Action,
        "character" => ElementType::Character,
        "parenthetical" => ElementType::Parenthetical,
        "dialogue" => ElementType::Dialogue,
        "transition" => ElementType::Transition,
        "shot" => ElementType::Shot,
        _ => ElementType::General,
    }
}

const fn element_to_fdx_type(kind: ElementType) -> &'static str {
    match kind {
        ElementType::SceneHeading => "Scene Heading",
        ElementType::Action => "Action",
        ElementType::Character => "Character",
        ElementType::Parenthetical => "Parenthetical",
        ElementType::Dialogue => "Dialogue",
        ElementType::Transition => "Transition",
        ElementType::Shot => "Shot",
        _ => "General",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fdx_round_trip_preserves_elements_and_turkish() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"),
            Element::new(
                ElementType::Action,
                "İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ.",
            ),
            Element::new(ElementType::Character, "ÇİĞDEM"),
            Element::new(ElementType::Dialogue, "Merhaba!"),
        ];

        let xml = serialize_fdx_xml(&doc).unwrap();
        let parsed = parse_fdx_xml(&xml, ScriptLanguage::Tr).unwrap();
        assert_eq!(parsed.elements.len(), doc.elements.len());
        for (a, b) in doc.elements.iter().zip(parsed.elements.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.plain_text(), b.plain_text());
        }
    }

    #[test]
    fn fdx_round_trip_preserves_shot_general_inline_styles_and_entities() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        let mut styled_dialogue = Element::new(ElementType::Dialogue, "");
        styled_dialogue.runs = vec![
            TextRun {
                text: "Kalın & ".to_string(),
                style: InlineStyle {
                    bold: true,
                    italic: false,
                    underline: false,
                },
            },
            TextRun {
                text: "<İtalik>".to_string(),
                style: InlineStyle {
                    bold: false,
                    italic: true,
                    underline: true,
                },
            },
        ];

        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "DIŞ. KÖPRÜ - GECE"),
            Element::new(ElementType::Shot, "YAKIN PLAN - SAAT"),
            Element::new(ElementType::Character, "KEMAL"),
            Element::new(ElementType::Parenthetical, "(saate bakarak)"),
            styled_dialogue,
            Element::new(ElementType::Transition, "KESME."),
            Element::new(ElementType::General, "Genel yapım notu"),
        ];

        let xml = serialize_fdx_xml(&doc).unwrap();
        let parsed = parse_fdx_xml(&xml, ScriptLanguage::Tr).unwrap();
        assert_eq!(parsed.elements.len(), doc.elements.len());
        for (a, b) in doc.elements.iter().zip(parsed.elements.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.runs, b.runs);
        }
    }
}
