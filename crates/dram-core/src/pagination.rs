//! Screenplay pagination engine shared by the live editor and PDF exporter.

use crate::model::{
    Document, DualDialogueGroup, DualSide, ElementType, InlineStyle, PageSize, TextRun,
};
use crate::text::to_upper_for_lang;

const PT_PER_INCH: f32 = 72.0;

/// Horizontal alignment for a paginated line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineAlign {
    /// Left-aligned at `x_pt`.
    Left,
    /// Centered between left and right bounds.
    Center,
    /// Right-aligned ending at `right_x_pt`.
    Right,
}

/// Configurable screenplay layout profile (defaults to industry-standard Courier 12pt).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutProfile {
    /// Target physical page size.
    pub page_size: PageSize,
    /// Font size in points (12.0 pt).
    pub font_size_pt: f32,
    /// Line height in points (12.0 pt -> ~54-55 lines on US Letter).
    pub line_height_pt: f32,
    /// Fixed-width character advance in points (Courier 12pt = 7.2 pt, 10 chars/in).
    pub char_width_pt: f32,
    /// Top margin in inches.
    pub margin_top_in: f32,
    /// Bottom margin in inches.
    pub margin_bottom_in: f32,
    /// Default page left margin in inches.
    pub margin_left_in: f32,
    /// Default page right margin in inches.
    pub margin_right_in: f32,
    /// Left edge of Action / SceneHeading in inches from page left.
    pub action_left_in: f32,
    /// Right margin of Action / SceneHeading in inches from page right.
    pub action_right_in: f32,
    /// Left edge of Dialogue in inches from page left.
    pub dialogue_left_in: f32,
    /// Right edge of Dialogue in inches from page left (6.0" on US Letter).
    pub dialogue_right_from_left_in: f32,
    /// Left edge of Parenthetical in inches from page left.
    pub parenthetical_left_in: f32,
    /// Right edge of Parenthetical in inches from page left.
    pub parenthetical_right_from_left_in: f32,
    /// Left edge of Character cue in inches from page left.
    pub character_left_in: f32,
    /// Right margin of Transition in inches from page right.
    pub transition_right_in: f32,
}

impl Default for LayoutProfile {
    fn default() -> Self {
        Self::for_page_size(PageSize::default())
    }
}

impl LayoutProfile {
    /// Returns the standard screenplay [`LayoutProfile`] for `page_size`.
    #[must_use]
    pub const fn for_page_size(page_size: PageSize) -> Self {
        Self {
            page_size,
            font_size_pt: 12.0,
            line_height_pt: 12.0,
            char_width_pt: 7.2,
            margin_top_in: 1.0,
            margin_bottom_in: 1.0,
            margin_left_in: 1.5,
            margin_right_in: 1.0,
            action_left_in: 1.5,
            action_right_in: 1.0,
            dialogue_left_in: 2.5,
            dialogue_right_from_left_in: 6.0,
            parenthetical_left_in: 3.1,
            parenthetical_right_from_left_in: 5.6,
            character_left_in: 3.7,
            transition_right_in: 1.0,
        }
    }

    /// Returns `(width_pt, height_pt)` for the configured [`PageSize`].
    #[must_use]
    pub const fn page_dimensions_pt(&self) -> (f32, f32) {
        match self.page_size {
            // 210mm x 297mm
            PageSize::A4 => (595.28, 841.89),
            // 8.5in x 11in
            PageSize::UsLetter => (612.0, 792.0),
        }
    }

    /// Returns the maximum number of body lines per page.
    #[must_use]
    pub fn lines_per_page(&self) -> usize {
        let (_, page_h) = self.page_dimensions_pt();
        let printable_h =
            (page_h - (self.margin_top_in + self.margin_bottom_in) * PT_PER_INCH).max(12.0);
        (printable_h / self.line_height_pt).floor() as usize
    }

    /// Returns `(left_x_pt, right_x_pt, align)` for an [`ElementType`].
    #[must_use]
    pub fn column_bounds_pt(&self, kind: ElementType) -> (f32, f32, LineAlign) {
        let (page_w, _) = self.page_dimensions_pt();
        let default_right = page_w - self.margin_right_in * PT_PER_INCH;
        match kind {
            ElementType::Dialogue | ElementType::Lyrics => (
                self.dialogue_left_in * PT_PER_INCH,
                (self.dialogue_right_from_left_in * PT_PER_INCH).min(default_right),
                LineAlign::Left,
            ),
            ElementType::Parenthetical => (
                self.parenthetical_left_in * PT_PER_INCH,
                (self.parenthetical_right_from_left_in * PT_PER_INCH).min(default_right),
                LineAlign::Left,
            ),
            ElementType::Character => (
                self.character_left_in * PT_PER_INCH,
                default_right,
                LineAlign::Left,
            ),
            ElementType::Transition => (
                self.action_left_in * PT_PER_INCH,
                page_w - self.transition_right_in * PT_PER_INCH,
                LineAlign::Right,
            ),
            ElementType::Centered => (
                self.action_left_in * PT_PER_INCH,
                default_right,
                LineAlign::Center,
            ),
            _ => (
                self.action_left_in * PT_PER_INCH,
                page_w - self.action_right_in * PT_PER_INCH,
                LineAlign::Left,
            ),
        }
    }

    /// Returns `(left_x_pt, right_x_pt, align)` for a side-by-side dual-dialogue column.
    #[must_use]
    pub fn dual_column_bounds_pt(
        &self,
        side: DualSide,
        kind: ElementType,
    ) -> (f32, f32, LineAlign) {
        let (page_w, _) = self.page_dimensions_pt();
        let left_edge = self.margin_left_in * PT_PER_INCH;
        let right_edge = page_w - self.margin_right_in * PT_PER_INCH;
        let gutter_pt = 18.0;
        let col_w = ((right_edge - left_edge - gutter_pt) * 0.5).max(self.char_width_pt * 12.0);
        let (col_l, col_r) = match side {
            DualSide::Left => (left_edge, left_edge + col_w),
            DualSide::Right => (left_edge + col_w + gutter_pt, right_edge),
        };
        match kind {
            ElementType::Character => (col_l, col_r, LineAlign::Center),
            ElementType::Parenthetical => {
                let indent = self.char_width_pt * 2.0;
                (
                    col_l + indent,
                    (col_r - indent).max(col_l + indent + self.char_width_pt * 4.0),
                    LineAlign::Left,
                )
            }
            _ => (col_l, col_r, LineAlign::Left),
        }
    }

    fn max_chars_for(&self, kind: ElementType) -> usize {
        let (left, right, _) = self.column_bounds_pt(kind);
        let width_pt = (right - left).max(self.char_width_pt);
        (width_pt / self.char_width_pt).floor().max(1.0) as usize
    }

    fn dual_max_chars_for(&self, side: DualSide, kind: ElementType) -> usize {
        let (left, right, _) = self.dual_column_bounds_pt(side, kind);
        let width_pt = (right - left).max(self.char_width_pt);
        (width_pt / self.char_width_pt).floor().max(1.0) as usize
    }
}

/// A single rendered line on a paginated page.
#[derive(Debug, Clone, PartialEq)]
pub struct PaginatedLine {
    /// Source element index in [`Document::elements`], if any.
    pub element_index: Option<usize>,
    /// Element type of this line.
    pub kind: ElementType,
    /// Styled text runs for this line.
    pub runs: Vec<TextRun>,
    /// Left X coordinate in points from page left.
    pub x_pt: f32,
    /// Right X coordinate in points from page left.
    pub right_x_pt: f32,
    /// Baseline Y coordinate in points from page top.
    pub y_pt: f32,
    /// Horizontal alignment.
    pub align: LineAlign,
    /// Optional scene number (attached to the first line of a `SceneHeading`).
    pub scene_number: Option<String>,
    /// Whether this line belongs to a revised element (prints `*` in the right margin).
    pub has_revision: bool,
}

/// A single paginated screenplay page.
#[derive(Debug, Clone, PartialEq)]
pub struct PaginatedPage {
    /// 1-based script page number.
    pub page_number: usize,
    /// Whether the page number should be printed (false on page 1).
    pub show_page_number: bool,
    /// Lines on this page.
    pub lines: Vec<PaginatedLine>,
}

/// Complete pagination result for a [`Document`].
#[derive(Debug, Clone, PartialEq)]
pub struct PaginatedDocument {
    /// Ordered script pages (title page is separate).
    pub pages: Vec<PaginatedPage>,
}

impl PaginatedDocument {
    /// Total number of script pages (at least 1).
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len().max(1)
    }

    /// Estimated screenplay runtime in minutes (industry rule of thumb: 1 page ≈ 1 minute).
    #[must_use]
    pub fn estimated_minutes(&self) -> usize {
        self.page_count()
    }
}

#[derive(Debug, Clone)]
struct WrappedBlock {
    element_index: usize,
    kind: ElementType,
    lines: Vec<Vec<TextRun>>,
    blank_before: usize,
    scene_number: Option<String>,
    dual_dialogue: Option<DualDialogueGroup>,
    has_revision: bool,
}

/// Paginates `doc` using `profile`.
#[must_use]
pub fn paginate(doc: &Document, profile: &LayoutProfile) -> PaginatedDocument {
    let max_lines = profile.lines_per_page().max(4);
    let mut blocks: Vec<WrappedBlock> = Vec::new();
    let mut prev_emitted_kind: Option<ElementType> = None;

    for (idx, el) in doc.elements.iter().enumerate() {
        // Non-printing outline elements (Section, Synopsis, Note) are skipped in script pagination.
        if matches!(
            el.kind,
            ElementType::Section(_) | ElementType::Synopsis | ElementType::Note
        ) {
            continue;
        }

        if el.kind == ElementType::PageBreak {
            blocks.push(WrappedBlock {
                element_index: idx,
                kind: ElementType::PageBreak,
                lines: Vec::new(),
                blank_before: 0,
                scene_number: None,
                dual_dialogue: None,
                has_revision: false,
            });
            prev_emitted_kind = Some(ElementType::PageBreak);
            continue;
        }

        let blank_before = if prev_emitted_kind.is_none()
            || prev_emitted_kind == Some(ElementType::PageBreak)
            || matches!(
                (prev_emitted_kind, el.kind),
                (
                    Some(ElementType::Character),
                    ElementType::Dialogue | ElementType::Parenthetical
                ) | (
                    Some(ElementType::Parenthetical),
                    ElementType::Dialogue | ElementType::Parenthetical
                ) | (
                    Some(ElementType::Dialogue),
                    ElementType::Parenthetical | ElementType::Dialogue
                )
            ) {
            0
        } else {
            1
        };

        let max_chars = if let Some(dual) = el.dual_dialogue {
            profile.dual_max_chars_for(dual.side, el.kind)
        } else {
            profile.max_chars_for(el.kind)
        };
        let mut runs = el.runs.clone();
        if el.kind.is_uppercase_convention() {
            for r in &mut runs {
                r.text = to_upper_for_lang(&r.text, doc.settings.script_language);
            }
        }
        let wrapped = wrap_runs(&runs, max_chars);
        let scene_number =
            if el.kind == ElementType::SceneHeading && doc.settings.show_scene_numbers {
                el.scene_number.as_ref().map(|s| s.value.clone())
            } else {
                None
            };

        blocks.push(WrappedBlock {
            element_index: idx,
            kind: el.kind,
            lines: wrapped,
            blank_before,
            scene_number,
            dual_dialogue: el.dual_dialogue,
            has_revision: el.revision.is_some(),
        });
        prev_emitted_kind = Some(el.kind);
    }

    let mut pages: Vec<PaginatedPage> = Vec::new();
    let mut current_lines: Vec<PaginatedLine> = Vec::new();
    let mut cursor_line: usize = 0;

    let flush_page = |pages: &mut Vec<PaginatedPage>,
                      current_lines: &mut Vec<PaginatedLine>,
                      cursor_line: &mut usize| {
        let page_num = pages.len() + 1;
        pages.push(PaginatedPage {
            page_number: page_num,
            show_page_number: page_num > 1,
            lines: std::mem::take(current_lines),
        });
        *cursor_line = 0;
    };

    let mut i = 0;
    let mut last_character_block: Option<&WrappedBlock> = None;

    while i < blocks.len() {
        let block = &blocks[i];

        if block.kind == ElementType::PageBreak {
            if !current_lines.is_empty() {
                flush_page(&mut pages, &mut current_lines, &mut cursor_line);
            }
            last_character_block = None;
            i += 1;
            continue;
        }

        // Dual dialogue group: emit Left and Right columns side by side at matching Y coordinates.
        if let Some(first_dual) = block.dual_dialogue {
            let mut group_end = i + 1;
            while group_end < blocks.len()
                && blocks[group_end]
                    .dual_dialogue
                    .is_some_and(|d| d.group_id == first_dual.group_id)
            {
                group_end += 1;
            }

            let mut left_rows: Vec<(usize, ElementType, Vec<TextRun>, bool)> = Vec::new();
            let mut right_rows: Vec<(usize, ElementType, Vec<TextRun>, bool)> = Vec::new();
            for gb in &blocks[i..group_end] {
                if let Some(d) = gb.dual_dialogue {
                    let target = match d.side {
                        DualSide::Left => &mut left_rows,
                        DualSide::Right => &mut right_rows,
                    };
                    for l in &gb.lines {
                        target.push((gb.element_index, gb.kind, l.clone(), gb.has_revision));
                    }
                }
            }

            if !left_rows.is_empty() && !right_rows.is_empty() {
                let height = left_rows.len().max(right_rows.len());
                let pre_blank = if cursor_line == 0 {
                    0
                } else {
                    block.blank_before
                };
                if cursor_line > 0 && cursor_line + pre_blank + height > max_lines {
                    flush_page(&mut pages, &mut current_lines, &mut cursor_line);
                }
                let effective_blank = if cursor_line == 0 {
                    0
                } else {
                    block.blank_before
                };
                cursor_line += effective_blank;

                for row in 0..height {
                    if cursor_line >= max_lines {
                        flush_page(&mut pages, &mut current_lines, &mut cursor_line);
                    }
                    let y_pt = profile.margin_top_in * PT_PER_INCH
                        + (cursor_line as f32 + 1.0) * profile.line_height_pt;
                    if let Some((el_idx, kind, runs, has_rev)) = left_rows.get(row) {
                        let (x_pt, right_x_pt, align) =
                            profile.dual_column_bounds_pt(DualSide::Left, *kind);
                        current_lines.push(PaginatedLine {
                            element_index: Some(*el_idx),
                            kind: *kind,
                            runs: runs.clone(),
                            x_pt,
                            right_x_pt,
                            y_pt,
                            align,
                            scene_number: None,
                            has_revision: *has_rev,
                        });
                    }
                    if let Some((el_idx, kind, runs, has_rev)) = right_rows.get(row) {
                        let (x_pt, right_x_pt, align) =
                            profile.dual_column_bounds_pt(DualSide::Right, *kind);
                        current_lines.push(PaginatedLine {
                            element_index: Some(*el_idx),
                            kind: *kind,
                            runs: runs.clone(),
                            x_pt,
                            right_x_pt,
                            y_pt,
                            align,
                            scene_number: None,
                            has_revision: *has_rev,
                        });
                    }
                    cursor_line += 1;
                }

                last_character_block = None;
                i = group_end;
                continue;
            }
        }

        let effective_blank = if cursor_line == 0 {
            0
        } else {
            block.blank_before
        };

        // Rule: SceneHeading never stranded alone at bottom of page (keep with next element's first line).
        if block.kind == ElementType::SceneHeading {
            let next_needed = blocks
                .get(i + 1)
                .filter(|b| b.kind != ElementType::PageBreak)
                .map_or(0, |b| b.blank_before + 1);
            let needed = effective_blank + block.lines.len() + next_needed;
            if cursor_line > 0 && cursor_line + needed > max_lines {
                flush_page(&mut pages, &mut current_lines, &mut cursor_line);
            }
        }

        // Rule: Character never separated from its first dialogue/parenthetical line.
        if block.kind == ElementType::Character {
            last_character_block = Some(block);
            let next_needed = blocks
                .get(i + 1)
                .filter(|b| matches!(b.kind, ElementType::Dialogue | ElementType::Parenthetical))
                .map_or(0, |_| 1);
            let needed = effective_blank + block.lines.len() + next_needed;
            if cursor_line > 0 && cursor_line + needed > max_lines {
                flush_page(&mut pages, &mut current_lines, &mut cursor_line);
            }
        } else if !matches!(
            block.kind,
            ElementType::Dialogue | ElementType::Parenthetical
        ) {
            last_character_block = None;
        }

        // Rule: Widow/orphan control for multi-line Action blocks.
        if block.kind == ElementType::Action && block.lines.len() >= 2 && cursor_line > 0 {
            let remaining = max_lines.saturating_sub(cursor_line + effective_blank);
            if remaining < 2 && block.lines.len() > remaining {
                flush_page(&mut pages, &mut current_lines, &mut cursor_line);
            }
        }

        let effective_blank = if cursor_line == 0 {
            0
        } else {
            block.blank_before
        };
        cursor_line += effective_blank;

        // Dialogue / Parenthetical splitting with (MORE)/(DEVAM) and (CONT'D)/(DEVAM)
        if matches!(
            block.kind,
            ElementType::Dialogue | ElementType::Parenthetical
        ) {
            let mut line_idx = 0;
            while line_idx < block.lines.len() {
                let remaining = max_lines.saturating_sub(cursor_line);
                let lines_left = block.lines.len() - line_idx;

                if lines_left <= remaining {
                    // Fits completely on current page
                    for l in &block.lines[line_idx..] {
                        push_line(
                            &mut current_lines,
                            profile,
                            &mut cursor_line,
                            Some(block.element_index),
                            block.kind,
                            l.clone(),
                            None,
                            block.has_revision,
                        );
                    }
                    break;
                }

                // Need to split across page: reserve 1 line for (MORE)/(DEVAM)
                if remaining >= 2 {
                    let take = remaining - 1;
                    for l in &block.lines[line_idx..line_idx + take] {
                        push_line(
                            &mut current_lines,
                            profile,
                            &mut cursor_line,
                            Some(block.element_index),
                            block.kind,
                            l.clone(),
                            None,
                            block.has_revision,
                        );
                    }
                    line_idx += take;

                    // Emit (MORE) / (DEVAM) at Character column
                    push_line(
                        &mut current_lines,
                        profile,
                        &mut cursor_line,
                        None,
                        ElementType::Character,
                        vec![TextRun::plain(
                            doc.settings.dictionary.dialogue_more.clone(),
                        )],
                        None,
                        false,
                    );
                }

                flush_page(&mut pages, &mut current_lines, &mut cursor_line);

                // Emit Character (CONT'D) / (DEVAM) at top of new page
                if let Some(char_block) = last_character_block {
                    let base_name = char_block
                        .lines
                        .first()
                        .map(|runs| runs.iter().map(|r| r.text.as_str()).collect::<String>())
                        .unwrap_or_default();
                    let contd_text = format!(
                        "{} {}",
                        base_name.trim_end(),
                        doc.settings.dictionary.dialogue_contd
                    );
                    push_line(
                        &mut current_lines,
                        profile,
                        &mut cursor_line,
                        Some(char_block.element_index),
                        ElementType::Character,
                        vec![TextRun::plain(contd_text)],
                        None,
                        false,
                    );
                }
            }
        } else {
            for (l_i, line_runs) in block.lines.iter().enumerate() {
                if cursor_line >= max_lines {
                    flush_page(&mut pages, &mut current_lines, &mut cursor_line);
                }
                let sn = if l_i == 0 {
                    block.scene_number.clone()
                } else {
                    None
                };
                push_line(
                    &mut current_lines,
                    profile,
                    &mut cursor_line,
                    Some(block.element_index),
                    block.kind,
                    line_runs.clone(),
                    sn,
                    block.has_revision,
                );
            }
        }

        i += 1;
    }

    if !current_lines.is_empty() || pages.is_empty() {
        flush_page(&mut pages, &mut current_lines, &mut cursor_line);
    }

    PaginatedDocument { pages }
}

#[allow(clippy::too_many_arguments)]
fn push_line(
    current_lines: &mut Vec<PaginatedLine>,
    profile: &LayoutProfile,
    cursor_line: &mut usize,
    element_index: Option<usize>,
    kind: ElementType,
    runs: Vec<TextRun>,
    scene_number: Option<String>,
    has_revision: bool,
) {
    let (x_pt, right_x_pt, align) = profile.column_bounds_pt(kind);
    let y_pt =
        profile.margin_top_in * PT_PER_INCH + (*cursor_line as f32 + 1.0) * profile.line_height_pt;
    current_lines.push(PaginatedLine {
        element_index,
        kind,
        runs,
        x_pt,
        right_x_pt,
        y_pt,
        align,
        scene_number,
        has_revision,
    });
    *cursor_line += 1;
}

/// Word-wraps styled [`TextRun`]s to fit within `max_chars` characters per line.
fn wrap_runs(runs: &[TextRun], max_chars: usize) -> Vec<Vec<TextRun>> {
    if runs.is_empty() || runs.iter().all(|r| r.text.is_empty()) {
        return vec![Vec::new()];
    }

    // Flatten into styled words + spaces while preserving inline styles.
    #[derive(Clone)]
    struct StyledToken {
        text: String,
        style: InlineStyle,
        is_space: bool,
    }

    let mut tokens: Vec<StyledToken> = Vec::new();
    for run in runs {
        let mut cur = String::new();
        let mut cur_space: Option<bool> = None;
        for ch in run.text.chars() {
            let is_sp = ch.is_whitespace();
            if cur_space == Some(is_sp) || cur_space.is_none() {
                cur.push(ch);
                cur_space = Some(is_sp);
            } else {
                tokens.push(StyledToken {
                    text: std::mem::take(&mut cur),
                    style: run.style,
                    is_space: cur_space.unwrap_or(false),
                });
                cur.push(ch);
                cur_space = Some(is_sp);
            }
        }
        if !cur.is_empty() {
            tokens.push(StyledToken {
                text: cur,
                style: run.style,
                is_space: cur_space.unwrap_or(false),
            });
        }
    }

    let mut lines: Vec<Vec<TextRun>> = Vec::new();
    let mut current_line: Vec<TextRun> = Vec::new();
    let mut line_chars = 0usize;

    let push_styled = |line: &mut Vec<TextRun>, text: String, style: InlineStyle| {
        if let Some(last) = line.last_mut() {
            if last.style == style {
                last.text.push_str(&text);
                return;
            }
        }
        line.push(TextRun { text, style });
    };

    for tok in tokens {
        let tok_len = tok.text.chars().count();
        if tok.is_space {
            if line_chars > 0 && line_chars + tok_len <= max_chars {
                push_styled(&mut current_line, tok.text, tok.style);
                line_chars += tok_len;
            }
            continue;
        }

        if line_chars > 0 && line_chars + tok_len > max_chars {
            lines.push(std::mem::take(&mut current_line));
            line_chars = 0;
        }

        if tok_len <= max_chars {
            push_styled(&mut current_line, tok.text, tok.style);
            line_chars += tok_len;
        } else {
            // Hard-break single word longer than max_chars
            let mut chunk = String::new();
            for ch in tok.text.chars() {
                if line_chars >= max_chars {
                    if !chunk.is_empty() {
                        push_styled(&mut current_line, std::mem::take(&mut chunk), tok.style);
                    }
                    lines.push(std::mem::take(&mut current_line));
                    line_chars = 0;
                }
                chunk.push(ch);
                line_chars += 1;
            }
            if !chunk.is_empty() {
                push_styled(&mut current_line, chunk, tok.style);
            }
        }
    }

    if !current_line.is_empty() || lines.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Element, ScriptLanguage};

    #[test]
    fn scene_heading_is_not_stranded_at_bottom_of_page() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();

        let profile = LayoutProfile::for_page_size(PageSize::UsLetter);
        let max_lines = profile.lines_per_page(); // 54

        // Fill page 1 up to max_lines - 1
        for i in 0..((max_lines - 1) / 2) {
            doc.elements.push(Element::new(
                ElementType::Action,
                format!("Kısa aksiyon satırı {i}."),
            ));
        }
        doc.elements
            .push(Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"));
        doc.elements
            .push(Element::new(ElementType::Action, "Yeni sahne başlar."));

        let paged = paginate(&doc, &profile);
        assert!(paged.page_count() >= 2);
        // SceneHeading must be at the top of page 2, not stranded at bottom of page 1
        let first_on_p2 = &paged.pages[1].lines[0];
        assert_eq!(first_on_p2.kind, ElementType::SceneHeading);
    }

    #[test]
    fn dialogue_split_adds_more_and_contd_in_script_language() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();

        let profile = LayoutProfile::for_page_size(PageSize::UsLetter);
        // Fill most of page 1
        for i in 0..24 {
            doc.elements.push(Element::new(
                ElementType::Action,
                format!("Aksiyon satırı {i}."),
            ));
        }
        doc.elements
            .push(Element::new(ElementType::Character, "ÇİĞDEM"));
        let long_dialogue = "İstanbul sokaklarında yürürken yağmurun altında uzun uzun düşündüm ve sonunda kararımı verdim. ".repeat(6);
        doc.elements
            .push(Element::new(ElementType::Dialogue, long_dialogue));

        let paged = paginate(&doc, &profile);
        assert!(paged.page_count() >= 2);
        assert!(!paged.pages[0].show_page_number);
        assert!(paged.pages[1].show_page_number);

        let p1_last = paged.pages[0].lines.last().unwrap();
        let p1_last_text: String = p1_last.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(p1_last_text, "(DEVAM)");

        let p2_first = &paged.pages[1].lines[0];
        let p2_first_text: String = p2_first.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(p2_first_text, "ÇİĞDEM (DEVAM)");
    }

    #[test]
    fn dual_dialogue_paginates_side_by_side_in_two_columns() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();

        let group_id = uuid::Uuid::new_v4();
        let left_group = Some(DualDialogueGroup {
            group_id,
            side: DualSide::Left,
        });
        let right_group = Some(DualDialogueGroup {
            group_id,
            side: DualSide::Right,
        });

        let mut c1 = Element::new(ElementType::Character, "AYŞE");
        c1.dual_dialogue = left_group;
        let mut d1 = Element::new(ElementType::Dialogue, "Kapıyı kilitle.");
        d1.dual_dialogue = left_group;

        let mut c2 = Element::new(ElementType::Character, "MEHMET");
        c2.dual_dialogue = right_group;
        let mut d2 = Element::new(ElementType::Dialogue, "Hemen kilitliyorum.");
        d2.dual_dialogue = right_group;

        doc.elements.extend([c1, d1, c2, d2]);

        let profile = LayoutProfile::for_page_size(PageSize::A4);
        let paged = paginate(&doc, &profile);
        assert_eq!(paged.page_count(), 1);

        let lines = &paged.pages[0].lines;
        let ayse = lines
            .iter()
            .find(|l| l.runs.iter().any(|r| r.text.contains("AYŞE")))
            .expect("AYŞE line");
        let mehmet = lines
            .iter()
            .find(|l| l.runs.iter().any(|r| r.text.contains("MEHMET")))
            .expect("MEHMET line");

        assert!((ayse.y_pt - mehmet.y_pt).abs() < 0.01);
        assert!(ayse.right_x_pt < mehmet.x_pt);
    }

    #[test]
    fn dual_dialogue_near_page_bottom_moves_cleanly_to_next_page() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        for i in 0..48 {
            doc.elements.push(Element::new(
                ElementType::Action,
                format!("Dolgu satırı {i}."),
            ));
        }

        let group_id = uuid::Uuid::new_v4();
        let left_group = Some(DualDialogueGroup {
            group_id,
            side: DualSide::Left,
        });
        let right_group = Some(DualDialogueGroup {
            group_id,
            side: DualSide::Right,
        });

        let mut c1 = Element::new(ElementType::Character, "LEYLA");
        c1.dual_dialogue = left_group;
        let mut d1 = Element::new(
            ElementType::Dialogue,
            "Bu ikili diyalog sayfa sonuna sığmayacak kadar uzun bir repliktir.",
        );
        d1.dual_dialogue = left_group;

        let mut c2 = Element::new(ElementType::Character, "MECNUN");
        c2.dual_dialogue = right_group;
        let mut d2 = Element::new(
            ElementType::Dialogue,
            "Evet, ikisi birlikte yeni sayfaya geçmelidir.",
        );
        d2.dual_dialogue = right_group;

        doc.elements.extend([c1, d1, c2, d2]);

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert!(paged.page_count() >= 2);
        let p2_lines = &paged.pages[1].lines;
        let leyla = p2_lines
            .iter()
            .find(|l| l.runs.iter().any(|r| r.text.contains("LEYLA")))
            .expect("LEYLA on page 2");
        let mecnun = p2_lines
            .iter()
            .find(|l| l.runs.iter().any(|r| r.text.contains("MECNUN")))
            .expect("MECNUN on page 2");
        assert!((leyla.y_pt - mecnun.y_pt).abs() < 0.01);
    }

    #[test]
    fn multi_page_dialogue_repeats_continuation_markers_across_pages() {
        let mut doc = Document::new(ScriptLanguage::En);
        doc.elements.clear();
        doc.elements
            .push(Element::new(ElementType::Character, "HAMLET"));
        let speech =
            "To be, or not to be, that is the question whether tis nobler in the mind to suffer. "
                .repeat(85);
        doc.elements
            .push(Element::new(ElementType::Dialogue, speech));

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert!(paged.page_count() >= 3);

        for page in &paged.pages[..paged.page_count() - 1] {
            let last_text: String = page
                .lines
                .last()
                .unwrap()
                .runs
                .iter()
                .map(|r| r.text.as_str())
                .collect();
            assert_eq!(last_text, "(MORE)");
        }
        for page in &paged.pages[1..] {
            let first_text: String = page.lines[0].runs.iter().map(|r| r.text.as_str()).collect();
            assert_eq!(first_text, "HAMLET (CONT'D)");
        }
    }

    #[test]
    fn long_character_cue_does_not_drop_text() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        let long_cue = "BİRİNCİ İSTASYON KOMUTAN YARDIMCISI VE NÖBETÇİ SUBAY (TELSİZDEN)";
        doc.elements
            .push(Element::new(ElementType::Character, long_cue));
        doc.elements
            .push(Element::new(ElementType::Dialogue, "Anlaşıldı, merkez."));

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert_eq!(paged.page_count(), 1);
        let joined: String = paged.pages[0]
            .lines
            .iter()
            .flat_map(|l| l.runs.iter().map(|r| r.text.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("KOMUTAN"));
        assert!(joined.contains("Anlaşıldı"));
    }

    #[test]
    fn explicit_page_break_forces_new_page() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        doc.elements
            .push(Element::new(ElementType::Action, "İlk sayfa metni."));
        doc.elements.push(Element::new(ElementType::PageBreak, ""));
        doc.elements
            .push(Element::new(ElementType::Action, "İkinci sayfa metni."));

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert_eq!(paged.page_count(), 2);
        assert!(paged.pages[0]
            .lines
            .iter()
            .any(|l| l.runs.iter().any(|r| r.text.contains("İlk sayfa"))));
        assert!(paged.pages[1]
            .lines
            .iter()
            .any(|l| l.runs.iter().any(|r| r.text.contains("İkinci sayfa"))));
    }

    #[test]
    fn long_action_paragraph_splits_across_pages_without_data_loss() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        let long_action = (0..140)
            .map(|i| format!("Aksiyon cümlesi numara {i} yağmur altında devam ediyor."))
            .collect::<Vec<_>>()
            .join(" ");
        doc.elements
            .push(Element::new(ElementType::Action, long_action));

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert!(paged.page_count() >= 2);
        let all_text: String = paged
            .pages
            .iter()
            .flat_map(|p| p.lines.iter())
            .flat_map(|l| l.runs.iter().map(|r| r.text.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all_text.contains("numara 0"));
        assert!(all_text.contains("numara 139"));
    }

    #[test]
    fn scene_heading_before_dialogue_keeps_heading_off_bottom_orphan() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        for i in 0..50 {
            doc.elements
                .push(Element::new(ElementType::Action, format!("Satır {i}.")));
        }
        doc.elements.push(Element::new(
            ElementType::SceneHeading,
            "DIŞ. İSKELE - GECE",
        ));
        doc.elements
            .push(Element::new(ElementType::Character, "DENİZ"));
        doc.elements
            .push(Element::new(ElementType::Dialogue, "Vapur yanaşıyor."));

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert!(paged.page_count() >= 2);
        let p1_last_text: String = paged.pages[0]
            .lines
            .last()
            .unwrap()
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        assert!(!p1_last_text.contains("DIŞ. İSKELE"));
    }

    #[test]
    fn revision_marks_are_preserved_across_dialogue_continuation() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements.clear();
        for i in 0..24 {
            doc.elements
                .push(Element::new(ElementType::Action, format!("Önceki {i}.")));
        }
        let mut ch = Element::new(ElementType::Character, "ZEYNEP");
        ch.revision = Some(crate::RevisionMark {
            color: "blue".to_string(),
            label: String::new(),
        });
        let mut dlg = Element::new(
            ElementType::Dialogue,
            "Bu revizyonlu replik sayfa sonuna denk geldiği için iki sayfaya bölünmelidir. "
                .repeat(6),
        );
        dlg.revision = Some(crate::RevisionMark {
            color: "blue".to_string(),
            label: String::new(),
        });
        doc.elements.extend([ch, dlg]);

        let paged = paginate(&doc, &LayoutProfile::for_page_size(PageSize::UsLetter));
        assert!(paged.page_count() >= 2);
        assert!(paged.pages[0].lines.iter().any(|l| l.has_revision));
        assert!(paged.pages[1].lines.iter().any(|l| l.has_revision));
        let p2_first: String = paged.pages[1].lines[0]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(p2_first, "ZEYNEP (DEVAM)");
    }
}
