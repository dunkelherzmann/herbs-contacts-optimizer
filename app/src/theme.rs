//! The app's colours, after macOS 27's Finder, for light and dark. Only the colours: the layout and
//! the shapes are the app's own.
//!
//! The dark values keep the steps between the parts of a Finder window as captured on macOS 27
//! (sidebar a little darker than the content, bars a little lighter, controls lighter still), on
//! the system's window colour. The light values are the system's colours for the light
//! appearance; no light Finder window was looked at.

use egui::{Color32, CornerRadius, Stroke};

/// The colours the window is drawn with that egui's own style has no place for.
#[derive(Clone, Copy)]
pub struct Palette {
    /// The list and the contact's panel.
    pub content: Color32,
    /// The panel of address books.
    pub sidebar: Color32,
    /// The toolbar and the status bar.
    pub bar: Color32,
    /// A button, and one under the pointer or pressed.
    pub capsule: Color32,
    pub capsule_on: Color32,
    pub secondary: Color32,
    pub separator: Color32,
    pub danger: Color32,
}

const fn grey(value: u8) -> Color32 {
    Color32::from_gray(value)
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette { content: grey(0x1e), sidebar: grey(0x1b), bar: grey(0x26), capsule: grey(0x38), capsule_on: grey(0x4c), secondary: grey(0x9a), separator: grey(0x39), danger: Color32::from_rgb(0xff, 0x42, 0x45) }
    } else {
        Palette { content: grey(0xff), sidebar: grey(0xf3), bar: grey(0xf9), capsule: grey(0xec), capsule_on: grey(0xdc), secondary: grey(0x80), separator: grey(0xe6), danger: Color32::from_rgb(0xff, 0x38, 0x3c) }
    }
}

/// The palette for the appearance this part of the window is drawn in.
pub fn of(ui: &egui::Ui) -> Palette {
    palette(ui.visuals().dark_mode)
}

fn visuals(dark: bool) -> egui::Visuals {
    let p = palette(dark);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let label = if dark { grey(0xdd) } else { grey(0x26) };
    let radius = CornerRadius::same(6);
    v.panel_fill = p.content;
    v.window_fill = if dark { grey(0x2a) } else { grey(0xf6) };
    v.extreme_bg_color = p.content;
    v.text_edit_bg_color = Some(if dark { grey(0x30) } else { grey(0xff) });
    // The system's second row colour: 5% white on dark, #f4f5f5 on light.
    v.faint_bg_color = if dark { grey(0x29) } else { Color32::from_rgb(0xf4, 0xf5, 0xf5) };
    v.weak_text_color = Some(p.secondary);
    v.hyperlink_color = if dark { Color32::from_rgb(0x41, 0x9c, 0xff) } else { Color32::from_rgb(0x00, 0x68, 0xda) };
    v.selection.bg_fill = if dark { Color32::from_rgb(0x00, 0x59, 0xd1) } else { Color32::from_rgb(0x00, 0x64, 0xe1) };
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_stroke = Stroke::new(1.0, p.separator);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, label);
    for (widget, fill) in [(&mut v.widgets.inactive, p.capsule), (&mut v.widgets.hovered, p.capsule_on), (&mut v.widgets.active, p.capsule_on), (&mut v.widgets.open, p.capsule_on)] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke = Stroke::new(1.0, label);
        widget.corner_radius = radius;
        widget.expansion = 0.0;
    }
    v.widgets.noninteractive.corner_radius = radius;
    v
}

/// Gives the window its fonts, sizes, spacing and both sets of colours.
pub fn install(ctx: &egui::Context) {
    // The Mac's own font on the Mac; elsewhere Inter, which is close to it and travels inside the app,
    // so the three versions look alike whatever fonts a machine has.
    #[cfg(target_os = "macos")]
    let font = std::fs::read("/System/Library/Fonts/SFNS.ttf").ok();
    #[cfg(not(target_os = "macos"))]
    let font = Some(include_bytes!("../assets/Inter-Regular.ttf").to_vec());
    if let Some(bytes) = font {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("system".into(), egui::FontData::from_owned(bytes).into());
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "system".into());
        ctx.set_fonts(fonts);
    }
    ctx.set_visuals_of(egui::Theme::Dark, visuals(true));
    ctx.set_visuals_of(egui::Theme::Light, visuals(false));
    ctx.all_styles_mut(|style| {
        use egui::{FontFamily::Proportional, FontId, TextStyle};
        style.text_styles.insert(TextStyle::Body, FontId::new(13.5, Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(13.5, Proportional));
        style.text_styles.insert(TextStyle::Small, FontId::new(11.0, Proportional));
        style.text_styles.insert(TextStyle::Heading, FontId::new(19.0, Proportional));
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 26.0;
    });
}
