//! Vector icon set (18×18 stroke icons) rasterised once with resvg and drawn tinted.
use egui::{pos2, Color32, ColorImage, Painter, Rect, TextureHandle, TextureOptions};
use std::collections::HashMap;

const ICONS: &[(&str, &str)] = &[
    ("star", r##"<path d="M9 1.8l2.1 4.6 5 .5-3.8 3.4 1.1 4.9L9 12.7l-4.4 2.5 1.1-4.9L1.9 6.9l5-.5z"/>"##),
    ("sheet", r##"<rect x="1.8" y="2.5" width="14.4" height="13" rx="1.2"/><path d="M8.5 11.5h7.7M8.5 11.5v4M12.3 11.5v4"/>"##),
    ("schedule", r##"<rect x="2" y="2.5" width="14" height="13" rx="1.2"/><path d="M2 6.3h14M2 10h14M6.5 2.5v13"/>"##),
    ("pipeline", r##"<rect x="1.5" y="6.5" width="4" height="5" rx="1"/><rect x="7" y="6.5" width="4" height="5" rx="1"/><rect x="12.5" y="2.5" width="4" height="4" rx="1"/><rect x="12.5" y="11.5" width="4" height="4" rx="1"/><path d="M5.5 9H7M11 9h.8V4.5h.7M11.8 9v4.5h.7"/>"##),
    ("script", r##"<rect x="1.8" y="2.5" width="14.4" height="13" rx="2"/><path d="M1.8 5.8h14.4M5 9l2 1.8L5 12.6M8.6 12.6h4"/>"##),
    ("template", r##"<rect x="2" y="2" width="6" height="6" rx="1"/><rect x="10" y="2" width="6" height="6" rx="1"/><rect x="2" y="10" width="6" height="6" rx="1"/><path d="M13 10.5v5M10.5 13h5"/>"##),
    ("subpage", r##"<rect x="1.8" y="4.5" width="9.5" height="9" rx="1.2"/><path d="M3.8 2.5h9.7a1.2 1.2 0 0 1 1.2 1.2v7.3"/><path d="M5.5 11.5 8.7 8.3M6 8.2h2.8V11"/>"##),
    ("subsel", r##"<rect x="1.8" y="1.8" width="7" height="7" rx="1" stroke-dasharray="1.8 1.4"/><rect x="9.5" y="9.5" width="6.7" height="6.7" rx="1"/><path d="M6.5 6.5 11.2 11.2M8.6 11.4h2.8V8.6"/>"##),
    ("flows", r##"<rect x="1.5" y="3" width="6" height="4" rx="1"/><rect x="10.5" y="3" width="6" height="4" rx="1"/><rect x="6" y="11" width="6" height="4" rx="1"/><path d="M7.5 5h3M4.5 7c0 4 1.5 6 1.5 6M13.5 7c0 3-1.5 6-1.5 6"/>"##),
    ("play", r##"<path d="M5.5 3.2v11.6L14.6 9z" fill="#fff" fill-opacity=".35"/>"##),
    ("pause", r##"<rect x="4.2" y="3.2" width="3.2" height="11.6" rx=".8"/><rect x="10.6" y="3.2" width="3.2" height="11.6" rx=".8"/>"##),
    ("motion", r##"<path d="M1.8 9h3M1.8 5.5h5M1.8 12.5h5"/><circle cx="12.2" cy="9" r="4"/><path d="M10.6 7.2 13.4 9l-2.8 1.8z" fill="#fff" fill-opacity=".5"/>"##),
    ("pointer", r##"<path d="M4.5 2.5v12l3.2-3.1 2.3 5.1 2-.9-2.3-5h4.3z" fill="#fff" stroke="none"/>"##),
    ("connector", r##"<circle cx="3.5" cy="14.5" r="1.8"/><circle cx="14.5" cy="3.5" r="1.8"/><path d="M5.3 14.5H9V3.5h3.7"/>"##),
    ("text", r##"<path d="M3.5 4V2.8h11V4M9 2.8v12.4M6.8 15.2h4.4"/>"##),
    ("heading", r##"<path d="M4 3v12M14 3v12M4 9h10"/>"##),
    ("pan", r##"<path d="M9 1.8v14.4M1.8 9h14.4M9 1.8 7 3.8M9 1.8l2 2M9 16.2l-2-2M9 16.2l2-2M1.8 9l2-2M1.8 9l2 2M16.2 9l-2-2M16.2 9l-2 2"/>"##),
    ("undo", r##"<path d="M6.5 3.5 3 7l3.5 3.5"/><path d="M3 7h7.5a4.2 4.2 0 0 1 0 8.4H7.5"/>"##),
    ("redo", r##"<path d="M11.5 3.5 15 7l-3.5 3.5"/><path d="M15 7H7.5a4.2 4.2 0 0 0 0 8.4h3"/>"##),
    ("cut", r##"<circle cx="5" cy="13.5" r="2.3"/><circle cx="13" cy="13.5" r="2.3"/><path d="M6.6 11.8 13.5 2.5M11.4 11.8 4.5 2.5"/>"##),
    ("copy", r##"<rect x="6" y="6" width="9.5" height="9.5" rx="1.8"/><path d="M12 6V3.8A1.3 1.3 0 0 0 10.7 2.5H3.8A1.3 1.3 0 0 0 2.5 3.8v6.9A1.3 1.3 0 0 0 3.8 12H6"/>"##),
    ("paste", r##"<rect x="3" y="3.5" width="12" height="12.5" rx="1.8"/><rect x="6.2" y="1.8" width="5.6" height="3.4" rx="1"/>"##),
    ("painter", r##"<rect x="2.5" y="2.5" width="11" height="4.5" rx="1.2"/><path d="M13.5 4.8h1.8v4.2H9v2.2"/><rect x="7.6" y="11.2" width="2.8" height="5" rx=".8"/>"##),
    ("alignL", r##"<path d="M3 4h12M3 7.5h8M3 11h12M3 14.5h8"/>"##),
    ("alignC", r##"<path d="M3 4h12M5 7.5h8M3 11h12M5 14.5h8"/>"##),
    ("alignR", r##"<path d="M3 4h12M7 7.5h8M3 11h12M7 14.5h8"/>"##),
    ("vTop", r##"<path d="M3 3h12M9 6v9M6 9l3-3 3 3"/>"##),
    ("vMid", r##"<path d="M3 9h12M9 2.5v4M9 11.5v4M7 4.8 9 6.6l2-1.8M7 13.2l2-1.8 2 1.8"/>"##),
    ("vBot", r##"<path d="M3 15h12M9 3v9M6 9l3 3 3-3"/>"##),
    ("front", r##"<rect x="2.5" y="2.5" width="8" height="8" rx="1" stroke-dasharray="2 1.6"/><rect x="7.5" y="7.5" width="8" height="8" rx="1" fill="#fff" fill-opacity=".35"/>"##),
    ("back", r##"<rect x="7.5" y="7.5" width="8" height="8" rx="1" stroke-dasharray="2 1.6"/><rect x="2.5" y="2.5" width="8" height="8" rx="1" fill="#fff" fill-opacity=".35"/>"##),
    ("forward", r##"<rect x="3" y="6" width="9" height="9" rx="1" fill="#fff" fill-opacity=".3"/><path d="M14 9V3M11.5 5.5 14 3l2.5 2.5"/>"##),
    ("backward", r##"<rect x="3" y="3" width="9" height="9" rx="1" fill="#fff" fill-opacity=".3"/><path d="M14 9v6M11.5 12.5 14 15l2.5-2.5"/>"##),
    ("group", r##"<rect x="1.8" y="1.8" width="14.4" height="14.4" rx="1.5" stroke-dasharray="2.2 1.8"/><rect x="4.5" y="4.5" width="4.5" height="4.5" rx=".8"/><circle cx="11.5" cy="11.5" r="2.4"/>"##),
    ("ungroup", r##"<rect x="2" y="2" width="6" height="6" rx="1"/><circle cx="12.5" cy="12.5" r="3"/><path d="M11 3h4v4M7 15H3v-4" stroke-dasharray="1.6 1.4"/>"##),
    ("sAlignL", r##"<path d="M2.5 2v14"/><rect x="5" y="4" width="10" height="3.5" rx=".8"/><rect x="5" y="10.5" width="6" height="3.5" rx=".8"/>"##),
    ("sAlignC", r##"<path d="M9 2v14"/><rect x="3" y="4" width="12" height="3.5" rx=".8"/><rect x="5.5" y="10.5" width="7" height="3.5" rx=".8"/>"##),
    ("sAlignR", r##"<path d="M15.5 2v14"/><rect x="3" y="4" width="10" height="3.5" rx=".8"/><rect x="7" y="10.5" width="6" height="3.5" rx=".8"/>"##),
    ("sAlignT", r##"<path d="M2 2.5h14"/><rect x="4" y="5" width="3.5" height="10" rx=".8"/><rect x="10.5" y="5" width="3.5" height="6" rx=".8"/>"##),
    ("sAlignM", r##"<path d="M2 9h14"/><rect x="4" y="3" width="3.5" height="12" rx=".8"/><rect x="10.5" y="5.5" width="3.5" height="7" rx=".8"/>"##),
    ("sAlignB", r##"<path d="M2 15.5h14"/><rect x="4" y="3" width="3.5" height="10" rx=".8"/><rect x="10.5" y="7" width="3.5" height="6" rx=".8"/>"##),
    ("distH", r##"<path d="M2 2v14M16 2v14"/><rect x="6.8" y="5" width="4.4" height="8" rx=".8"/>"##),
    ("distV", r##"<path d="M2 2h14M2 16h14"/><rect x="5" y="6.8" width="8" height="4.4" rx=".8"/>"##),
    ("rotate", r##"<path d="M14.8 9A5.8 5.8 0 1 1 13 4.8"/><path d="M13.6 1.8v3.6H10"/>"##),
    ("rotateL", r##"<path d="M3.2 9A5.8 5.8 0 1 0 5 4.8"/><path d="M4.4 1.8v3.6H8"/>"##),
    ("flipH", r##"<path d="M9 1.8v14.4" stroke-dasharray="1.8 1.6"/><path d="M6.5 4.5 2.5 13h4zM11.5 4.5l4 8.5h-4z"/>"##),
    ("flipV", r##"<path d="M1.8 9h14.4" stroke-dasharray="1.8 1.6"/><path d="M4.5 6.5 13 2.5v4zM4.5 11.5l8.5 4v-4z"/>"##),
    ("lock", r##"<rect x="3.5" y="8" width="11" height="8" rx="1.6"/><path d="M6 8V5.8a3 3 0 0 1 6 0V8"/>"##),
    ("page", r##"<path d="M4 1.8h6.5L14 5.3v10.9H4z"/><path d="M10.5 1.8v3.5H14M9 8.5v5M6.5 11h5"/>"##),
    ("container", r##"<rect x="1.8" y="2.5" width="14.4" height="13" rx="1.5"/><path d="M1.8 6.5h14.4"/><path d="M4 4.5h4"/>"##),
    ("boundary", r##"<rect x="1.8" y="2.5" width="14.4" height="13" rx="2.5" stroke-dasharray="2.2 1.8"/>"##),
    ("swimlane", r##"<rect x="1.8" y="2.5" width="14.4" height="13" rx="1"/><path d="M5 2.5v13M5 9h11.2"/>"##),
    ("image", r##"<rect x="2" y="3" width="14" height="12" rx="1.6"/><circle cx="6.2" cy="7" r="1.4"/><path d="m2.5 13.5 4.2-4 3 3 2-2 3.8 3.5"/>"##),
    ("callout", r##"<path d="M3 2.5h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H8l-3.5 3.3V12.5H3a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z"/>"##),
    ("layout", r##"<rect x="6.5" y="1.8" width="5" height="3.6" rx=".8"/><rect x="1.8" y="12.6" width="5" height="3.6" rx=".8"/><rect x="11.2" y="12.6" width="5" height="3.6" rx=".8"/><path d="M9 5.4V9M4.3 12.6V9h9.4v3.6"/>"##),
    ("layoutLR", r##"<rect x="1.8" y="6.5" width="3.6" height="5" rx=".8"/><rect x="12.6" y="1.8" width="3.6" height="5" rx=".8"/><rect x="12.6" y="11.2" width="3.6" height="5" rx=".8"/><path d="M5.4 9H9M12.6 4.3H9v9.4h3.6"/>"##),
    ("zoomIn", r##"<circle cx="7.8" cy="7.8" r="5.3"/><path d="m11.6 11.6 4.6 4.6M5.5 7.8h4.6M7.8 5.5v4.6"/>"##),
    ("zoomOut", r##"<circle cx="7.8" cy="7.8" r="5.3"/><path d="m11.6 11.6 4.6 4.6M5.5 7.8h4.6"/>"##),
    ("fit", r##"<path d="M2 6V2h4M12 2h4v4M16 12v4h-4M6 16H2v-4"/><rect x="5.5" y="5.5" width="7" height="7" rx="1"/>"##),
    ("actual", r##"<rect x="2" y="3" width="14" height="12" rx="1.5"/><path d="M6 7v4.5M5 7.5 6 7M11.5 7v4.5M10.5 7.5l1-.5M8.8 8.3v.1M8.8 10.4v.1"/>"##),
    ("sun", r##"<circle cx="9" cy="9" r="3.4"/><path d="M9 1.5v1.8M9 14.7v1.8M1.5 9h1.8M14.7 9h1.8M3.7 3.7l1.3 1.3M13 13l1.3 1.3M3.7 14.3 5 13M13 5l1.3-1.3"/>"##),
    ("moon", r##"<path d="M15 10.8A6.5 6.5 0 1 1 7.2 3a5.2 5.2 0 0 0 7.8 7.8z"/>"##),
    ("system", r##"<rect x="1.8" y="3" width="14.4" height="10" rx="1.5"/><path d="M6 16h6M9 13v3"/><path d="M9 3h5.7a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1-1.5 1.5H9z" fill="#fff" fill-opacity=".45"/>"##),
    ("save", r##"<path d="M3 2.5h9.5L15.5 5.5v10H3z"/><path d="M5.8 2.5v4h6v-4M5.8 15.5v-5h6.4v5"/>"##),
    ("saveAs", r##"<path d="M3 2.5h9.5L15.5 5.5v4M3 2.5v13h6"/><path d="M5.8 2.5v4h6v-4"/><path d="m11 16 .5-2 4-4 1.5 1.5-4 4z"/>"##),
    ("open", r##"<path d="M2 4.5V14a1.2 1.2 0 0 0 1.2 1.2h10.6l2.6-6.5H6L3.5 15"/><path d="M2 4.5V3.6A1.1 1.1 0 0 1 3.1 2.5h3.6l1.6 1.8h5.3a1.1 1.1 0 0 1 1.1 1.1v3.3"/>"##),
    ("newdoc", r##"<path d="M4 1.8h6.5L14 5.3v10.9H4z"/><path d="M10.5 1.8v3.5H14"/>"##),
    ("pdf", r##"<path d="M4 1.8h6.5L14 5.3v10.9H4z"/><path d="M10.5 1.8v3.5H14M6.5 9h5M6.5 11.5h5M6.5 14h3"/>"##),
    ("vector", r##"<path d="M3 15C5 7 13 11 15 3"/><rect x="1.5" y="13.5" width="3" height="3" rx=".5"/><rect x="13.5" y="1.5" width="3" height="3" rx=".5"/><path d="M3 15h5M15 3h-5" stroke-dasharray="1.4 1.2"/>"##),
    ("export", r##"<path d="M9 11.5V2M5.5 5.5 9 2l3.5 3.5"/><path d="M3 10v5a1.2 1.2 0 0 0 1.2 1.2h9.6A1.2 1.2 0 0 0 15 15v-5"/>"##),
    ("trash", r##"<path d="M2.5 4.5h13M7 4.5V2.8h4v1.7M4 4.5l.9 11.3h8.2L14 4.5M7.5 7.5v5.5M10.5 7.5v5.5"/>"##),
    ("plus", r##"<path d="M9 3v12M3 9h12"/>"##),
    ("minus", r##"<path d="M3 9h12"/>"##),
    ("search", r##"<circle cx="7.8" cy="7.8" r="5"/><path d="m11.5 11.5 4.2 4.2"/>"##),
    ("close", r##"<path d="m4.5 4.5 9 9M13.5 4.5l-9 9"/>"##),
    ("chevL", r##"<path d="M11 4 6 9l5 5"/>"##),
    ("chevR", r##"<path d="M7 4l5 5-5 5"/>"##),
    ("duplicate", r##"<rect x="2.5" y="2.5" width="9" height="9" rx="1.4"/><rect x="6.5" y="6.5" width="9" height="9" rx="1.4"/><path d="M11 9v4M9 11h4"/>"##),
    ("selectAll", r##"<rect x="2" y="2" width="14" height="14" rx="1.4" stroke-dasharray="2.2 1.8"/><rect x="5" y="5" width="8" height="8" rx="1" fill="#fff" fill-opacity=".35"/>"##),
    ("routeOrtho", r##"<path d="M2.5 15V9h13V3"/><circle cx="2.5" cy="15" r="1.2" fill="#fff"/><path d="m13.5 5 2-2 2 2"/>"##),
    ("routeStraight", r##"<path d="M2.5 15.5 15.5 2.5"/><path d="M11.2 2.5h4.3v4.3"/>"##),
    ("routeCurve", r##"<path d="M2.5 15.5C2.5 7 15.5 11 15.5 2.5"/><path d="m13.5 4.5 2-2 2 2"/>"##),
    ("sizeFit", r##"<rect x="2" y="2" width="14" height="14" rx="1.2" stroke-dasharray="2 1.6"/><rect x="5.5" y="5.5" width="7" height="7" rx="1"/><path d="M2 2l3.5 3.5M16 16l-3.5-3.5"/>"##),
    ("info", r##"<circle cx="9" cy="9" r="7"/><path d="M9 8v4.5M9 5.5v.1"/>"##),
    ("keyboard", r##"<rect x="1.8" y="4" width="14.4" height="10" rx="1.5"/><path d="M4.5 7h.1M7 7h.1M9.5 7h.1M12 7h.1M4.5 9.5h.1M13.5 9.5h.1M6.5 11.5h5"/>"##),
    ("bigger", r##"<path d="M2 15 6.5 3 11 15M3.7 10.5h5.6M14 4v5M11.5 6.5h5"/>"##),
    ("smaller", r##"<path d="M2 15 6.5 3 11 15M3.7 10.5h5.6M11.5 6.5h5"/>"##),
    ("bold", r##"<path d="M5 3h5a3 3 0 0 1 0 6H5zM5 9h6a3.2 3.2 0 0 1 0 6.4H5z" stroke-width="2"/>"##),
    ("italic", r##"<path d="M8 3h6M4 15h6M11 3 7 15"/>"##),
    ("solid", r##"<path d="M1.5 9h15" stroke-width="2"/>"##),
    ("dashed", r##"<path d="M1.5 9h3.2M7.4 9h3.2M13.3 9h3.2" stroke-width="2"/>"##),
    ("dotted", r##"<path d="M2 9h.1M6 9h.1M10 9h.1M14 9h.1" stroke-width="2.4"/>"##),
    ("gridLayout", r##"<rect x="2" y="2" width="5.5" height="5.5" rx="1.2"/><rect x="10.5" y="2" width="5.5" height="5.5" rx="1.2"/><rect x="2" y="10.5" width="5.5" height="5.5" rx="1.2"/><rect x="10.5" y="10.5" width="5.5" height="5.5" rx="1.2"/>"##),
    ("tidy", r##"<path d="M2.5 15V9h8V3.5"/><circle cx="2.5" cy="15" r="1.3" fill="#fff"/><path d="m8.5 5.5 2-2 2 2"/><path d="M14 9.5v4M12 11.5h4M15.5 2.5v2M14.5 3.5h2" stroke-width="1.2"/>"##),
    ("autoArrange", r##"<rect x="6.5" y="1.8" width="5" height="3.6" rx=".8"/><rect x="1.8" y="12.6" width="5" height="3.6" rx=".8"/><rect x="11.2" y="12.6" width="5" height="3.6" rx=".8"/><path d="M9 5.4V9M4.3 12.6V9h9.4v3.6"/><path d="M15 2v3M13.5 3.5h3" stroke-width="1.2"/>"##),
    ("data", r##"<rect x="2" y="2.5" width="14" height="13" rx="1.4"/><path d="M2 6.5h14M2 10.5h14M7 6.5v9"/>"##),
];

pub struct Icons {
    map: HashMap<&'static str, TextureHandle>,
}

impl Icons {
    pub fn load(ctx: &egui::Context) -> Self {
        let mut map = HashMap::new();
        let px = 54u32;
        for (name, inner) in ICONS {
            let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18" width="{px}" height="{px}" fill="none" stroke="#fff" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">{inner}</svg>"##);
            let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) else { continue };
            let Some(mut pm) = resvg::tiny_skia::Pixmap::new(px, px) else { continue };
            resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pm.as_mut());
            // tiny-skia is premultiplied; egui wants premultiplied too
            let img = ColorImage::from_rgba_premultiplied([px as usize, px as usize], pm.data());
            map.insert(*name, ctx.load_texture(format!("icon-{name}"), img, TextureOptions::LINEAR));
        }
        Icons { map }
    }

    pub fn paint(&self, p: &Painter, name: &str, rect: Rect, tint: Color32) {
        if let Some(t) = self.map.get(name) {
            p.image(t.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
        }
    }

    pub fn has(&self, name: &str) -> bool { self.map.contains_key(name) }
}

static GLOBAL: std::sync::OnceLock<Icons> = std::sync::OnceLock::new();
pub fn init(ctx: &egui::Context) { let _ = GLOBAL.set(Icons::load(ctx)); }
pub fn get() -> &'static Icons { GLOBAL.get().expect("icons not initialised") }
