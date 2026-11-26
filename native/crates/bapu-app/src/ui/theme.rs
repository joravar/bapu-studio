//! Bapu Studio custom theme system — faithfully porting the HSL dark & light palettes
//! from `src/index.css` into `egui::Style` and `egui::Visuals`.

use egui::{Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle, Visuals};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {
    pub dark: bool,
    pub bg_app: Color32,
    pub bg_sidebar: Color32,
    pub bg_header: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_input: Color32,
    pub bg_console: Color32,
    pub border_subtle: Color32,
    pub border_active: Color32,
    pub accent_primary: Color32,
    pub accent_hover: Color32,
    pub accent_cyan: Color32,
    pub accent_emerald: Color32,
    pub accent_amber: Color32,
    pub accent_violet: Color32,
    pub accent_rose: Color32,
    pub accent_pink: Color32,
    pub text_main: Color32,
    pub text_muted: Color32,
    pub text_dim: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg_app: Color32::from_rgb(9, 13, 20),        // #090d14
            bg_sidebar: Color32::from_rgb(15, 21, 34),   // #0f1522
            bg_header: Color32::from_rgb(13, 19, 31),    // #0d131f
            bg_card: Color32::from_rgb(19, 28, 45),      // #131c2d
            bg_card_hover: Color32::from_rgb(24, 35, 56),// #182338
            bg_input: Color32::from_rgb(12, 18, 30),     // #0c121e
            bg_console: Color32::from_rgb(8, 12, 19),    // #080c13
            border_subtle: Color32::from_rgb(28, 39, 60),// #1c273c
            border_active: Color32::from_rgb(43, 59, 90),// #2b3b5a
            accent_primary: Color32::from_rgb(59, 130, 246), // #3b82f6
            accent_hover: Color32::from_rgb(37, 99, 235),    // #2563eb
            accent_cyan: Color32::from_rgb(6, 182, 212),     // #06b6d4
            accent_emerald: Color32::from_rgb(16, 185, 129), // #10b981
            accent_amber: Color32::from_rgb(245, 158, 11),   // #f59e0b
            accent_violet: Color32::from_rgb(139, 92, 246),  // #8b5cf6
            accent_rose: Color32::from_rgb(239, 68, 68),     // #ef4444
            accent_pink: Color32::from_rgb(236, 72, 153),    // #ec4899
            text_main: Color32::from_rgb(241, 245, 249),     // #f1f5f9
            text_muted: Color32::from_rgb(148, 163, 184),    // #94a3b8
            text_dim: Color32::from_rgb(100, 116, 139),      // #64748b
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg_app: Color32::from_rgb(244, 246, 249),        // #f4f6f9
            bg_sidebar: Color32::from_rgb(238, 241, 246),    // #eef1f6
            bg_header: Color32::from_rgb(255, 255, 255),     // #ffffff
            bg_card: Color32::from_rgb(255, 255, 255),       // #ffffff
            bg_card_hover: Color32::from_rgb(238, 242, 247), // #eef2f7
            bg_input: Color32::from_rgb(248, 250, 252),      // #f8fafc
            bg_console: Color32::from_rgb(241, 245, 249),    // #f1f5f9
            border_subtle: Color32::from_rgb(225, 230, 237), // #e1e6ed
            border_active: Color32::from_rgb(199, 207, 219), // #c7cfdb
            accent_primary: Color32::from_rgb(37, 99, 235),  // #2563eb
            accent_hover: Color32::from_rgb(29, 78, 216),    // #1d4ed8
            accent_cyan: Color32::from_rgb(8, 145, 178),     // #0891b2
            accent_emerald: Color32::from_rgb(5, 150, 105),  // #059669
            accent_amber: Color32::from_rgb(217, 119, 6),    // #d97706
            accent_violet: Color32::from_rgb(124, 58, 237),  // #7c3aed
            accent_rose: Color32::from_rgb(220, 38, 38),     // #dc2626
            accent_pink: Color32::from_rgb(219, 39, 119),    // #db2777
            text_main: Color32::from_rgb(15, 23, 42),        // #0f172a
            text_muted: Color32::from_rgb(71, 85, 105),      // #475569
            text_dim: Color32::from_rgb(124, 138, 160),      // #7c8aa0
        }
    }

    pub fn for_mode(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let p = Palette::for_mode(dark);
    let mut style = (*ctx.style()).clone();

    style.text_styles.insert(TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(13.2, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(12.8, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace));
    style.text_styles.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));

    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(11.0, 6.0);
    style.spacing.window_margin = Margin::same(16.0_f32);
    style.spacing.menu_margin = Margin::same(8.0_f32);
    style.spacing.indent = 14.0;
    style.spacing.scroll.bar_width = 6.0;

    let mut visuals = if dark { Visuals::dark() } else { Visuals::light() };
    visuals.override_text_color = Some(p.text_main);
    visuals.panel_fill = p.bg_app;
    visuals.window_fill = p.bg_card;
    visuals.window_stroke = Stroke::new(1.0_f32, p.border_active);
    visuals.window_rounding = Rounding::same(12.0_f32);
    visuals.menu_rounding = Rounding::same(8.0_f32);
    visuals.faint_bg_color = p.bg_sidebar;
    visuals.extreme_bg_color = p.bg_input;
    visuals.code_bg_color = p.bg_console;
    visuals.hyperlink_color = p.accent_cyan;
    visuals.warn_fg_color = p.accent_amber;
    visuals.error_fg_color = p.accent_rose;

    let r = Rounding::same(7.0_f32);

    visuals.widgets.noninteractive.bg_fill = p.bg_card;
    visuals.widgets.noninteractive.weak_bg_fill = p.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_muted);
    visuals.widgets.noninteractive.rounding = r;

    visuals.widgets.inactive.bg_fill = p.bg_card;
    visuals.widgets.inactive.weak_bg_fill = p.bg_card;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.2_f32, p.text_main);
    visuals.widgets.inactive.rounding = r;

    visuals.widgets.hovered.bg_fill = p.bg_card_hover;
    visuals.widgets.hovered.weak_bg_fill = p.bg_card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.border_active);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, p.text_main);
    visuals.widgets.hovered.rounding = r;

    visuals.widgets.active.bg_fill = p.accent_primary.gamma_multiply(0.22);
    visuals.widgets.active.weak_bg_fill = p.accent_primary.gamma_multiply(0.22);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, p.accent_primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.text_main);
    visuals.widgets.active.rounding = r;

    visuals.widgets.open.bg_fill = p.bg_card_hover;
    visuals.widgets.open.weak_bg_fill = p.bg_card_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.open.fg_stroke = Stroke::new(1.3_f32, p.text_main);
    visuals.widgets.open.rounding = r;

    visuals.selection.bg_fill = p.accent_primary.gamma_multiply(0.28);
    visuals.selection.stroke = Stroke::new(1.2_f32, p.accent_primary);

    style.visuals = visuals;
    ctx.set_style(style);
}
