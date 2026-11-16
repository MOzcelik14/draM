//! Fountain 1.1 parser and serializer.

use dram_core::text::to_upper_for_lang;
use dram_core::{
    Document, DocumentSettings, DualDialogueGroup, DualSide, Element, ElementType, InlineNote,
    InlineStyle, SceneNumber, ScriptLanguage, TextRun, TitlePage,
};
use uuid::Uuid;

/// Parses a Fountain 1.1 string into a [`Document`].
#[must_use]
pub fn parse_fountain(input: &str, script_language: ScriptLanguage) -> Document {
    let mut doc = Document::new(script_language);
    doc.elements.clear();

    // Normalize line endings and strip Fountain boneyard comments (`/* ... */`).
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let cleaned = strip_boneyard(&normalized);

    let mut lines: Vec<&str> = cleaned.split('\n').collect();

    // Parse optional Title Page at the top of the document.
    let body_start = parse_title_page(&lines, &mut doc.title_page);
    if body_start > 0 {
        lines.drain(0..body_start);
    }

    let mut idx = 0;
    let mut in_dialogue = false;
    let mut last_character_idx: Option<usize> = None;

    while idx < lines.len() {
        let raw_line = lines[idx];
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            in_dialogue = false;
            idx += 1;
            continue;
        }

        // Page break: `===` (three or more `=`)
        if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '=') {
            in_dialogue = false;
            doc.elements.push(Element::new(ElementType::PageBreak, ""));
            idx += 1;
            continue;
        }

        // Section: `# Heading` (1..=6 `#` chars)
        if let Some((depth, text)) = parse_section(trimmed) {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Section(depth), "");
            parse_inline_markup(text, &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Synopsis: `= text`
        if let Some(rest) = trimmed.strip_prefix('=') {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Synopsis, "");
            parse_inline_markup(rest.trim_start(), &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Standalone Note: `[[ note ]]`
        if trimmed.starts_with("[[") && trimmed.ends_with("]]") && trimmed.len() >= 4 {
            in_dialogue = false;
            let note_text = trimmed[2..trimmed.len() - 2].trim();
            doc.elements
                .push(Element::new(ElementType::Note, note_text));
            idx += 1;
            continue;
        }

        // Centered: `> text <`
        if trimmed.starts_with('>') && trimmed.ends_with('<') && trimmed.len() >= 2 {
            in_dialogue = false;
            let inner = trimmed[1..trimmed.len() - 1].trim();
            let mut el = Element::new(ElementType::Centered, "");
            parse_inline_markup(inner, &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Forced Transition: `> text` (not ending in `<`)
        if let Some(rest) = trimmed.strip_prefix('>') {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Transition, "");
            parse_inline_markup(rest.trim_start(), &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Lyrics: `~ text`
        if let Some(rest) = trimmed.strip_prefix('~') {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Lyrics, "");
            parse_inline_markup(rest.trim_start(), &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        let prev_blank = idx == 0 || lines[idx - 1].trim().is_empty();
        let next_blank = idx + 1 >= lines.len() || lines[idx + 1].trim().is_empty();

        // Forced Action: `!text`
        if let Some(rest) = raw_line.strip_prefix('!') {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Action, "");
            parse_inline_markup(rest, &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Scene Heading: forced `.` (not `..`) or recognized prefix when preceded by blank line
        if is_forced_scene_heading(trimmed)
            || (prev_blank && doc.settings.is_scene_heading_prefix(trimmed))
        {
            in_dialogue = false;
            let heading_raw = if is_forced_scene_heading(trimmed) {
                trimmed[1..].trim_start()
            } else {
                trimmed
            };
            let (clean_heading, scene_num) = extract_scene_number(heading_raw);
            let mut el = Element::new(ElementType::SceneHeading, "");
            parse_inline_markup(clean_heading, &mut el);
            if let Some(num) = scene_num {
                el.scene_number = Some(SceneNumber {
                    value: num,
                    locked: false,
                });
            }
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Standard Transition: uppercase ending in `TO:` surrounded by blank lines
        if prev_blank && next_blank && is_standard_transition(trimmed, script_language) {
            in_dialogue = false;
            let mut el = Element::new(ElementType::Transition, "");
            parse_inline_markup(trimmed, &mut el);
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Dialogue / Parenthetical inside a dialogue block
        if in_dialogue {
            if trimmed.starts_with('(') && trimmed.ends_with(')') {
                let mut el = Element::new(ElementType::Parenthetical, "");
                parse_inline_markup(trimmed, &mut el);
                doc.elements.push(el);
            } else {
                let mut el = Element::new(ElementType::Dialogue, "");
                parse_inline_markup(trimmed, &mut el);
                doc.elements.push(el);
            }
            idx += 1;
            continue;
        }

        // Character cue: preceded by blank line, followed by non-blank line, and either forced `@` or uppercase
        if prev_blank && !next_blank && is_character_cue(trimmed, script_language) {
            in_dialogue = true;
            let mut cue = trimmed.strip_prefix('@').unwrap_or(trimmed).trim();
            let mut dual_right = false;
            if let Some(without_caret) = cue.strip_suffix('^') {
                dual_right = true;
                cue = without_caret.trim_end();
            }

            let mut el = Element::new(ElementType::Character, "");
            parse_inline_markup(cue, &mut el);

            if dual_right {
                let group_id = Uuid::new_v4();
                if let Some(prev_char_idx) = last_character_idx {
                    // Tag previous character and its following dialogue/parentheticals as Left.
                    for prev_el in &mut doc.elements[prev_char_idx..] {
                        if matches!(
                            prev_el.kind,
                            ElementType::Character
                                | ElementType::Dialogue
                                | ElementType::Parenthetical
                        ) {
                            prev_el.dual_dialogue = Some(DualDialogueGroup {
                                group_id,
                                side: DualSide::Left,
                            });
                        } else {
                            break;
                        }
                    }
                }
                el.dual_dialogue = Some(DualDialogueGroup {
                    group_id,
                    side: DualSide::Right,
                });
            }

            last_character_idx = Some(doc.elements.len());
            doc.elements.push(el);
            idx += 1;
            continue;
        }

        // Default: Action
        let mut el = Element::new(ElementType::Action, "");
        parse_inline_markup(raw_line, &mut el);
        doc.elements.push(el);
        idx += 1;
    }

    // Propagate Right dual dialogue group to dialogue/parentheticals immediately following a Right character.
    propagate_right_dual_dialogue(&mut doc.elements);

    doc
}

fn propagate_right_dual_dialogue(elements: &mut [Element]) {
    let mut active_right_group: Option<DualDialogueGroup> = None;
    for el in elements {
        match el.kind {
            ElementType::Character => {
                active_right_group = el.dual_dialogue.filter(|g| g.side == DualSide::Right);
            }
            ElementType::Dialogue | ElementType::Parenthetical => {
                if el.dual_dialogue.is_none() {
                    el.dual_dialogue = active_right_group;
                }
            }
            _ => {
                active_right_group = None;
            }
        }
    }
}

/// Serializes a [`Document`] into Fountain 1.1 format.
#[must_use]
pub fn serialize_fountain(doc: &Document) -> String {
    let mut out = String::new();

    // Title Page
    let tp_block = serialize_title_page(&doc.title_page);
    if !tp_block.is_empty() {
        out.push_str(&tp_block);
        out.push('\n');
    }

    let mut prev_kind: Option<ElementType> = None;
    let mut prev_was_dual_left = false;

    for (i, el) in doc.elements.iter().enumerate() {
        let is_dialogue_child =
            matches!(el.kind, ElementType::Dialogue | ElementType::Parenthetical);

        if i > 0 && !is_dialogue_child {
            // Blank line before non-dialogue-continuation blocks
            if !out.ends_with("\n\n") && !out.is_empty() {
                out.push('\n');
            }
        } else if is_dialogue_child
            && prev_kind.is_some()
            && !matches!(
                prev_kind,
                Some(ElementType::Character | ElementType::Dialogue | ElementType::Parenthetical)
            )
            && !out.ends_with("\n\n")
        {
            out.push('\n');
        }

        let content = serialize_runs_and_notes(&el.runs, &el.inline_notes);

        match el.kind {
            ElementType::SceneHeading => {
                if !doc.settings.is_scene_heading_prefix(&content) && !content.starts_with('.') {
                    out.push('.');
                }
                out.push_str(&content);
                if let Some(num) = &el.scene_number {
                    out.push_str(" #");
                    out.push_str(&num.value);
                    out.push('#');
                }
                out.push('\n');
            }
            ElementType::Action | ElementType::General => {
                if needs_forced_action(&content, &doc.settings) {
                    out.push('!');
                }
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Character => {
                let upper_check = to_upper_for_lang(&content, doc.settings.script_language);
                if content != upper_check || !content.chars().any(char::is_alphabetic) {
                    out.push('@');
                }
                out.push_str(&content);
                if let Some(dual) = el.dual_dialogue {
                    if dual.side == DualSide::Right && prev_was_dual_left {
                        out.push_str(" ^");
                    }
                    prev_was_dual_left = dual.side == DualSide::Left;
                } else {
                    prev_was_dual_left = false;
                }
                out.push('\n');
            }
            ElementType::Parenthetical | ElementType::Dialogue => {
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Transition => {
                if !is_standard_transition(&content, doc.settings.script_language) {
                    out.push_str("> ");
                }
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Shot => {
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Centered => {
                out.push_str("> ");
                out.push_str(&content);
                out.push_str(" <\n");
            }
            ElementType::Lyrics => {
                out.push_str("~ ");
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::PageBreak => {
                out.push_str("===\n");
            }
            ElementType::Section(depth) => {
                let hashes = "#".repeat(depth.clamp(1, 6) as usize);
                out.push_str(&hashes);
                out.push(' ');
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Synopsis => {
                out.push_str("= ");
                out.push_str(&content);
                out.push('\n');
            }
            ElementType::Note => {
                out.push_str("[[");
                out.push_str(&content);
                out.push_str("]]\n");
            }
        }

        prev_kind = Some(el.kind);
    }

    out
}

fn needs_forced_action(content: &str, settings: &DocumentSettings) -> bool {
    let trimmed = content.trim();
    !trimmed.is_empty()
        && (settings.is_scene_heading_prefix(trimmed)
            || trimmed.starts_with('.')
            || trimmed.starts_with('>')
            || trimmed.starts_with('~')
            || trimmed.starts_with('=')
            || trimmed.starts_with('#'))
}

fn strip_boneyard(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        if let Some(end) = after_start.find("*/") {
            rest = &after_start[end + 2..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

fn parse_title_page(lines: &[&str], tp: &mut TitlePage) -> usize {
    if lines.is_empty() {
        return 0;
    }
    // First line must be `Key: value` or `Key:`
    let first = lines[0];
    if !looks_like_title_key(first) {
        return 0;
    }

    let mut current_key: Option<String> = None;
    let mut current_val = String::new();
    let mut idx = 0;

    while idx < lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            if let Some(k) = current_key.take() {
                assign_title_field(tp, &k, current_val.trim_end());
                current_val.clear();
            }
            return idx + 1;
        }
        if (line.starts_with(' ') || line.starts_with('\t')) && current_key.is_some() {
            if !current_val.is_empty() {
                current_val.push('\n');
            }
            current_val.push_str(line.trim());
        } else if let Some((k, v)) = split_title_kv(line) {
            if let Some(prev_k) = current_key.take() {
                assign_title_field(tp, &prev_k, current_val.trim_end());
                current_val.clear();
            }
            current_key = Some(k.to_string());
            current_val.push_str(v.trim());
        } else {
            // Not a valid title page; abort.
            *tp = TitlePage::default();
            return 0;
        }
        idx += 1;
    }

    if let Some(k) = current_key.take() {
        assign_title_field(tp, &k, current_val.trim_end());
    }
    idx
}

fn looks_like_title_key(line: &str) -> bool {
    split_title_kv(line).is_some()
}

fn split_title_kv(line: &str) -> Option<(&str, &str)> {
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let colon = line.find(':')?;
    let key = line[..colon].trim();
    if key.is_empty() || key.contains('.') {
        return None;
    }
    Some((key, &line[colon + 1..]))
}

fn assign_title_field(tp: &mut TitlePage, key: &str, value: &str) {
    match key.to_ascii_lowercase().as_str() {
        "title" => tp.title = value.to_string(),
        "credit" => tp.credit = value.to_string(),
        "author" | "authors" => tp.author = value.to_string(),
        "source" => tp.source = value.to_string(),
        "draft date" => tp.draft_date = value.to_string(),
        "contact" => tp.contact = value.to_string(),
        "copyright" => tp.copyright = value.to_string(),
        _ => tp.extra.push((key.to_string(), value.to_string())),
    }
}

fn serialize_title_page(tp: &TitlePage) -> String {
    let mut out = String::new();
    let pairs = [
        ("Title", &tp.title),
        ("Credit", &tp.credit),
        ("Author", &tp.author),
        ("Source", &tp.source),
        ("Draft date", &tp.draft_date),
        ("Contact", &tp.contact),
        ("Copyright", &tp.copyright),
    ];
    for (k, v) in pairs {
        if !v.is_empty() {
            write_title_kv(&mut out, k, v);
        }
    }
    for (k, v) in &tp.extra {
        if !v.is_empty() {
            write_title_kv(&mut out, k, v);
        }
    }
    out
}

fn write_title_kv(out: &mut String, key: &str, val: &str) {
    out.push_str(key);
    out.push(':');
    if val.contains('\n') {
        out.push('\n');
        for line in val.lines() {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    } else {
        out.push(' ');
        out.push_str(val);
        out.push('\n');
    }
}

fn parse_section(trimmed: &str) -> Option<(u8, &str)> {
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    Some((hashes as u8, trimmed[hashes..].trim_start()))
}

fn is_forced_scene_heading(trimmed: &str) -> bool {
    trimmed.starts_with('.') && !trimmed.starts_with("..") && trimmed.len() > 1
}

fn extract_scene_number(heading: &str) -> (&str, Option<String>) {
    if let Some(last_hash) = heading.rfind('#') {
        let before_last = &heading[..last_hash];
        if let Some(first_hash) = before_last.rfind('#') {
            let candidate = &heading[first_hash + 1..last_hash];
            if !candidate.is_empty() && !candidate.contains(char::is_whitespace) {
                let clean = heading[..first_hash].trim_end();
                return (clean, Some(candidate.to_string()));
            }
        }
    }
    (heading, None)
}

fn is_standard_transition(trimmed: &str, lang: ScriptLanguage) -> bool {
    let upper = to_upper_for_lang(trimmed, lang);
    trimmed == upper && (trimmed.ends_with("TO:") || trimmed.ends_with("KESME."))
}

fn is_character_cue(trimmed: &str, lang: ScriptLanguage) -> bool {
    if trimmed.starts_with('@') {
        return trimmed.len() > 1;
    }
    let without_caret = trimmed.strip_suffix('^').unwrap_or(trimmed).trim_end();
    if without_caret.is_empty() {
        return false;
    }
    // Strip optional parenthetical extension like `(V.O.)` or `(DEVAM)` when checking uppercase
    let has_alpha = without_caret.chars().any(char::is_alphabetic);
    has_alpha && without_caret == to_upper_for_lang(without_caret, lang)
}

/// Parses Fountain inline emphasis (`*`, `**`, `***`, `_`, `\` escape) and inline notes (`[[ ]]`).
fn parse_inline_markup(input: &str, el: &mut Element) {
    // First extract inline `[[note]]` blocks and record their byte offsets in clean markup.
    let mut markup = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("[[") {
        markup.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let note_body = after[..end].trim().to_string();
            el.inline_notes.push(InlineNote {
                id: Uuid::new_v4(),
                offset: markup.len(),
                text: note_body,
                color: None,
            });
            rest = &after[end + 2..];
        } else {
            markup.push_str("[[");
            rest = after;
        }
    }
    markup.push_str(rest);

    let runs = parse_emphasis_runs(&markup);
    el.runs = runs;
}

fn parse_emphasis_runs(input: &str) -> Vec<TextRun> {
    let mut runs: Vec<TextRun> = Vec::new();
    let mut current = String::new();
    let mut style = InlineStyle::PLAIN;
    let bytes = input.as_bytes();
    let mut i = 0;

    let flush = |runs: &mut Vec<TextRun>, buf: &mut String, st: InlineStyle| {
        if !buf.is_empty() {
            runs.push(TextRun {
                text: std::mem::take(buf),
                style: st,
            });
        }
    };

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let next_char = input[i + 1..].chars().next().unwrap();
            current.push(next_char);
            i += 1 + next_char.len_utf8();
            continue;
        }

        if input[i..].starts_with("***") {
            flush(&mut runs, &mut current, style);
            style.bold = !style.bold;
            style.italic = !style.italic;
            i += 3;
            continue;
        }
        if input[i..].starts_with("**") {
            flush(&mut runs, &mut current, style);
            style.bold = !style.bold;
            i += 2;
            continue;
        }
        if bytes[i] == b'*' {
            flush(&mut runs, &mut current, style);
            style.italic = !style.italic;
            i += 1;
            continue;
        }
        if bytes[i] == b'_' {
            flush(&mut runs, &mut current, style);
            style.underline = !style.underline;
            i += 1;
            continue;
        }

        let ch = input[i..].chars().next().unwrap();
        current.push(ch);
        i += ch.len_utf8();
    }

    flush(&mut runs, &mut current, style);
    runs
}

fn serialize_runs_and_notes(runs: &[TextRun], notes: &[InlineNote]) -> String {
    let mut out = String::new();
    for run in runs {
        let prefix = style_marker(run.style);
        out.push_str(prefix);
        for ch in run.text.chars() {
            if matches!(ch, '*' | '_' | '\\') {
                out.push('\\');
            }
            out.push(ch);
        }
        out.push_str(prefix);
    }
    for note in notes {
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str("[[");
        out.push_str(&note.text);
        out.push_str("]]");
    }
    out
}

const fn style_marker(style: InlineStyle) -> &'static str {
    match (style.bold, style.italic, style.underline) {
        (true, true, _) => "***",
        (true, false, _) => "**",
        (false, true, _) => "*",
        (false, false, true) => "_",
        (false, false, false) => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fountain_round_trip_covers_all_elements_and_turkish() {
        let source = "\
Title: İSTANBUL GECE
Credit: Yazan
Author: Çiğdem Şöför

# Birinci Perde

= Kahramanımız şehre varır.

İÇ. İSTANBUL TREN GARI - GÜN #1A#

!İSTANBUL ışık ILIK iğne ÇİĞDEM öğrenci ŞÖFÖR Ğ.

ÇİĞDEM
(gülümseyerek)
**Hoş bulduk** *İstanbul*! [[buraya müzik]]

ALİ ^
Ben de buradayım!

> KESME.

> SON <

~ Şarkı sözleri burada

===
";

        let doc = parse_fountain(source, ScriptLanguage::Tr);
        assert_eq!(doc.title_page.title, "İSTANBUL GECE");
        assert_eq!(doc.title_page.author, "Çiğdem Şöför");

        let exported = serialize_fountain(&doc);
        let reparsed = parse_fountain(&exported, ScriptLanguage::Tr);

        assert_eq!(reparsed.title_page, doc.title_page);
        assert_eq!(reparsed.elements.len(), doc.elements.len());
        for (a, b) in doc.elements.iter().zip(reparsed.elements.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.plain_text(), b.plain_text());
            assert_eq!(a.scene_number, b.scene_number);
            assert_eq!(a.dual_dialogue.is_some(), b.dual_dialogue.is_some());
        }
    }

    #[test]
    fn fountain_strips_boneyard_and_handles_escapes() {
        let source = "/* gizli not\niki satır */\n\\*yıldızlı\\* normal";
        let doc = parse_fountain(source, ScriptLanguage::En);
        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].plain_text(), "*yıldızlı* normal");
    }
}
