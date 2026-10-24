//! The app in other languages.
//!
//! Every piece of interface text is written in English in the code and looked up at run time:
//! `tr("Save")` gives the translation in the chosen language (or the English when there is
//! none). The translations live in `locales/<code>.json`, one `{"English": "translation"}`
//! object per language, built into the app. `tools/i18n.py` lists the English strings.
//!
//! Arabic, Persian and Urdu need more than a translation: egui doesn't order mixed
//! right-to-left text (see rtl.rs), so the widget text helpers here ([`t`], [`w`], [`text`],
//! [`galley`]) lay such text out in visual order first. And the whole window is mirrored
//! ([`rtl`]).

use egui::text::LayoutJob;
use egui::{Align, Align2, Color32, FontId, Galley, Painter, Pos2, TextFormat, WidgetText};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

/// (code, name in its own language, translations)
pub const LANGS: &[(&str, &str, &str)] = &[
    ("en", "English", "{}"),
    ("ar", "العربية", include_str!("../locales/ar.json")),
    ("fr", "Français", include_str!("../locales/fr.json")),
    ("es", "Español", include_str!("../locales/es.json")),
    ("pt", "Português", include_str!("../locales/pt.json")),
    ("hi", "हिन्दी", include_str!("../locales/hi.json")),
    ("fa", "فارسی", include_str!("../locales/fa.json")),
    ("ur", "اردو", include_str!("../locales/ur.json")),
    ("ms", "Bahasa Melayu", include_str!("../locales/ms.json")),
    ("id", "Bahasa Indonesia", include_str!("../locales/id.json")),
    ("th", "ไทย", include_str!("../locales/th.json")),
    ("vi", "Tiếng Việt", include_str!("../locales/vi.json")),
    ("zh-CN", "简体中文", include_str!("../locales/zh-CN.json")),
    ("zh-TW", "繁體中文", include_str!("../locales/zh-TW.json")),
    ("ja", "日本語", include_str!("../locales/ja.json")),
    ("fil", "Filipino", include_str!("../locales/fil.json")),
    ("sw", "Kiswahili", include_str!("../locales/sw.json")),
];

const RTL_LANGS: &[&str] = &["ar", "fa", "ur"];

static CUR: AtomicUsize = AtomicUsize::new(0);
static TABLE: OnceLock<RwLock<HashMap<&'static str, &'static str>>> = OnceLock::new();

fn table() -> &'static RwLock<HashMap<&'static str, &'static str>> { TABLE.get_or_init(|| RwLock::new(HashMap::new())) }

/// The language in use (its code).
pub fn current() -> &'static str { LANGS[CUR.load(Ordering::Relaxed)].0 }

/// Is the interface right to left (Arabic, Persian, Urdu)?
pub fn rtl() -> bool { RTL_LANGS.contains(&current()) }

/// Switch language: a code from [`LANGS`], or "" / unknown for the system's language.
pub fn set(code: &str) {
    let code = if code.is_empty() { system() } else { code.to_string() };
    let i = LANGS.iter().position(|l| l.0 == code).unwrap_or(0);
    if i == CUR.load(Ordering::Relaxed) && i != 0 && !table().read().unwrap().is_empty() { return; }
    let map: HashMap<String, String> = serde_json::from_str(LANGS[i].2).unwrap_or_default();
    // leaked once per switch: a few hundred kB at most, and switching is rare
    let map: HashMap<&'static str, &'static str> = map.into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (&*Box::leak(k.into_boxed_str()), &*Box::leak(v.into_boxed_str()))).collect();
    *table().write().unwrap() = map;
    CUR.store(i, Ordering::Relaxed);
    // the language's script may need a font that loads on demand (Chinese, Japanese in the browser)
    crate::scripts::note(LANGS[i].1);
}

/// The system's (or browser's) language, as one of ours.
pub fn system() -> String {
    let loc = sys_locale::get_locale().unwrap_or_default().replace('_', "-");
    pick(&loc).to_string()
}

/// One of our languages for a locale name such as "fr-CA", "zh-Hant-TW" or "tl".
pub fn pick(loc: &str) -> &'static str {
    let l = loc.to_ascii_lowercase();
    let base = l.split('-').next().unwrap_or("");
    match base {
        "zh" => if l.contains("tw") || l.contains("hk") || l.contains("mo") || l.contains("hant") { "zh-TW" } else { "zh-CN" },
        "tl" | "fil" => "fil",
        "in" => "id",
        "pt" => "pt",
        _ => LANGS.iter().find(|x| x.0 == base).map(|x| x.0).unwrap_or("en"),
    }
}

/// The translation of an interface string (the English itself when there's none).
pub fn tr(s: &str) -> &str {
    if CUR.load(Ordering::Relaxed) == 0 { return s; }
    let found = table().read().unwrap().get(s).copied();
    found.unwrap_or(s)
}

/// [`tr`] for text with `{name}` placeholders, filled from `args`.
pub fn trf(s: &str, args: &[(&str, &str)]) -> String {
    let mut out = tr(s).to_string();
    for (k, v) in args { out = out.replace(&format!("{{{k}}}"), v); }
    out
}

// ---- the window, mirrored for right-to-left languages ------------------------------------------

/// The layout for a row of controls: left to right, or right to left in Arabic, Persian, Urdu.
pub fn row_layout(align: Align) -> egui::Layout {
    if rtl() { egui::Layout::right_to_left(align) } else { egui::Layout::left_to_right(align) }
}

/// The opposite end of a row (where "the rest" goes: close buttons, zoom).
pub fn end_layout(align: Align) -> egui::Layout {
    if rtl() { egui::Layout::left_to_right(align) } else { egui::Layout::right_to_left(align) }
}

/// `ui.vertical`, right-aligned in right-to-left languages (rows inside it then run right to left).
pub fn vertical<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    // right-aligned only where the surrounding layout is mirrored: inside a left-to-right row a
    // right-aligned column would claim the rest of the row
    let right = rtl() && ui.layout().prefer_right_to_left();
    ui.scope_builder(egui::UiBuilder::new().layout(egui::Layout::top_down(if right { Align::Max } else { Align::Min })), add)
}

/// Run `add` mirrored for right-to-left languages: right-aligned, and every `ui.horizontal` in it
/// runs right to left. (Takes the whole available width.)
pub fn mirrored<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    if rtl() { ui.with_layout(egui::Layout::top_down(Align::Max), add).inner } else { add(ui) }
}

/// A side panel at the start of the window (left, or right in right-to-left languages).
pub fn start_panel(id: &'static str) -> egui::Panel { if rtl() { egui::Panel::right(id) } else { egui::Panel::left(id) } }

/// A side panel at the end of the window.
pub fn end_panel(id: &'static str) -> egui::Panel { if rtl() { egui::Panel::left(id) } else { egui::Panel::right(id) } }

// ---- text for widgets and painters, right to left where needed --------------------------------

/// Interface text for a widget: translated, and ready to draw.
pub fn t(s: &str) -> WidgetText { w(tr(s)) }

/// Text for a widget, not translated (names, numbers, text built with [`trf`]).
pub fn w(s: impl Into<String>) -> WidgetText {
    let s: String = s.into();
    if !crate::rtl::has_rtl(&s) { return WidgetText::from(s); }
    laid(job(&s, FontId::proportional(12.5), Color32::PLACEHOLDER, 420.0, Align::LEFT))
}

/// A finished galley, so the widget doesn't wrap the visual-order rows again.
fn laid(j: LayoutJob) -> WidgetText {
    match crate::platform::CTX.get() {
        Some(ctx) => WidgetText::Galley(ctx.fonts_mut(|f| f.layout_job(j))),
        None => WidgetText::from(j),
    }
}

/// [`w`] with a size and colour (in place of `RichText::new(..).size(..).color(..)`).
pub fn rich(s: impl Into<String>, size: f32, color: Option<Color32>, strong: bool) -> WidgetText {
    let s: String = s.into();
    if !crate::rtl::has_rtl(&s) {
        let mut r = egui::RichText::new(s).size(size);
        if let Some(c) = color { r = r.color(c); }
        if strong { r = r.strong(); }
        return r.into();
    }
    let font = if strong { crate::render::font(size, true) } else { FontId::proportional(size) };
    laid(job(&s, font, color.unwrap_or(Color32::PLACEHOLDER), 420.0, Align::LEFT))
}

fn job(s: &str, font: FontId, color: Color32, wrap: f32, halign: Align) -> LayoutJob {
    let fmt = TextFormat { font_id: font, color, ..Default::default() };
    match crate::platform::CTX.get() {
        Some(ctx) => crate::rtl::visual_job(s, fmt, wrap, halign, &mut |j| ctx.fonts_mut(|f| f.layout_job(j))),
        None => { let mut j = LayoutJob::default(); j.append(s, 0.0, fmt); j }
    }
}

/// A galley for painting text (not translated: pass `tr(..)` for interface text).
pub fn galley(p: &Painter, s: &str, font: FontId, color: Color32) -> Arc<Galley> {
    if !crate::rtl::has_rtl(s) { return p.layout_no_wrap(s.to_string(), font, color); }
    let fmt = TextFormat { font_id: font, color, ..Default::default() };
    let j = crate::rtl::visual_job(s, fmt, f32::INFINITY, Align::LEFT, &mut |j| p.layout_job(j));
    p.layout_job(j)
}

/// A galley wrapped at `width`, right to left where needed.
pub fn wrapped(p: &Painter, s: &str, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    if !crate::rtl::has_rtl(s) { return p.layout(s.to_string(), font, color, width); }
    let fmt = TextFormat { font_id: font, color, ..Default::default() };
    let j = crate::rtl::visual_job(s, fmt, width, Align::LEFT, &mut |j| p.layout_job(j));
    p.layout_job(j)
}

/// `Painter::text`, right to left where needed.
pub fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, color: Color32) -> egui::Rect {
    let g = galley(p, s, font, color);
    let rect = anchor.anchor_size(pos, g.size());
    p.galley(rect.min, g, color);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_languages() {
        assert_eq!(pick("fr-CA"), "fr");
        assert_eq!(pick("zh-Hant-TW"), "zh-TW");
        assert_eq!(pick("zh-CN"), "zh-CN");
        assert_eq!(pick("tl-PH"), "fil");
        assert_eq!(pick("de-DE"), "en");
        assert_eq!(pick("ar-EG"), "ar");
    }

    #[test]
    fn locale_files_parse() {
        for (code, _, json) in LANGS {
            let m: Result<HashMap<String, String>, _> = serde_json::from_str(json);
            assert!(m.is_ok(), "locales/{code}.json: {:?}", m.err());
            // placeholders must survive translation
            for (k, v) in m.unwrap() {
                for p in k.split('{').skip(1).filter_map(|x| x.split('}').next()) {
                    assert!(v.is_empty() || v.contains(&format!("{{{p}}}")), "{code}: \"{v}\" lost {{{p}}} of \"{k}\"");
                }
            }
        }
    }
}
