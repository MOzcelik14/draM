//! Screenplay breakdown reports (character statistics, locations, INT/EXT, DAY/NIGHT, CSV export).

use std::collections::{HashMap, HashSet};

use crate::model::{Document, ElementType, ScriptLanguage};
use crate::text::{cmp_tr, to_upper_for_lang};

/// Aggregated dialogue and scene statistics for a single character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterReport {
    /// Character name (uppercase normalized).
    pub name: String,
    /// Number of dialogue blocks spoken by this character.
    pub dialogue_blocks: usize,
    /// Total words spoken across all dialogue blocks.
    pub word_count: usize,
    /// Number of distinct scenes in which this character appears.
    pub scene_count: usize,
}

/// Aggregated scene statistics for a single location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationReport {
    /// Normalized location name (stripped of INT./EXT. prefix and DAY/NIGHT suffix).
    pub location: String,
    /// Total number of scenes at this location.
    pub scene_count: usize,
    /// Scenes marked interior (`İÇ.` / `INT.`).
    pub interior_count: usize,
    /// Scenes marked exterior (`DIŞ.` / `EXT.`).
    pub exterior_count: usize,
    /// Scenes marked day (`GÜN` / `GÜNDÜZ` / `DAY`).
    pub day_count: usize,
    /// Scenes marked night (`GECE` / `NIGHT`).
    pub night_count: usize,
}

/// Complete breakdown report for a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBreakdown {
    /// Total number of `SceneHeading` elements.
    pub total_scenes: usize,
    /// Number of interior scenes.
    pub interior_scenes: usize,
    /// Number of exterior scenes.
    pub exterior_scenes: usize,
    /// Number of day scenes.
    pub day_scenes: usize,
    /// Number of night scenes.
    pub night_scenes: usize,
    /// Character statistics sorted by dialogue blocks descending, then Turkish collation.
    pub characters: Vec<CharacterReport>,
    /// Location statistics sorted by scene count descending, then Turkish collation.
    pub locations: Vec<LocationReport>,
}

/// Generates a [`ScriptBreakdown`] from `doc`.
#[must_use]
pub fn generate_breakdown(doc: &Document) -> ScriptBreakdown {
    let lang = doc.settings.script_language;
    let mut total_scenes = 0usize;
    let mut interior_scenes = 0usize;
    let mut exterior_scenes = 0usize;
    let mut day_scenes = 0usize;
    let mut night_scenes = 0usize;

    // Character accumulators: name -> (dialogue_blocks, word_count, scene_ids)
    let mut char_map: HashMap<String, (usize, usize, HashSet<usize>)> = HashMap::new();
    // Location accumulators: location -> (scenes, int, ext, day, night)
    let mut loc_map: HashMap<String, (usize, usize, usize, usize, usize)> = HashMap::new();

    let mut current_scene_id: usize = 0;
    let mut active_character: Option<String> = None;

    for el in &doc.elements {
        match el.kind {
            ElementType::SceneHeading => {
                total_scenes += 1;
                current_scene_id = total_scenes;
                active_character = None;

                let raw = el.plain_text();
                let parsed = parse_scene_heading(&raw, lang);
                if parsed.is_interior {
                    interior_scenes += 1;
                }
                if parsed.is_exterior {
                    exterior_scenes += 1;
                }
                if parsed.is_day {
                    day_scenes += 1;
                }
                if parsed.is_night {
                    night_scenes += 1;
                }

                if !parsed.location.is_empty() {
                    let entry = loc_map.entry(parsed.location).or_insert((0, 0, 0, 0, 0));
                    entry.0 += 1;
                    if parsed.is_interior {
                        entry.1 += 1;
                    }
                    if parsed.is_exterior {
                        entry.2 += 1;
                    }
                    if parsed.is_day {
                        entry.3 += 1;
                    }
                    if parsed.is_night {
                        entry.4 += 1;
                    }
                }
            }
            ElementType::Character => {
                let raw = el.plain_text();
                let base = strip_character_extension(&raw);
                let normalized = to_upper_for_lang(base.trim(), lang);
                if !normalized.is_empty() {
                    let entry = char_map
                        .entry(normalized.clone())
                        .or_insert_with(|| (0, 0, HashSet::new()));
                    entry.0 += 1;
                    if current_scene_id > 0 {
                        entry.2.insert(current_scene_id);
                    }
                    active_character = Some(normalized);
                } else {
                    active_character = None;
                }
            }
            ElementType::Dialogue => {
                if let Some(ref name) = active_character {
                    let words = el.plain_text().split_whitespace().count();
                    if let Some(entry) = char_map.get_mut(name) {
                        entry.1 += words;
                    }
                }
            }
            ElementType::Parenthetical => {
                // Keep active_character alive across parentheticals.
            }
            _ => {
                active_character = None;
            }
        }
    }

    let mut characters: Vec<CharacterReport> = char_map
        .into_iter()
        .map(
            |(name, (dialogue_blocks, word_count, scenes))| CharacterReport {
                name,
                dialogue_blocks,
                word_count,
                scene_count: scenes.len(),
            },
        )
        .collect();
    characters.sort_by(|a, b| {
        b.dialogue_blocks
            .cmp(&a.dialogue_blocks)
            .then_with(|| b.word_count.cmp(&a.word_count))
            .then_with(|| cmp_tr(&a.name, &b.name))
    });

    let mut locations: Vec<LocationReport> = loc_map
        .into_iter()
        .map(
            |(location, (scene_count, interior_count, exterior_count, day_count, night_count))| {
                LocationReport {
                    location,
                    scene_count,
                    interior_count,
                    exterior_count,
                    day_count,
                    night_count,
                }
            },
        )
        .collect();
    locations.sort_by(|a, b| {
        b.scene_count
            .cmp(&a.scene_count)
            .then_with(|| cmp_tr(&a.location, &b.location))
    });

    ScriptBreakdown {
        total_scenes,
        interior_scenes,
        exterior_scenes,
        day_scenes,
        night_scenes,
        characters,
        locations,
    }
}

/// Serializes `breakdown` into a UTF-8 CSV document containing character and location tables.
#[must_use]
pub fn breakdown_to_csv(breakdown: &ScriptBreakdown) -> String {
    let mut out = String::new();
    out.push_str("Section,Name,Scenes,DialogueBlocks,Words,Interior,Exterior,Day,Night\n");

    for ch in &breakdown.characters {
        out.push_str(&format!(
            "Character,{},{},{},{},,,,\n",
            csv_escape(&ch.name),
            ch.scene_count,
            ch.dialogue_blocks,
            ch.word_count
        ));
    }

    for loc in &breakdown.locations {
        out.push_str(&format!(
            "Location,{},{},,,{},{},{},{}\n",
            csv_escape(&loc.location),
            loc.scene_count,
            loc.interior_count,
            loc.exterior_count,
            loc.day_count,
            loc.night_count
        ));
    }

    out
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn strip_character_extension(raw: &str) -> &str {
    match raw.find('(') {
        Some(pos) => &raw[..pos],
        None => raw,
    }
}

struct ParsedHeading {
    location: String,
    is_interior: bool,
    is_exterior: bool,
    is_day: bool,
    is_night: bool,
}

fn parse_scene_heading(raw: &str, lang: ScriptLanguage) -> ParsedHeading {
    let upper = to_upper_for_lang(raw.trim(), lang);
    let mut rest = upper.as_str();

    let (is_interior, is_exterior) = if let Some(s) = rest
        .strip_prefix("İÇ./DIŞ.")
        .or_else(|| rest.strip_prefix("İÇ/DIŞ."))
        .or_else(|| rest.strip_prefix("INT./EXT."))
        .or_else(|| rest.strip_prefix("INT/EXT."))
        .or_else(|| rest.strip_prefix("I/E."))
    {
        rest = s.trim();
        (true, true)
    } else if let Some(s) = rest
        .strip_prefix("İÇ.")
        .or_else(|| rest.strip_prefix("İÇ "))
        .or_else(|| rest.strip_prefix("INT."))
        .or_else(|| rest.strip_prefix("INT "))
    {
        rest = s.trim();
        (true, false)
    } else if let Some(s) = rest
        .strip_prefix("DIŞ.")
        .or_else(|| rest.strip_prefix("DIŞ "))
        .or_else(|| rest.strip_prefix("EXT."))
        .or_else(|| rest.strip_prefix("EXT "))
    {
        rest = s.trim();
        (false, true)
    } else {
        (false, false)
    };

    let (loc_part, time_part) = match rest.rsplit_once(" - ") {
        Some((l, t)) => (l.trim(), t.trim()),
        None => (rest.trim(), ""),
    };

    let is_night = time_part.contains("GECE")
        || time_part.contains("NIGHT")
        || time_part.contains("AKŞAM")
        || time_part.contains("EVENING");
    let is_day = !is_night
        && (time_part.contains("GÜN")
            || time_part.contains("DAY")
            || time_part.contains("SABAH")
            || time_part.contains("MORNING")
            || time_part.contains("ÖĞLE"));

    ParsedHeading {
        location: loc_part.to_string(),
        is_interior,
        is_exterior,
        is_day,
        is_night,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Element;

    #[test]
    fn breakdown_counts_turkish_scenes_characters_and_exports_csv() {
        let mut doc = Document::new(ScriptLanguage::Tr);
        doc.elements = vec![
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GÜN"),
            Element::new(ElementType::Character, "AYŞE (D.S.)"),
            Element::new(ElementType::Dialogue, "Kahve soğudu bile."),
            Element::new(ElementType::Character, "MEHMET"),
            Element::new(ElementType::Dialogue, "Yenisini söylerim."),
            Element::new(ElementType::SceneHeading, "DIŞ. İSTASYON - GECE"),
            Element::new(ElementType::Character, "AYŞE"),
            Element::new(ElementType::Dialogue, "Son tren kalktı."),
            Element::new(ElementType::SceneHeading, "İÇ. KAFE - GECE"),
        ];

        let report = generate_breakdown(&doc);
        assert_eq!(report.total_scenes, 3);
        assert_eq!(report.interior_scenes, 2);
        assert_eq!(report.exterior_scenes, 1);
        assert_eq!(report.day_scenes, 1);
        assert_eq!(report.night_scenes, 2);

        assert_eq!(report.characters.len(), 2);
        assert_eq!(report.characters[0].name, "AYŞE");
        assert_eq!(report.characters[0].dialogue_blocks, 2);
        assert_eq!(report.characters[0].word_count, 6);
        assert_eq!(report.characters[0].scene_count, 2);

        assert_eq!(report.locations.len(), 2);
        assert_eq!(report.locations[0].location, "KAFE");
        assert_eq!(report.locations[0].scene_count, 2);
        assert_eq!(report.locations[0].interior_count, 2);
        assert_eq!(report.locations[0].day_count, 1);
        assert_eq!(report.locations[0].night_count, 1);

        let csv = breakdown_to_csv(&report);
        assert!(csv.contains("Character,AYŞE,2,2,6"));
        assert!(csv.contains("Location,KAFE,2,,,2,0,1,1"));
    }
}
