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
