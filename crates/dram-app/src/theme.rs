//! Typography and Fade In-inspired visual theme configuration for `dram-app`.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Stroke, Vec2,
};

const COURIER_PRIME_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/CourierPrime-Regular.ttf");
const COURIER_PRIME_BOLD: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-Bold.ttf");

/// Signature crimson accent used for active format items and active navigator scene cards.
pub const ACCENT_CRIMSON: Color32 = Color32::from_rgb(163, 29, 29);
/// Muted crimson card fill for active scene cards in the navigator.
pub const ACCENT_CRIMSON_DARK: Color32 = Color32::from_rgb(120, 22, 22);

/// Registers embedded Courier Prime fonts for authentic screenplay typography in the editor.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "courier_prime_regular".to_string(),
        FontData::from_static(COURIER_PRIME_REGULAR).into(),
    );
    fonts.font_data.insert(
        "courier_prime_bold".to_string(),
        FontData::from_static(COURIER_PRIME_BOLD).into(),
    );

    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "courier_prime_regular".to_string());
    }
    ctx.set_fonts(fonts);
}

/// Color tokens for the screenplay desk and crisp white paper sheet.
pub struct PaperPalette {
    /// Background behind the centered screenplay sheet.
    pub desk_bg: Color32,
    /// Screenplay paper sheet fill (always crisp white like Fade In).
    pub paper_bg: Color32,
    /// Screenplay paper sheet 1px border.
    pub paper_border: Color32,
    /// Primary ink color on the paper sheet.
    pub paper_ink: Color32,
    /// Subtle highlight for the currently focused element row on the paper sheet.
    pub active_row_bg: Color32,
    /// Muted text color for secondary annotations (synopsis, notes, badges).
    pub muted_text: Color32,
}

/// Returns the [`PaperPalette`] matching `dark` mode.
#[must_use]
pub const fn paper_palette(dark: bool) -> PaperPalette {
    if dark {
        PaperPalette {
            desk_bg: Color32::from_rgb(47, 49, 54),
            paper_bg: Color32::from_rgb(255, 255, 255),
            paper_border: Color32::from_rgb(32, 34, 38),
            paper_ink: Color32::from_rgb(18, 19, 22),
            active_row_bg: Color32::from_rgb(242, 244, 248),
            muted_text: Color32::from_rgb(107, 114, 128),
        }
    } else {
        PaperPalette {
            desk_bg: Color32::from_rgb(228, 231, 236),
            paper_bg: Color32::from_rgb(255, 255, 255),
            paper_border: Color32::from_rgb(203, 208, 218),
            paper_ink: Color32::from_rgb(18, 19, 22),
            active_row_bg: Color32::from_rgb(242, 244, 248),
            muted_text: Color32::from_rgb(107, 114, 128),
        }
    }
}

/// Applies consistent spacing, corner radii, and Fade In-inspired studio dark/light colors to `ctx`.
pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(38, 39, 43);
        v.window_fill = Color32::from_rgb(42, 44, 49);
        v.extreme_bg_color = Color32::from_rgb(30, 31, 35);
        v.faint_bg_color = Color32::from_rgb(48, 50, 56);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(56, 58, 65));
        v.widgets.inactive.bg_fill = Color32::from_rgb(52, 54, 61);
        v.widgets.hovered.bg_fill = Color32::from_rgb(64, 67, 75);
        v.widgets.active.bg_fill = ACCENT_CRIMSON;
        v.selection.bg_fill = ACCENT_CRIMSON;
        v
    } else {
        let mut v = egui::Visuals::light();
        v.panel_fill = Color32::from_rgb(244, 245, 247);
        v.window_fill = Color32::from_rgb(255, 255, 255);
        v.extreme_bg_color = Color32::from_rgb(236, 238, 242);
        v.faint_bg_color = Color32::from_rgb(238, 240, 244);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(214, 218, 226));
        v.widgets.inactive.bg_fill = Color32::from_rgb(233, 236, 241);
        v.widgets.hovered.bg_fill = Color32::from_rgb(220, 224, 232);
        v.widgets.active.bg_fill = ACCENT_CRIMSON;
        v.selection.bg_fill = ACCENT_CRIMSON;
        v
    };

    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(4);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(6.0, 5.0);
        style.spacing.button_padding = Vec2::new(9.0, 4.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.menu_margin = Margin::same(6);
    });
}

/// Draws the `draM.` Sayfa & Kadraj monogram mark at `size × size` logical pixels.
pub fn draw_logo_mark(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let p = ui.painter();
    let s = size / 64.0;
    let pt = |x: f32, y: f32| egui::pos2(rect.min.x + x * s, rect.min.y + y * s);

    // Base rounded square
    p.rect(
        rect,
        CornerRadius::same((14.0 * s).round().max(2.0) as u8),
        Color32::from_rgb(23, 24, 28),
        Stroke::new((1.5 * s).max(1.0), Color32::from_rgb(44, 49, 60)),
        egui::StrokeKind::Inside,
    );

    // Cinema viewfinder corner brackets
    let bracket_stroke = Stroke::new((1.75 * s).max(1.0), Color32::from_rgb(90, 96, 109));
    p.line_segment([pt(13.0, 20.0), pt(13.0, 14.0)], bracket_stroke);
    p.line_segment([pt(13.0, 14.0), pt(19.0, 14.0)], bracket_stroke);
    p.line_segment([pt(51.0, 20.0), pt(51.0, 14.0)], bracket_stroke);
    p.line_segment([pt(51.0, 14.0), pt(45.0, 14.0)], bracket_stroke);
    p.line_segment([pt(13.0, 44.0), pt(13.0, 50.0)], bracket_stroke);
    p.line_segment([pt(13.0, 50.0), pt(19.0, 50.0)], bracket_stroke);
    p.line_segment([pt(51.0, 44.0), pt(51.0, 50.0)], bracket_stroke);
    p.line_segment([pt(51.0, 50.0), pt(45.0, 50.0)], bracket_stroke);

    // Folded screenplay page
    p.add(egui::Shape::convex_polygon(
        vec![
            pt(21.0, 15.0),
            pt(36.0, 15.0),
            pt(44.0, 23.0),
            pt(44.0, 49.0),
            pt(21.0, 49.0),
        ],
        Color32::from_rgb(247, 246, 242),
        Stroke::NONE,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![pt(36.0, 15.0), pt(44.0, 23.0), pt(36.0, 23.0)],
        Color32::from_rgb(224, 178, 99),
        Stroke::NONE,
    ));

    // Script lines & accent dot
    if size >= 24.0 {
        p.rect_filled(
            egui::Rect::from_min_max(pt(25.0, 28.0), pt(39.0, 30.0)),
            1.0,
            Color32::from_rgb(23, 24, 28),
        );
        p.rect_filled(
            egui::Rect::from_min_max(pt(25.0, 33.0), pt(36.0, 35.0)),
            1.0,
            Color32::from_rgb(90, 96, 109),
        );
        p.rect_filled(
            egui::Rect::from_min_max(pt(28.0, 38.0), pt(37.0, 40.0)),
            1.0,
            Color32::from_rgb(90, 96, 109),
        );
    }
    p.circle_filled(
        pt(40.5, 43.5),
        (2.2 * s).max(1.2),
        Color32::from_rgb(224, 178, 99),
    );
}

/// Generates a 64×64 RGBA window/taskbar icon matching `assets/logo-mark.svg`.
#[must_use]
pub fn app_icon() -> egui::IconData {
    const W: usize = 64;
    const H: usize = 64;
    let mut rgba = vec![0u8; W * H * 4];

    let mut set_px = |x: usize, y: usize, c: [u8; 4]| {
        if x < W && y < H {
            let i = (y * W + x) * 4;
            rgba[i..i + 4].copy_from_slice(&c);
        }
    };

    let bg = [23, 24, 28, 255];
    let border = [44, 49, 60, 255];
    let bracket = [90, 96, 109, 255];
    let paper = [247, 246, 242, 255];
    let gold = [224, 178, 99, 255];

    // Rounded square (x, y in 2..62, radius 12)
    for y in 2..62 {
        for x in 2..62 {
            let dx = if x < 14 {
                14 - x
            } else if x >= 50 {
                x - 49
            } else {
                0
            };
            let dy = if y < 14 {
                14 - y
            } else if y >= 50 {
                y - 49
            } else {
                0
            };
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= 144 {
                let is_border = !(3..61).contains(&x) || !(3..61).contains(&y) || dist_sq >= 121;
                set_px(x, y, if is_border { border } else { bg });
            }
        }
    }

    // Viewfinder brackets
    for (x0, y0, x1, y1) in [
        (13, 14, 19, 15),
        (13, 14, 14, 20),
        (45, 14, 51, 15),
        (50, 14, 51, 20),
        (13, 49, 19, 50),
        (13, 44, 14, 50),
        (45, 49, 51, 50),
        (50, 44, 51, 50),
    ] {
        for y in y0..=y1 {
            for x in x0..=x1 {
                set_px(x, y, bracket);
            }
        }
    }

    // Folded screenplay page (x: 21..44, y: 15..49, dog-ear: x+y > 36+23=59 -> top-right cut)
    for y in 15..=49 {
        for x in 21..=44 {
            if x >= 36 && y <= 23 {
                if (x - 36) <= (y - 15) {
                    set_px(x, y, gold);
                }
            } else {
                set_px(x, y, paper);
            }
        }
    }

    // Script lines
    for y in 28..=29 {
        for x in 25..=39 {
            set_px(x, y, bg);
        }
    }
    for y in 33..=34 {
        for x in 25..=36 {
            set_px(x, y, bracket);
        }
    }
    for y in 38..=39 {
        for x in 28..=37 {
            set_px(x, y, bracket);
        }
    }
    // Accent dot at (40, 43)
    for y in 42..=45 {
        for x in 39..=42 {
            let dx = x as isize - 40;
            let dy = y as isize - 43;
            if dx * dx + dy * dy <= 4 {
                set_px(x, y, gold);
            }
        }
    }

    egui::IconData {
        rgba,
        width: W as u32,
        height: H as u32,
    }
}
