//! Typography and visual theme configuration for `dram-app`.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Stroke, Vec2,
};

const COURIER_PRIME_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/CourierPrime-Regular.ttf");
const COURIER_PRIME_BOLD: &[u8] = include_bytes!("../../../assets/fonts/CourierPrime-Bold.ttf");

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

/// Color tokens for the screenplay desk and paper sheet.
pub struct PaperPalette {
    /// Background behind the centered screenplay sheet.
    pub desk_bg: Color32,
    /// Screenplay paper sheet fill.
    pub paper_bg: Color32,
    /// Screenplay paper sheet 1px border.
    pub paper_border: Color32,
    /// Subtle highlight for the currently focused element row.
    pub active_row_bg: Color32,
    /// Muted text color for secondary annotations (synopsis, notes, badges).
    pub muted_text: Color32,
}

/// Returns the [`PaperPalette`] matching `dark` mode.
#[must_use]
pub const fn paper_palette(dark: bool) -> PaperPalette {
    if dark {
        PaperPalette {
            desk_bg: Color32::from_rgb(22, 24, 29),
            paper_bg: Color32::from_rgb(31, 34, 42),
            paper_border: Color32::from_rgb(48, 53, 66),
            active_row_bg: Color32::from_rgb(39, 43, 53),
            muted_text: Color32::from_rgb(144, 152, 168),
        }
    } else {
        PaperPalette {
            desk_bg: Color32::from_rgb(235, 237, 241),
            paper_bg: Color32::from_rgb(255, 255, 255),
            paper_border: Color32::from_rgb(212, 216, 224),
            active_row_bg: Color32::from_rgb(244, 246, 250),
            muted_text: Color32::from_rgb(104, 112, 128),
        }
    }
}

/// Applies consistent spacing, corner radii, and dark/light colors to `ctx`.
pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(27, 30, 36);
        v.window_fill = Color32::from_rgb(30, 33, 40);
        v.extreme_bg_color = Color32::from_rgb(20, 22, 27);
        v.faint_bg_color = Color32::from_rgb(38, 42, 51);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(46, 51, 63));
        v.widgets.inactive.bg_fill = Color32::from_rgb(40, 44, 54);
        v.widgets.hovered.bg_fill = Color32::from_rgb(52, 58, 72);
        v.widgets.active.bg_fill = Color32::from_rgb(62, 70, 88);
        v.selection.bg_fill = Color32::from_rgb(66, 120, 210);
        v
    } else {
        let mut v = egui::Visuals::light();
        v.panel_fill = Color32::from_rgb(246, 247, 249);
        v.window_fill = Color32::from_rgb(255, 255, 255);
        v.extreme_bg_color = Color32::from_rgb(238, 240, 244);
        v.faint_bg_color = Color32::from_rgb(240, 242, 246);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(218, 222, 230));
        v.widgets.inactive.bg_fill = Color32::from_rgb(235, 238, 243);
        v.widgets.hovered.bg_fill = Color32::from_rgb(223, 227, 235);
        v.selection.bg_fill = Color32::from_rgb(43, 108, 176);
        v
    };

    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(6);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(5);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);
    visuals.widgets.active.corner_radius = CornerRadius::same(5);

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = Margin::same(14);
        style.spacing.menu_margin = Margin::same(8);
    });
}
