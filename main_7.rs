use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, Style, Visuals};

// Color palette
pub const BG_PRIMARY: Color32 = Color32::from_rgb(13, 13, 18);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(22, 22, 30);
pub const BG_TERTIARY: Color32 = Color32::from_rgb(32, 32, 42);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 175);
pub const TEXT_DIM: Color32 = Color32::from_rgb(90, 90, 105);

pub const BORDER: Color32 = Color32::from_rgb(55, 55, 70);

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(66, 133, 244);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 199, 89);
pub const ACCENT_RED: Color32 = Color32::from_rgb(255, 69, 58);
pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 204, 0);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(175, 82, 222);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(50, 215, 219);

// Gradient colors
pub const GRADIENT_START: Color32 = Color32::from_rgb(99, 102, 241);  // Indigo
pub const GRADIENT_MID: Color32 = Color32::from_rgb(168, 85, 247);    // Purple
pub const GRADIENT_END: Color32 = Color32::from_rgb(236, 72, 153);    // Pink

pub fn gradient_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);

    if t < 0.5 {
        let t = t * 2.0;
        lerp_color(GRADIENT_START, GRADIENT_MID, t)
    } else {
        let t = (t - 0.5) * 2.0;
        lerp_color(GRADIENT_MID, GRADIENT_END, t)
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

pub fn setup_custom_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    // Load JetBrains Mono
    // You'll need to include this font file or use system fonts
    let font_data = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");

    fonts.font_data.insert(
        "JetBrains".to_owned(),
        FontData::from_static(font_data),
    );

    fonts
        .families
        .entry(FontFamily::Name("JetBrains".into()))
        .or_default()
        .insert(0, "JetBrains".to_owned());

    // Make JetBrains Mono the default proportional font
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "JetBrains".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrains".to_owned());

    ctx.set_fonts(fonts);
}

pub fn setup_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals = Visuals::dark();
    style.visuals.window_fill = BG_PRIMARY;
    style.visuals.panel_fill = BG_PRIMARY;
    style.visuals.faint_bg_color = BG_SECONDARY;
    style.visuals.extreme_bg_color = BG_TERTIARY;
    style.visuals.code_bg_color = BG_TERTIARY;

    style.visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT_SECONDARY;

    style.visuals.widgets.inactive.bg_fill = BG_TERTIARY;
    style.visuals.widgets.inactive.fg_stroke.color = TEXT_SECONDARY;

    style.visuals.widgets.hovered.bg_fill = BG_TERTIARY;
    style.visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;

    style.visuals.widgets.active.bg_fill = ACCENT_BLUE;
    style.visuals.widgets.active.fg_stroke.color = TEXT_PRIMARY;

    style.visuals.selection.bg_fill = ACCENT_BLUE.gamma_multiply(0.4);
    style.visuals.selection.stroke.color = ACCENT_BLUE;

    style.spacing.item_spacing = egui::Vec2::new(8.0, 8.0);
    style.spacing.button_padding = egui::Vec2::new(12.0, 8.0);

    ctx.set_style(style);
}
