//! Fonts for writing systems beyond Latin, Greek, Cyrillic and Arabic (plus the Urdu letters
//! DejaVu Sans lacks): Devanagari (Hindi, Marathi,
//! Nepali), Thai, and Chinese / Japanese (Noto Sans CJK SC, cut down to everyday characters:
//! GB 2312, Big5 level 1, JIS level 1, kana and CJK punctuation; see web/fonts/README.md).
//!
//! egui shapes these itself (HarfBuzz); it only needs a font. Devanagari and Thai are small and
//! built in everywhere. The Chinese/Japanese font (4 MB) is built into the desktop apps; the
//! browser version downloads it the first time a drawing shows such text.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x9FFF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF | 0x20000..=0x2FA1F)
}

pub fn is_thai(c: char) -> bool { matches!(c as u32, 0x0E00..=0x0E7F) }

const DEVANAGARI: &[u8] = include_bytes!("../web/fonts/NotoSansDevanagari-Regular.ttf");
const DEVANAGARI_BOLD: &[u8] = include_bytes!("../web/fonts/NotoSansDevanagari-Bold.ttf");
const THAI: &[u8] = include_bytes!("../web/fonts/NotoSansThai-Regular.ttf");
const THAI_BOLD: &[u8] = include_bytes!("../web/fonts/NotoSansThai-Bold.ttf");
/// The Arabic-script letters DejaVu Sans lacks (Urdu ہ ۂ ے ۓ ۔ and others), from Noto Sans Arabic.
const URDU: &[u8] = include_bytes!("../web/fonts/NotoSansArabic-Urdu-Regular.ttf");
const URDU_BOLD: &[u8] = include_bytes!("../web/fonts/NotoSansArabic-Urdu-Bold.ttf");
#[cfg(not(target_arch = "wasm32"))]
const CJK: &[u8] = include_bytes!("../web/fonts/NotoSansCJK-sub.otf");

/// Add the script fonts after the fonts already there (they are only used for their scripts).
pub fn add_to(f: &mut FontDefinitions) {
    let mut add = |name: &str, bytes: &'static [u8], family: FontFamily| {
        f.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
        f.families.entry(family).or_default().push(name.into());
    };
    add("devanagari", DEVANAGARI, FontFamily::Proportional);
    add("thai", THAI, FontFamily::Proportional);
    add("urdu", URDU, FontFamily::Proportional);
    add("devanagari-bold", DEVANAGARI_BOLD, FontFamily::Name("bold".into()));
    add("thai-bold", THAI_BOLD, FontFamily::Name("bold".into()));
    add("urdu-bold", URDU_BOLD, FontFamily::Name("bold".into()));
    #[cfg(not(target_arch = "wasm32"))]
    {
        add("cjk", CJK, FontFamily::Proportional);
        add("cjk-b", CJK, FontFamily::Name("bold".into()));
    }
}

/// Fonts for exporting (resvg): the same ones, where they aren't on the system.
pub fn add_to_db(db: &mut svg2pdf::usvg::fontdb::Database) {
    for b in [DEVANAGARI, DEVANAGARI_BOLD, THAI, THAI_BOLD, URDU, URDU_BOLD] { db.load_font_data(b.to_vec()); }
    if let Some(b) = cjk() { db.load_font_data(b.to_vec()); }
}

#[cfg(not(target_arch = "wasm32"))]
fn cjk() -> Option<&'static [u8]> { Some(CJK) }

/// Text is about to be drawn: in the browser, fetch the Chinese/Japanese font the first time it
/// is needed. (The desktop apps have it built in.)
pub fn note(text: &str) {
    #[cfg(target_arch = "wasm32")]
    if text.chars().any(is_cjk) { web::fetch_cjk(); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

#[cfg(target_arch = "wasm32")]
fn cjk() -> Option<&'static [u8]> { web::CJK.get().map(|v| v.as_slice()) }

#[cfg(target_arch = "wasm32")]
mod web {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    pub static CJK: OnceLock<Vec<u8>> = OnceLock::new();
    static STARTED: AtomicBool = AtomicBool::new(false);
    /// next to index.html, so the app works from any address
    const URL: &str = "fonts/NotoSansCJK-sub.otf";

    pub fn fetch_cjk() {
        if STARTED.swap(true, Ordering::Relaxed) { return; }
        wasm_bindgen_futures::spawn_local(async {
            let Some(bytes) = get().await else { STARTED.store(false, Ordering::Relaxed); return };
            let _ = CJK.set(bytes.clone());
            if let Some(ctx) = crate::platform::CTX.get() {
                use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
                ctx.add_font(FontInsert::new("cjk", egui::FontData::from_owned(bytes), vec![
                    InsertFontFamily { family: egui::FontFamily::Proportional, priority: FontPriority::Lowest },
                    InsertFontFamily { family: egui::FontFamily::Name("bold".into()), priority: FontPriority::Lowest },
                ]));
                ctx.request_repaint();
            }
        });
    }

    async fn get() -> Option<Vec<u8>> {
        let win = web_sys::window()?;
        let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(URL)).await.ok()?.dyn_into().ok()?;
        if !resp.ok() { return None; }
        let buf = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
        Some(js_sys::Uint8Array::new(&buf).to_vec())
    }
}
