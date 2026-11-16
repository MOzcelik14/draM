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
    let ayse_exists = paged
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .any(|line| line.runs.iter().any(|r| r.text.contains("AYŞE")));
    let mehmet_exists = paged
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .any(|line| line.runs.iter().any(|r| r.text.contains("MEHMET")));
    assert!(ayse_exists && mehmet_exists);

    let pdf_bytes = export_pdf_bytes(&reloaded, &profile).expect("export PDF bytes");
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.len() > 5000);
}
