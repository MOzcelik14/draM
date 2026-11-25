//! Synthetic screenplay performance benchmark suite for `draM.`.
//!
//! Measures cold `.dram` load, initial & incremental pagination, simulated keystroke
//! latency, autosave recovery write, PDF export, scroll virtualization cost, and peak RSS
//! across 5, 30, 120, and 500-page synthetic screenplays.
//!
//! Run with:
//! `cargo run --release -p dram-pdf --example perf_bench`

use std::io::Cursor;
use std::time::Instant;

use dram_core::{
    paginate, Document, EditCommand, Element, ElementType, LayoutProfile, PageSize, ScriptLanguage,
    TextRun, UndoManager,
};
use dram_format::{read_dram_from_reader, write_dram_to_writer, RecoveryCoordinator};
use dram_pdf::export_pdf_bytes;

fn build_synthetic_script(target_pages: usize) -> Document {
    let mut doc = Document::new(ScriptLanguage::Tr);
    doc.title_page.title = format!("SENTETİK SENARYO ({target_pages} SAYFA)");
    doc.title_page.author = "draM. Benchmark".to_string();
    doc.elements.clear();

    // ~14 scenes per 5 pages -> roughly ~5.5 elements per scene (~2.8 scenes/page).
    let scene_count = (target_pages * 3).max(3);
    for s in 0..scene_count {
        doc.elements.push(Element::new(
            ElementType::SceneHeading,
            format!("İÇ. İSTASYON BÖLÜM {} - GECE", s + 1),
        ));
        doc.elements.push(Element::new(
            ElementType::Action,
            "Yağmur peronun camlarına vururken görevli elindeki feneri koridorun sonuna doğru tutar ve adımlarını hızlandırır.",
        ));
        doc.elements.push(Element::new(
            ElementType::Character,
            if s % 2 == 0 { "AYŞE" } else { "MEHMET" },
        ));
        doc.elements.push(Element::new(
            ElementType::Dialogue,
            "Bütün kayıtları kontrol ettik, zaman çizelgesi tam olarak beklendiği gibi ilerliyor ve hiçbir sapma görünmüyor.",
        ));
        doc.elements.push(Element::new(
            ElementType::Character,
            if s % 2 == 0 { "MEHMET" } else { "AYŞE" },
        ));
        doc.elements.push(Element::new(
            ElementType::Dialogue,
            "Güzel. Yedek kopyayı arşive aktaralım ve bir sonraki sahneye geçelim.",
        ));
    }
    doc.renumber_scenes();
    doc
}

fn read_peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb_str = rest.split_whitespace().next()?;
            return kb_str.parse::<u64>().ok();
        }
    }
    None
}

fn bench_scale(target_pages: usize) {
    let doc = build_synthetic_script(target_pages);
    let profile = LayoutProfile::for_page_size(PageSize::A4);

    // Serialize once into memory for cold load benchmark.
    let mut archive_bytes = Cursor::new(Vec::new());
    write_dram_to_writer(&mut archive_bytes, &doc).expect("serialize .dram");
    let raw_archive = archive_bytes.into_inner();

    // 1. Cold open (.dram parse + decompress)
    let t0 = Instant::now();
    let mut loaded = read_dram_from_reader(Cursor::new(&raw_archive)).expect("load .dram");
    let open_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // 2. Initial pagination
    let t1 = Instant::now();
    let paged = paginate(&loaded, &profile);
    let first_pag_ms = t1.elapsed().as_secs_f64() * 1000.0;

    // 3. Simulated keystroke latency (20 UpdateRunsById + plain_text)
    let mut undo = UndoManager::new();
    let target_id = loaded.elements[loaded.elements.len() / 2].id;
    let t2 = Instant::now();
    for step in 0..20 {
        let prev = loaded
            .elements
            .iter()
            .find(|e| e.id == target_id)
            .unwrap()
            .runs
            .clone();
        undo.execute(
            &mut loaded,
            EditCommand::UpdateRunsById {
                id: target_id,
                old_runs: prev,
                new_runs: vec![TextRun::plain(format!("Tuş vuruşu denemesi {step}"))],
            },
        )
        .expect("keystroke edit");
    }
    let avg_key_us = (t2.elapsed().as_secs_f64() * 1_000_000.0) / 20.0;

    // 4. Re-pagination after edit
    let t3 = Instant::now();
    let _ = paginate(&loaded, &profile);
    let repag_ms = t3.elapsed().as_secs_f64() * 1000.0;

    // 5. Autosave (RecoveryCoordinator)
    let tmp_dir = std::env::temp_dir().join("dram_perf_bench");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let proj_path = tmp_dir.join(format!("bench_{target_pages}.dram"));
    let coord = RecoveryCoordinator::new();
    let gen = coord.bump_generation();
    let t4 = Instant::now();
    let _ = coord
        .try_write_recovery(&proj_path, &loaded, gen)
        .expect("autosave");
    let autosave_ms = t4.elapsed().as_secs_f64() * 1000.0;
    let _ = coord.invalidate_and_remove(&proj_path);

    // 6. PDF generation
    let t5 = Instant::now();
    let pdf = export_pdf_bytes(&loaded, &profile).expect("pdf export");
    let pdf_ms = t5.elapsed().as_secs_f64() * 1000.0;

    // 7. Scroll virtualization window scan simulation (60 frames across document)
    let total = loaded.elements.len();
    let t6 = Instant::now();
    let mut checksum = 0usize;
    for f in 0..60 {
        let focus = (f * total / 60).min(total.saturating_sub(1));
        let start = focus.saturating_sub(60);
        let end = (focus + 61).min(total);
        checksum += end - start;
    }
    let scroll_frame_us = (t6.elapsed().as_secs_f64() * 1_000_000.0) / 60.0;
    std::hint::black_box(checksum);

    let peak_rss_mb = read_peak_rss_kib().map_or(0.0, |k| k as f64 / 1024.0);

    println!(
        "{:>4}p (actual {:>3}p, {:>4} elems) | open: {:>6.2} ms | pag1: {:>6.2} ms | repag: {:>6.2} ms | key: {:>6.2} us | autosave: {:>6.2} ms | pdf: {:>7.2} ms ({:>5} KB) | scroll: {:>5.2} us | peakRSS: {:>6.1} MB",
        target_pages,
        paged.page_count(),
        total,
        open_ms,
        first_pag_ms,
        repag_ms,
        avg_key_us,
        autosave_ms,
        pdf_ms,
        pdf.len() / 1024,
        scroll_frame_us,
        peak_rss_mb,
    );
}

fn main() {
    println!("draM. Synthetic Performance Benchmark Suite (A4, Release)");
    println!("{}", "-".repeat(132));
    for pages in [5, 30, 120, 500] {
        bench_scale(pages);
    }
}
