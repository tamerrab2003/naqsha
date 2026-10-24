//! Arabic and other right-to-left text on the canvas.
//!
//! egui lays text out one character after another, left to right, with no shaping. That leaves
//! Arabic unjoined and backwards. Before egui sees such text we:
//!   1. shape it: each Arabic letter becomes its isolated, initial, medial or final presentation
//!      form (Unicode Arabic Presentation Forms A/B), lam + alef become one ligature;
//!   2. let egui wrap it (widths don't depend on order), then
//!   3. put each row in visual order with the Unicode bidi algorithm (mirroring brackets).
//!
//! The file keeps the text as typed. The SVG / PNG / PDF exporters write the typed (logical) text
//! back with its direction and let resvg shape it, so exports are real, selectable Arabic:
//! [`logical`] maps a visual row back to what was typed.

use std::cell::RefCell;
use std::collections::HashMap;
use unicode_bidi::{BidiClass, Level, ParagraphBidiInfo};

/// True when the text has a right-to-left letter (Arabic, Hebrew, Syriac, Thaana, N'Ko …).
pub fn has_rtl(text: &str) -> bool {
    text.chars().any(|c| (c as u32) >= 0x0590 && matches!(unicode_bidi::bidi_class(c), BidiClass::R | BidiClass::AL))
}

/// Direction of a paragraph from its first strong letter (Unicode rules P2/P3).
pub fn is_rtl_paragraph(text: &str) -> bool {
    for c in text.chars() {
        match unicode_bidi::bidi_class(c) {
            BidiClass::L => return false,
            BidiClass::R | BidiClass::AL => return true,
            _ => {}
        }
    }
    false
}

// ---- shaping ---------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Join { Dual, Right, Causing, Transparent, None }

/// (letter, isolated, final, initial, medial); 0 = no such form (a right-joining letter).
const FORMS: &[(char, u32, u32, u32, u32)] = &[
    ('\u{0621}', 0xFE80, 0, 0, 0),
    ('\u{0622}', 0xFE81, 0xFE82, 0, 0),
    ('\u{0623}', 0xFE83, 0xFE84, 0, 0),
    ('\u{0624}', 0xFE85, 0xFE86, 0, 0),
    ('\u{0625}', 0xFE87, 0xFE88, 0, 0),
    ('\u{0626}', 0xFE89, 0xFE8A, 0xFE8B, 0xFE8C),
    ('\u{0627}', 0xFE8D, 0xFE8E, 0, 0),
    ('\u{0628}', 0xFE8F, 0xFE90, 0xFE91, 0xFE92),
    ('\u{0629}', 0xFE93, 0xFE94, 0, 0),
    ('\u{062A}', 0xFE95, 0xFE96, 0xFE97, 0xFE98),
    ('\u{062B}', 0xFE99, 0xFE9A, 0xFE9B, 0xFE9C),
    ('\u{062C}', 0xFE9D, 0xFE9E, 0xFE9F, 0xFEA0),
    ('\u{062D}', 0xFEA1, 0xFEA2, 0xFEA3, 0xFEA4),
    ('\u{062E}', 0xFEA5, 0xFEA6, 0xFEA7, 0xFEA8),
    ('\u{062F}', 0xFEA9, 0xFEAA, 0, 0),
    ('\u{0630}', 0xFEAB, 0xFEAC, 0, 0),
    ('\u{0631}', 0xFEAD, 0xFEAE, 0, 0),
    ('\u{0632}', 0xFEAF, 0xFEB0, 0, 0),
    ('\u{0633}', 0xFEB1, 0xFEB2, 0xFEB3, 0xFEB4),
    ('\u{0634}', 0xFEB5, 0xFEB6, 0xFEB7, 0xFEB8),
    ('\u{0635}', 0xFEB9, 0xFEBA, 0xFEBB, 0xFEBC),
    ('\u{0636}', 0xFEBD, 0xFEBE, 0xFEBF, 0xFEC0),
    ('\u{0637}', 0xFEC1, 0xFEC2, 0xFEC3, 0xFEC4),
    ('\u{0638}', 0xFEC5, 0xFEC6, 0xFEC7, 0xFEC8),
    ('\u{0639}', 0xFEC9, 0xFECA, 0xFECB, 0xFECC),
    ('\u{063A}', 0xFECD, 0xFECE, 0xFECF, 0xFED0),
    ('\u{0641}', 0xFED1, 0xFED2, 0xFED3, 0xFED4),
    ('\u{0642}', 0xFED5, 0xFED6, 0xFED7, 0xFED8),
    ('\u{0643}', 0xFED9, 0xFEDA, 0xFEDB, 0xFEDC),
    ('\u{0644}', 0xFEDD, 0xFEDE, 0xFEDF, 0xFEE0),
    ('\u{0645}', 0xFEE1, 0xFEE2, 0xFEE3, 0xFEE4),
    ('\u{0646}', 0xFEE5, 0xFEE6, 0xFEE7, 0xFEE8),
    ('\u{0647}', 0xFEE9, 0xFEEA, 0xFEEB, 0xFEEC),
    ('\u{0648}', 0xFEED, 0xFEEE, 0, 0),
    ('\u{0649}', 0xFEEF, 0xFEF0, 0, 0),
    ('\u{064A}', 0xFEF1, 0xFEF2, 0xFEF3, 0xFEF4),
    // Persian and Urdu letters (Presentation Forms-A)
    ('\u{0679}', 0xFB66, 0xFB67, 0xFB68, 0xFB69),
    ('\u{067E}', 0xFB56, 0xFB57, 0xFB58, 0xFB59),
    ('\u{0686}', 0xFB7A, 0xFB7B, 0xFB7C, 0xFB7D),
    ('\u{0688}', 0xFB88, 0xFB89, 0, 0),
    ('\u{0691}', 0xFB8C, 0xFB8D, 0, 0),
    ('\u{0698}', 0xFB8A, 0xFB8B, 0, 0),
    ('\u{06A9}', 0xFB8E, 0xFB8F, 0xFB90, 0xFB91),
    ('\u{06AF}', 0xFB92, 0xFB93, 0xFB94, 0xFB95),
    ('\u{06BA}', 0xFB9E, 0xFB9F, 0, 0),
    ('\u{06C1}', 0xFBA6, 0xFBA7, 0xFBA8, 0xFBA9),
    ('\u{06CC}', 0xFBFC, 0xFBFD, 0xFBFE, 0xFBFF),
    ('\u{06D2}', 0xFBAE, 0xFBAF, 0, 0),
];

fn forms(c: char) -> Option<&'static (char, u32, u32, u32, u32)> { FORMS.iter().find(|f| f.0 == c) }

fn joining(c: char) -> Join {
    let u = c as u32;
    if c == '\u{0640}' || c == '\u{200D}' { return Join::Causing; } // tatweel, zero-width joiner
    if (0x064B..=0x065F).contains(&u) || u == 0x0670 || (0x06D6..=0x06ED).contains(&u) { return Join::Transparent; }
    match forms(c) { Some(f) if f.3 != 0 => Join::Dual, Some(f) if f.2 != 0 => Join::Right, _ => Join::None }
}

/// Lam + this alef as one ligature: (isolated, final).
fn lam_alef(alef: char) -> Option<(u32, u32)> {
    match alef { '\u{0622}' => Some((0xFEF5, 0xFEF6)), '\u{0623}' => Some((0xFEF7, 0xFEF8)), '\u{0625}' => Some((0xFEF9, 0xFEFA)), '\u{0627}' => Some((0xFEFB, 0xFEFC)), _ => None }
}

/// Shape Arabic letters into presentation forms (logical order kept). Also returns, for every
/// output character, the byte range of the input it came from (a lam-alef covers two letters).
pub fn shape(text: &str) -> (String, Vec<std::ops::Range<usize>>) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let end_of = |i: usize| chars.get(i + 1).map(|c| c.0).unwrap_or(text.len());
    // the nearest non-mark neighbour on each side
    let prev = |i: usize| chars[..i].iter().rev().map(|c| c.1).find(|c| joining(*c) != Join::Transparent);
    let next_idx = |i: usize| (i + 1..chars.len()).find(|&j| joining(chars[j].1) != Join::Transparent);
    let mut out = String::with_capacity(text.len());
    let mut map = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let (b, c) = chars[i];
        let j = joining(c);
        if !matches!(j, Join::Dual | Join::Right) { out.push(c); map.push(b..end_of(i)); i += 1; continue; }
        let f = forms(c).unwrap();
        let joins_prev = matches!(prev(i).map(joining), Some(Join::Dual | Join::Causing));
        let nj = next_idx(i);
        // lam followed by an alef: one ligature
        if c == '\u{0644}' {
            if let Some(n) = nj.filter(|&n| n == i + 1) {
                if let Some((iso, fin)) = lam_alef(chars[n].1) {
                    out.push(char::from_u32(if joins_prev { fin } else { iso }).unwrap());
                    map.push(b..end_of(n));
                    i = n + 1;
                    continue;
                }
            }
        }
        let joins_next = j == Join::Dual && matches!(nj.map(|n| joining(chars[n].1)), Some(Join::Dual | Join::Right | Join::Causing));
        let form = match (joins_prev, joins_next) { (true, true) => f.4, (true, false) => f.2, (false, true) => f.3, _ => f.1 };
        out.push(char::from_u32(form).unwrap_or(c));
        map.push(b..end_of(i));
        i += 1;
    }
    (out, map)
}

// ---- visual order ------------------------------------------------------------------------------

/// One line in visual (left-to-right display) order, from shaped logical text.
pub fn visual_line(line: &str, rtl_base: bool) -> String {
    if line.is_empty() { return String::new(); }
    let para = ParagraphBidiInfo::new(line, Some(if rtl_base { Level::rtl() } else { Level::ltr() }));
    let levels = para.reordered_levels_per_char(0..line.len());
    let chars: Vec<char> = line.chars().collect();
    let order = ParagraphBidiInfo::reorder_visual(&levels);
    let mut v: Vec<char> = order.iter().map(|&k| {
        let c = chars[k];
        if levels[k].is_rtl() { unicode_bidi_mirroring::get_mirrored(c).unwrap_or(c) } else { c }
    }).collect();
    // marks came out before their letter; put each run of marks back after it, so it draws on top
    let mut k = 0;
    while k < v.len() {
        if joining(v[k]) == Join::Transparent {
            let s = k;
            while k < v.len() && joining(v[k]) == Join::Transparent { k += 1; }
            if k < v.len() { v[s..=k].rotate_right(1); }
        }
        k += 1;
    }
    v.into_iter().collect()
}

/// Right-to-left script characters (Hebrew, Arabic, Syriac, Thaana, N'Ko and their presentation forms).
fn rtl_script(c: char) -> bool { matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF) }

/// Append text to a layout job so egui draws it in the order given. egui shapes each run with
/// HarfBuzz and, seeing Arabic letters, would run the whole run right to left: Latin words and
/// digits next to Arabic come out backwards, and the already-ordered visual text reversed again.
/// Each right-to-left character gets its own section (so its own one-glyph run); everything
/// else goes in as it is.
pub fn append(job: &mut egui::text::LayoutJob, text: &str, fmt: egui::TextFormat) {
    crate::scripts::note(text);
    if !text.chars().any(rtl_script) { job.append(text, 0.0, fmt); return; }
    // (pushed directly: LayoutJob::append merges neighbouring sections of the same format)
    let push = |job: &mut egui::text::LayoutJob, t: &str| {
        let a = job.text.len();
        job.text.push_str(t);
        job.sections.push(egui::text::LayoutSection { leading_space: 0.0, byte_range: egui::text::ByteIndex(a)..egui::text::ByteIndex(job.text.len()), format: fmt.clone() });
    };
    // a letter and the marks after it (harakat) stay together, so the marks sit on it
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut start = 0;
    let mut k = 0;
    while k < chars.len() {
        let (i, c) = chars[k];
        k += 1;
        if !rtl_script(c) { continue; }
        if start < i { push(job, &text[start..i]); }
        while k < chars.len() && joining(chars[k].1) == Join::Transparent { k += 1; }
        let e = chars.get(k).map(|x| x.0).unwrap_or(text.len());
        push(job, &text[i..e]);
        start = e;
    }
    if start < text.len() { push(job, &text[start..]); }
}

/// A layout job that draws `text` (as typed) correctly when it holds right-to-left letters:
/// shaped, wrapped at `wrap` (with `lay`, which lays a job out), each row in visual order.
/// Rows are remembered so exports can write the typed text.
pub fn visual_job(text: &str, fmt: egui::TextFormat, wrap: f32, halign: egui::Align, lay: &mut dyn FnMut(egui::text::LayoutJob) -> std::sync::Arc<egui::Galley>) -> egui::text::LayoutJob {
    use egui::text::LayoutJob;
    let (shaped, map) = shape(text);
    let mut wj = LayoutJob::default();
    wj.wrap.max_width = wrap;
    append(&mut wj, &shaped, fmt.clone());
    let wrapped = lay(wj);
    let sc: Vec<char> = shaped.chars().collect();
    let mut rows: Vec<String> = vec![];
    let mut k = 0; // index into the shaped characters
    let mut para_start = 0;
    for row in &wrapped.rows {
        let n = row.glyphs.len();
        let (a, b) = (k.min(sc.len()), (k + n).min(sc.len()));
        // the paragraph this row belongs to decides its base direction
        let para_end = sc[para_start..].iter().position(|c| *c == '\n').map(|q| para_start + q).unwrap_or(sc.len());
        let para: String = sc[para_start..para_end].iter().collect();
        let rtl_base = is_rtl_paragraph(&para);
        let line: String = sc[a..b].iter().collect();
        let trimmed = line.trim_end();
        let vis = visual_line(trimmed, rtl_base);
        if b > a && !trimmed.is_empty() {
            let last = a + trimmed.chars().count() - 1;
            remember(&vis, &text[map[a].start..map[last].end], rtl_base);
        }
        rows.push(vis);
        k += n;
        if row.ends_with_newline { k += 1; para_start = k.min(sc.len()); }
    }
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    job.halign = halign;
    append(&mut job, &rows.join("\n"), fmt);
    job
}

thread_local! {
    /// visual row → (typed text of that row, right-to-left)
    static ROWS: RefCell<HashMap<String, (String, bool)>> = RefCell::new(HashMap::new());
}

pub fn remember(visual: &str, logical: &str, rtl: bool) {
    ROWS.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 20_000 { m.clear(); }
        m.entry(visual.to_string()).or_insert_with(|| (logical.to_string(), rtl));
    });
}

/// The typed text and direction behind a row laid out by [`crate::render::layout`].
pub fn logical(visual: &str) -> Option<(String, bool)> { ROWS.with(|m| m.borrow().get(visual).cloned()) }

#[cfg(test)]
mod tests {
    use super::*;

    fn vis(s: &str) -> String { let (sh, _) = shape(s); visual_line(&sh, is_rtl_paragraph(s)) }

    #[test]
    fn joins_letters() {
        // سلام: seen initial, lam-alef final, meem isolated; read right to left
        assert_eq!(shape("سلام").0, "\u{FEB3}\u{FEFC}\u{FEE1}");
        // محطة ضخ: meem initial, hah medial, tah medial, teh marbuta final
        assert_eq!(shape("محطة").0, "\u{FEE3}\u{FEA4}\u{FEC4}\u{FE94}");
        assert_eq!(shape("لا").0, "\u{FEFB}");
        assert_eq!(shape("abc").0, "abc");
    }

    #[test]
    fn visual_order() {
        assert_eq!(vis("سلام"), "\u{FEE1}\u{FEFC}\u{FEB3}");
        // a Latin tag inside Arabic keeps its own order, and sits on the left
        let v = vis("مضخة P-101");
        assert!(v.starts_with("P-101 "), "{v}");
        // brackets mirror in right-to-left runs
        assert!(vis("(مضخة)").starts_with('('));
        // Latin text untouched
        assert_eq!(vis("Pump (P-101)"), "Pump (P-101)");
    }

    #[test]
    fn marks_follow_letters() {
        let v = vis("بَ");
        assert_eq!(v.chars().count(), 2);
        assert!(joining(v.chars().nth(1).unwrap()) == Join::Transparent);
    }

    #[test]
    fn galley_rows() {
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::theme::fonts());
        for _ in 0..2 {
            let _ = ctx.run_ui(Default::default(), |ui| {
                let g = crate::render::layout(ui.painter(), "العميل", 13.0, false, false, egui::Color32::BLACK, 100.0, egui::Align::Center);
                let rows: Vec<String> = g.rows.iter().map(|r| r.glyphs.iter().map(|g| g.chr).collect()).collect();
                let rows: Vec<&str> = rows.iter().map(|r| r.as_str()).collect();
                assert_eq!(rows, vec!["\u{FEDE}\u{FEF4}\u{FEE4}\u{FECC}\u{FEDF}\u{FE8D}"]);
                assert_eq!(logical(rows[0]), Some(("العميل".to_string(), true)));
            });
        }
    }
}
