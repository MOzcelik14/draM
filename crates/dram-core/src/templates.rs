//! Starter screenplay project templates for Turkish and English scripts.

use crate::model::{Document, Element, ElementType, ScriptLanguage};

/// Built-in screenplay project templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    /// Three-act feature film screenplay.
    FeatureFilm,
    /// Concise short film screenplay.
    ShortFilm,
    /// Episodic television script with teaser and act breaks.
    TvEpisode,
}

/// Builds a new [`Document`] pre-populated with structure for `template` and `lang`.
#[must_use]
pub fn document_from_template(template: ProjectTemplate, lang: ScriptLanguage) -> Document {
    let mut doc = Document::new(lang);
    doc.elements.clear();

    match (template, lang) {
        (ProjectTemplate::FeatureFilm, ScriptLanguage::Tr) => {
            doc.title_page.title = "ADSIZ UZUN METRAJ".to_string();
            doc.title_page.credit = "Yazan".to_string();
            doc.elements = vec![
                Element::new(ElementType::Section(1), "1. PERDE"),
                Element::new(ElementType::SceneHeading, "İÇ. AÇILIŞ MEKANI - GÜN"),
                Element::new(ElementType::Action, "Açılış sahnesi burada başlar."),
                Element::new(ElementType::Section(1), "2. PERDE"),
                Element::new(ElementType::SceneHeading, "DIŞ. İKİNCİ PERDE - GÜN"),
                Element::new(ElementType::Action, "Çatışma derinleşir."),
                Element::new(ElementType::Section(1), "3. PERDE"),
                Element::new(ElementType::SceneHeading, "İÇ. FİNAL MEKANI - GECE"),
                Element::new(ElementType::Action, "Doruk noktası ve çözülme."),
                Element::new(ElementType::Transition, "KARARMA."),
            ];
        }
        (ProjectTemplate::FeatureFilm, ScriptLanguage::En) => {
            doc.title_page.title = "UNTITLED FEATURE".to_string();
            doc.title_page.credit = "Written by".to_string();
            doc.elements = vec![
                Element::new(ElementType::Section(1), "ACT ONE"),
                Element::new(ElementType::SceneHeading, "INT. OPENING LOCATION - DAY"),
                Element::new(ElementType::Action, "The opening image unfolds here."),
                Element::new(ElementType::Section(1), "ACT TWO"),
                Element::new(ElementType::SceneHeading, "EXT. SECOND ACT - DAY"),
                Element::new(ElementType::Action, "The central conflict escalates."),
                Element::new(ElementType::Section(1), "ACT THREE"),
                Element::new(ElementType::SceneHeading, "INT. FINALE LOCATION - NIGHT"),
                Element::new(ElementType::Action, "Climax and resolution."),
                Element::new(ElementType::Transition, "FADE OUT."),
            ];
        }
        (ProjectTemplate::ShortFilm, ScriptLanguage::Tr) => {
            doc.title_page.title = "ADSIZ KISA FİLM".to_string();
            doc.title_page.credit = "Yazan".to_string();
            doc.elements = vec![
                Element::new(ElementType::SceneHeading, "İÇ. MEKAN - GÜN"),
                Element::new(
                    ElementType::Synopsis,
                    "Kısa filmin ana fikri ve görsel açılış özeti.",
                ),
                Element::new(
                    ElementType::Action,
                    "Hikaye doğrudan çatışmanın ortasından başlar.",
                ),
                Element::new(ElementType::Transition, "KARARMA."),
            ];
        }
        (ProjectTemplate::ShortFilm, ScriptLanguage::En) => {
            doc.title_page.title = "UNTITLED SHORT".to_string();
            doc.title_page.credit = "Written by".to_string();
            doc.elements = vec![
                Element::new(ElementType::SceneHeading, "INT. LOCATION - DAY"),
                Element::new(
                    ElementType::Synopsis,
                    "Brief logline and visual hook for the short film.",
                ),
                Element::new(ElementType::Action, "The story begins in medias res."),
                Element::new(ElementType::Transition, "FADE OUT."),
            ];
        }
        (ProjectTemplate::TvEpisode, ScriptLanguage::Tr) => {
            doc.title_page.title = "DİZİ ADI — 1. BÖLÜM".to_string();
            doc.title_page.credit = "Yazan".to_string();
            doc.elements = vec![
                Element::new(ElementType::Section(1), "ÖN AÇILIŞ (TEASER)"),
                Element::new(ElementType::SceneHeading, "DIŞ. ŞEHİR - GECE"),
                Element::new(ElementType::Action, "İzleyiciyi yakalayan açılış olayı."),
                Element::new(ElementType::Section(1), "1. PERDE"),
                Element::new(ElementType::SceneHeading, "İÇ. ANA MEKAN - GÜN"),
                Element::new(ElementType::Action, "Bölümün ana hikayesi kurulur."),
                Element::new(ElementType::Section(1), "2. PERDE"),
                Element::new(ElementType::SceneHeading, "İÇ. KRİZ MEKANI - GECE"),
                Element::new(ElementType::Action, "Bölüm sonu gerilimi ve kapanış."),
            ];
        }
        (ProjectTemplate::TvEpisode, ScriptLanguage::En) => {
            doc.title_page.title = "SERIES TITLE — EPISODE 101".to_string();
            doc.title_page.credit = "Written by".to_string();
            doc.elements = vec![
                Element::new(ElementType::Section(1), "TEASER"),
                Element::new(ElementType::SceneHeading, "EXT. CITY - NIGHT"),
                Element::new(ElementType::Action, "Cold open hook."),
                Element::new(ElementType::Section(1), "ACT ONE"),
                Element::new(ElementType::SceneHeading, "INT. MAIN LOCATION - DAY"),
                Element::new(ElementType::Action, "The episode A-story begins."),
                Element::new(ElementType::Section(1), "ACT TWO"),
                Element::new(ElementType::SceneHeading, "INT. CLIMAX LOCATION - NIGHT"),
                Element::new(ElementType::Action, "Act-out turn and cliffhanger."),
            ];
        }
    }

    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_produce_non_empty_localized_documents() {
        for tpl in [
            ProjectTemplate::FeatureFilm,
            ProjectTemplate::ShortFilm,
            ProjectTemplate::TvEpisode,
        ] {
            for lang in [ScriptLanguage::Tr, ScriptLanguage::En] {
                let doc = document_from_template(tpl, lang);
                assert!(!doc.title_page.title.is_empty());
                assert!(doc
                    .elements
                    .iter()
                    .any(|e| e.kind == ElementType::SceneHeading));
            }
        }
    }
}
