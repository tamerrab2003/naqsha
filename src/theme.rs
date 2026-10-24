//! Nixpx-derived colour system for light and dark appearance.
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Pal {
    pub dark: bool,
    pub bg: Color32,
    pub chrome: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    #[allow(dead_code)]
    pub accent2: Color32,
    pub accent_soft: Color32,
    pub canvas: Color32,
    pub page: Color32,
    pub grid_minor: Color32,
    pub grid_major: Color32,
    pub ruler: Color32,
    pub ruler_text: Color32,
    pub shape_fill: Color32,
    pub shape_stroke: Color32,
    pub shape_text: Color32,
    pub cont_fill: Color32,
    pub conn: Color32,
    pub sel: Color32,
    pub guide: Color32,
    pub glue: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 { Color32::from_rgb(r, g, b) }
const fn ca(r: u8, g: u8, b: u8, a: u8) -> Color32 { Color32::from_rgba_premultiplied((r as u16 * a as u16 / 255) as u8, (g as u16 * a as u16 / 255) as u8, (b as u16 * a as u16 / 255) as u8, a) }

pub const NX_NIGHT: Color32 = c(0x0F, 0x0F, 0x1A);

pub const LIGHT: Pal = Pal {
    dark: false,
    bg: c(0xF4, 0xF4, 0xFA),
    chrome: c(0xEC, 0xEC, 0xF5),
    panel: c(0xFF, 0xFF, 0xFF),
    panel2: c(0xF6, 0xF6, 0xFB),
    hover: c(0xEE, 0xEC, 0xFB),
    border: c(0xE1, 0xE1, 0xEC),
    text: c(0x15, 0x15, 0x2A),
    muted: c(0x67, 0x67, 0x84),
    faint: c(0x9B, 0x9B, 0xB4),
    accent: c(0x6C, 0x4D, 0xFF),
    accent2: c(0x19, 0xC3, 0xDD),
    accent_soft: c(0xEA, 0xE5, 0xFF),
    canvas: c(0xE7, 0xE7, 0xF1),
    page: c(0xFF, 0xFF, 0xFF),
    grid_minor: ca(60, 60, 120, 16),
    grid_major: ca(60, 60, 120, 34),
    ruler: c(0xF6, 0xF6, 0xFB),
    ruler_text: c(0x8A, 0x8A, 0xA6),
    shape_fill: c(0xEF, 0xEB, 0xFF),
    shape_stroke: c(0x6C, 0x4D, 0xFF),
    shape_text: c(0x17, 0x17, 0x2C),
    cont_fill: ca(108, 77, 255, 12),
    conn: c(0x4F, 0x4F, 0x6E),
    sel: c(0x19, 0xA7, 0xC9),
    guide: c(0xFF, 0x3D, 0x8B),
    glue: c(0x22, 0xC5, 0x5E),
};

pub const DARK: Pal = Pal {
    dark: true,
    bg: c(0x0F, 0x0F, 0x1A),
    chrome: c(0x0C, 0x0C, 0x16),
    panel: c(0x15, 0x15, 0x24),
    panel2: c(0x1A, 0x1A, 0x2C),
    hover: c(0x23, 0x23, 0x40),
    border: c(0x25, 0x25, 0x3A),
    text: c(0xE9, 0xE8, 0xF7),
    muted: c(0x9A, 0x99, 0xB8),
    faint: c(0x66, 0x65, 0x8A),
    accent: c(0x8B, 0x73, 0xFF),
    accent2: c(0x2A, 0xD4, 0xEE),
    accent_soft: c(0x2A, 0x24, 0x52),
    canvas: c(0x0A, 0x0A, 0x13),
    page: c(0x13, 0x13, 0x1F),
    grid_minor: ca(160, 160, 230, 11),
    grid_major: ca(160, 160, 230, 24),
    ruler: c(0x11, 0x11, 0x20),
    ruler_text: c(0x6E, 0x6D, 0x92),
    shape_fill: c(0x22, 0x1D, 0x45),
    shape_stroke: c(0x8F, 0x78, 0xFF),
    shape_text: c(0xEC, 0xEB, 0xFA),
    cont_fill: ca(139, 115, 255, 16),
    conn: c(0xA6, 0xA5, 0xC6),
    sel: c(0x2A, 0xD4, 0xEE),
    guide: c(0xFF, 0x4F, 0x98),
    glue: c(0x22, 0xC5, 0x5E),
};

/// Swatches offered in colour pickers (Nixpx brand-adjacent).
pub const SWATCHES: &[&str] = &[
    "#6C4DFF", "#8B73FF", "#19C3DD", "#10B981", "#F59E0B", "#EF4444", "#EC4899", "#3B82F6",
    "#EFEBFF", "#E0F7FB", "#DCFCE7", "#FEF3C7", "#FEE2E2", "#FCE7F3", "#DBEAFE", "#F1F5F9",
    "#0F0F1A", "#334155", "#64748B", "#94A3B8", "#CBD5E1", "#FFFFFF",
];

/// Diagram themes: (name, fill, stroke, text, connector)
pub const DIAGRAM_THEMES: &[(&str, &str, &str, &str, &str)] = &[
    ("Nixpx", "#EFEBFF", "#6C4DFF", "#17172C", "#4F4F6E"),
    ("Night", "#1D1A3A", "#8B73FF", "#ECEBFA", "#A6A5C6"),
    ("Lagoon", "#E0F7FB", "#0E9FB8", "#0B3440", "#3E6F7A"),
    ("Mint", "#DCFCE7", "#16A34A", "#0F2E1C", "#3F6B52"),
    ("Ember", "#FEF0E2", "#EA580C", "#3B1A08", "#7C4A2D"),
    ("Rose", "#FCE7F3", "#DB2777", "#3F0D24", "#7A3C5A"),
    ("Slate", "#F1F5F9", "#475569", "#0F172A", "#475569"),
    ("Mono", "#FFFFFF", "#111111", "#111111", "#111111"),
];

pub fn to_c32(c: crate::model::Col) -> Color32 { Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3]) }
pub fn from_c32(c: Color32) -> crate::model::Col { let [r, g, b, a] = c.to_srgba_unmultiplied(); crate::model::Col([r, g, b, a]) }

pub fn visuals(p: &Pal) -> Visuals {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.panel2;
    v.faint_bg_color = p.panel2;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.hyperlink_color = p.accent;
    v.override_text_color = Some(p.text);
    let r = CornerRadius::same(6);
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.bg_fill = p.panel2;
    v.widgets.inactive.weak_bg_fill = p.panel2;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.2, p.text);
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.bg_fill = p.accent_soft;
    v.widgets.active.weak_bg_fill = p.accent_soft;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.2, p.accent);
    v.widgets.active.corner_radius = r;
    v.widgets.open = v.widgets.active;
    v.slider_trailing_fill = true;
    v
}

/// Prefer the macOS system font when present; fall back to egui's bundled fonts elsewhere.
pub fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::default();
    let candidates_regular = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    let candidates_bold = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ];
    // the browser has no font files to read: it carries a subset of DejaVu Sans (web/fonts),
    // which has the arrows, symbols and Arabic letters egui's built-in font lacks
    #[cfg(target_arch = "wasm32")]
    let (web_regular, web_bold): (Option<Vec<u8>>, Option<Vec<u8>>) = (Some(include_bytes!("../web/fonts/DejaVuSans.ttf").to_vec()), Some(include_bytes!("../web/fonts/DejaVuSans-Bold.ttf").to_vec()));
    #[cfg(not(target_arch = "wasm32"))]
    let (web_regular, web_bold): (Option<Vec<u8>>, Option<Vec<u8>>) = (None, None);
    if let Some(bytes) = web_regular.or_else(|| candidates_regular.iter().find_map(|p| std::fs::read(p).ok())) {
        f.font_data.insert("system".into(), Arc::new(FontData::from_owned(bytes)));
        f.families.entry(FontFamily::Proportional).or_default().insert(0, "system".into());
    }
    let mut bold_list = vec![];
    if let Some(bytes) = web_bold.or_else(|| candidates_bold.iter().find_map(|p| std::fs::read(p).ok())) {
        f.font_data.insert("system-bold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_list.push("system-bold".to_string());
    }
    // Arabic: the system UI fonts (San Francisco, Segoe UI) don't map Arabic presentation forms,
    // which is what the canvas draws Arabic with (see rtl.rs); DejaVu Sans does
    #[cfg(not(target_arch = "wasm32"))]
    {
        f.font_data.insert("arabic".into(), Arc::new(FontData::from_static(include_bytes!("../web/fonts/DejaVuSans.ttf"))));
        f.font_data.insert("arabic-bold".into(), Arc::new(FontData::from_static(include_bytes!("../web/fonts/DejaVuSans-Bold.ttf"))));
        let prop = f.families.entry(FontFamily::Proportional).or_default();
        let at = prop.iter().position(|n| n == "system").map(|i| i + 1).unwrap_or(0);
        prop.insert(at, "arabic".into());
        bold_list.push("arabic-bold".into());
    }
    bold_list.extend(f.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    f.families.insert(FontFamily::Name("bold".into()), bold_list);
    crate::scripts::add_to(&mut f);
    f
}
