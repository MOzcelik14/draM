//! End-to-end integration tests: Fountain -> Document -> .dram -> Document -> PDF.

use std::io::Cursor;

use dram_core::{paginate, DualSide, LayoutProfile, PageSize, ScriptLanguage};
use dram_format::{parse_fountain, read_dram_from_reader, write_dram_to_writer};
use dram_pdf::export_pdf_bytes;

#[test]
fn fountain_to_dram_to_pdf_pipeline_preserves_turkish_and_dual_dialogue() {
    let fountain = "\
Title: GECE VARDİYASI
Author: Örnek Yazar

İÇ. İSTASYON - GECE

Yağmur camlara vurur. **Çetin** ve *Şule* peronda bekler.

AYŞE
Güvenlik kapısı kapalı mı?

MEHMET ^
Evet, bütün ışıklar söndü.

KESME:
";

    let doc = parse_fountain(fountain, ScriptLanguage::Tr);
    assert_eq!(doc.title_page.title, "GECE VARDİYASI");
    assert_eq!(doc.title_page.author, "Örnek Yazar");
    assert!(doc
        .elements
        .iter()
        .any(|el| el.dual_dialogue.is_some_and(|d| d.side == DualSide::Right)));

    let mut archive_buf = Cursor::new(Vec::new());
    write_dram_to_writer(&mut archive_buf, &doc).expect("write .dram archive");

    archive_buf.set_position(0);
    let reloaded = read_dram_from_reader(archive_buf).expect("read .dram archive");

    assert_eq!(reloaded.title_page.title, "GECE VARDİYASI");
    assert_eq!(reloaded.elements.len(), doc.elements.len());
    assert!(reloaded
        .elements
        .iter()
        .any(|el| el.plain_text().contains("Çetin") && el.plain_text().contains("Şule")));
    assert!(reloaded
        .elements
        .iter()
        .any(|el| el.dual_dialogue.is_some_and(|d| d.side == DualSide::Left)));
    assert!(reloaded
        .elements
        .iter()
        .any(|el| el.dual_dialogue.is_some_and(|d| d.side == DualSide::Right)));

    let profile = LayoutProfile::for_page_size(PageSize::A4);
    let paged = paginate(&reloaded, &profile);
    assert!(paged.page_count() >= 1);
    let ayse = paged
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .find(|line| line.runs.iter().any(|r| r.text.contains("AYŞE")))
        .expect("AYŞE line");
    let mehmet = paged
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .find(|line| line.runs.iter().any(|r| r.text.contains("MEHMET")))
        .expect("MEHMET line");
    assert!((ayse.y_pt - mehmet.y_pt).abs() < 0.01);
    assert!(ayse.right_x_pt < mehmet.x_pt);

    let pdf_bytes = export_pdf_bytes(&reloaded, &profile).expect("export PDF bytes");
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.len() > 5000);
}

fn count_pdf_pages(pdf_bytes: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf_bytes);
    // Krilla emits `/Type /Page` or `/Type/Page` (and `/Type/Pages` for the root tree).
    let mut count = 0usize;
    for needle in ["/Type/Page", "/Type /Page"] {
        for (idx, _) in text.match_indices(needle) {
            let after = text.as_bytes().get(idx + needle.len()).copied();
            if !matches!(after, Some(b's')) {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn pdf_golden_structure_verifies_page_count_mediabox_and_coordinates() {
    let fountain = "\
Title: ALTIN REFERANS
Author: Test Senarist

İÇ. LABORATUVAR - GÜN

Masa üzerinde eski bir saat durmaktadır.

ALİ
Bu kayıt tam olarak iki sayfa üretmeli.

===

DIŞ. BAHÇE - GECE

> KESME.
";
    let doc = parse_fountain(fountain, ScriptLanguage::Tr);

    // 1. A4 golden verification (1 title page + 2 script pages = 3 PDF pages)
    let a4_profile = LayoutProfile::for_page_size(PageSize::A4);
    let a4_paged = paginate(&doc, &a4_profile);
    assert_eq!(a4_paged.page_count(), 2);

    let scene = &a4_paged.pages[0].lines[0];
    let action = a4_paged.pages[0]
        .lines
        .iter()
        .find(|l| l.runs.iter().any(|r| r.text.contains("Masa")))
        .expect("action line");
    let character = a4_paged.pages[0]
        .lines
        .iter()
        .find(|l| l.runs.iter().any(|r| r.text.contains("ALİ")))
        .expect("character line");
    let transition = a4_paged.pages[1]
        .lines
        .iter()
        .find(|l| l.runs.iter().any(|r| r.text.contains("KESME")))
        .expect("transition line");

    assert!((scene.x_pt - 108.0).abs() < 1.0);
    assert!((action.x_pt - 108.0).abs() < 1.0);
    assert!(character.x_pt > action.x_pt + 100.0);
    assert_eq!(transition.align, dram_core::LineAlign::Right);
    assert!(transition.right_x_pt > character.x_pt);
    assert!(a4_paged.pages[1].lines[0].y_pt <= scene.y_pt + 0.1);

    let a4_pdf = export_pdf_bytes(&doc, &a4_profile).expect("A4 PDF");
    assert!(a4_pdf.starts_with(b"%PDF-"));
    assert!(String::from_utf8_lossy(&a4_pdf).contains("%%EOF"));
    assert_eq!(count_pdf_pages(&a4_pdf), 1 + a4_paged.page_count());
    let a4_text = String::from_utf8_lossy(&a4_pdf);
    assert!(a4_text.contains("595."));

    // 2. US Letter golden verification (612 x 792 pt MediaBox)
    let letter_profile = LayoutProfile::for_page_size(PageSize::UsLetter);
    let letter_paged = paginate(&doc, &letter_profile);
    let letter_pdf = export_pdf_bytes(&doc, &letter_profile).expect("Letter PDF");
    assert_eq!(count_pdf_pages(&letter_pdf), 1 + letter_paged.page_count());
    let letter_text = String::from_utf8_lossy(&letter_pdf);
    assert!(letter_text.contains("612 "));
}
