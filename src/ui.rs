//! Chrome: title bar, ribbon, shapes panel, format pane, status bar, menus, dialogs, shortcuts.
#[allow(unused_imports)]
use crate::i18n::{t, tr, trf, w};
use crate::app::*;
use crate::canvas;
use crate::export;
use crate::geom;
use crate::library::{self, CATEGORIES, DEFS};
use crate::model::*;
use crate::render::{self, with_alpha, Ctx, View};
use crate::theme::{self, from_c32, to_c32, Pal};
use crate::platform;
use egui::{pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Id as EId, Key, Layout, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui};
use std::collections::HashMap;

// ------------------------------------------------------------------ widgets
pub fn paint_icon(ui: &Ui, name: &str, rect: Rect, tint: Color32) {
    let ic = crate::icons::get();
    if ic.has(name) { ic.paint(ui.painter(), name, rect, tint); } else { ui.painter().text(rect.center(), Align2::CENTER_CENTER, name, FontId::proportional(rect.height() * 0.8), tint); }
}

/// Toolbar button: optional icon + optional label, painted to match the Nixpx look.
pub(crate) fn tb(ui: &mut Ui, pal: &Pal, icon: &str, label: &str, tip: &str, on: bool) -> Response {
    let fg = if on { pal.accent } else { pal.text };
    let g = if label.is_empty() { None } else { Some(crate::i18n::galley(ui.painter(), tr(label), FontId::proportional(12.5), fg)) };
    let has_icon = !icon.is_empty();
    let w = (8.0 + if has_icon { 18.0 } else { 0.0 } + g.as_ref().map_or(0.0, |g| g.size().x + if has_icon { 6.0 } else { 0.0 }) + 8.0).max(28.0);
    let (rect, r) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
    let enabled = ui.is_enabled();
    if on { ui.painter().rect_filled(rect, 6.0, pal.accent_soft); } else if r.hovered() && enabled { ui.painter().rect_filled(rect, 6.0, pal.hover); }
    let mut x = rect.min.x + if g.is_none() { (rect.width() - 18.0) / 2.0 } else { 8.0 };
    let dim = |c: Color32| if enabled { c } else { render::with_alpha(c, 0.35) };
    // right to left: the label first, the icon after it on the right
    let rtl = crate::i18n::rtl() && g.is_some();
    if rtl { if let Some(g) = &g { let h = g.size().y; ui.painter().galley(pos2(x, rect.center().y - h / 2.0), g.clone(), dim(fg)); x += g.size().x + 6.0; } }
    if has_icon {
        paint_icon(ui, icon, Rect::from_min_size(pos2(x, rect.center().y - 9.0), vec2(18.0, 18.0)), dim(if on || (r.hovered() && enabled) { pal.accent } else { pal.muted }));
        x += 24.0;
    }
    if !rtl { if let Some(g) = g { let h = g.size().y; ui.painter().galley(pos2(x, rect.center().y - h / 2.0), g, dim(fg)); } }
    tip_text(r, tip)
}

/// A tooltip: translated, with the shortcut keys of this platform.
pub(crate) fn tip_text(r: Response, tip: &str) -> Response {
    if tip.is_empty() { r } else { r.on_hover_text(w(platform::keys(tr(tip)))) }
}

pub(crate) fn big(ui: &mut Ui, pal: &Pal, icon: &str, label: &str, tip: &str) -> Response {
    let label = tr(label);
    let tw = crate::i18n::galley(ui.painter(), label, FontId::proportional(11.5), pal.text).size().x;
    let (rect, r) = ui.allocate_exact_size(vec2((tw + 14.0).max(56.0), 62.0), Sense::click());
    if r.hovered() { ui.painter().rect_filled(rect, 7.0, pal.hover); }
    paint_icon(ui, icon, Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(24.0, 24.0)), if r.hovered() { pal.accent } else { pal.muted });
    crate::i18n::text(ui.painter(), rect.center_bottom() - vec2(0.0, 13.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), pal.text);
    tip_text(r, tip)
}

pub(crate) fn group(ui: &mut Ui, pal: &Pal, name: &str, add: impl FnOnce(&mut Ui)) {
    crate::i18n::vertical(ui, |ui| {
        let r = ui.horizontal(|ui| { ui.set_min_height(62.0); ui.spacing_mut().item_spacing = vec2(2.0, 2.0); add(ui); }).response;
        let name = tr(name);
        let lw = crate::i18n::galley(ui.painter(), name, FontId::proportional(10.5), pal.faint).size().x;
        ui.allocate_ui_with_layout(vec2(r.rect.width().max(lw + 4.0), 14.0), Layout::top_down(Align::Center), |ui| { ui.add(egui::Label::new(crate::i18n::rich(name, 10.5, Some(pal.faint), false)).wrap_mode(egui::TextWrapMode::Extend)); });
    });
    let (r, _) = ui.allocate_exact_size(vec2(1.0, 74.0), Sense::hover());
    ui.painter().line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, pal.border));
}

fn swatch(ui: &mut Ui, pal: &Pal, c: Option<Color32>, selected: bool, tip: &str) -> Response {
    let (rect, r) = ui.allocate_exact_size(vec2(17.0, 17.0), Sense::click());
    let rr = if r.hovered() { rect.expand(1.0) } else { rect };
    match c {
        Some(c) => { ui.painter().rect_filled(rr, 4.0, c); }
        None => {
            ui.painter().rect_filled(rr, 4.0, pal.panel);
            ui.painter().line_segment([rr.left_bottom(), rr.right_top()], Stroke::new(1.5, Color32::from_rgb(229, 72, 77)));
        }
    }
    ui.painter().rect_stroke(rr, 4.0, Stroke::new(1.0, if pal.dark { Color32::from_white_alpha(40) } else { Color32::from_black_alpha(30) }), StrokeKind::Inside);
    if selected { ui.painter().rect_stroke(rr.expand(2.0), 5.0, Stroke::new(1.5, pal.accent), StrokeKind::Outside); }
    tip_text(r, tip)
}

/// Colour row: Auto (theme), None, swatches, custom picker. Returns Some(new value) when changed.
fn color_row(ui: &mut Ui, pal: &Pal, cur: Option<Col>, auto_col: Color32, allow_none: bool) -> Option<Option<Col>> {
    let mut res = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        // auto
        let (rect, r) = ui.allocate_exact_size(vec2(17.0, 17.0), Sense::click());
        ui.painter().rect_filled(rect, 4.0, auto_col);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, "A", FontId::proportional(10.0), if auto_col.r() as u32 + auto_col.g() as u32 + auto_col.b() as u32 > 380 { Color32::BLACK } else { Color32::WHITE });
        if cur.is_none() { ui.painter().rect_stroke(rect.expand(2.0), 5.0, Stroke::new(1.5, pal.accent), StrokeKind::Outside); }
        if r.on_hover_text(t("Automatic (follows light/dark theme)")).clicked() { res = Some(None); }
        if allow_none && swatch(ui, pal, None, cur.map_or(false, |c| c.is_none()), "No colour").clicked() { res = Some(Some(Col::NONE)); }
        for h in theme::SWATCHES {
            let c = Col::hex(h);
            if swatch(ui, pal, Some(to_c32(c)), cur == Some(c), h).clicked() { res = Some(Some(c)); }
        }
        let mut c32 = cur.filter(|c| !c.is_none()).map(to_c32).unwrap_or(auto_col);
        if egui::color_picker::color_edit_button_srgba(ui, &mut c32, egui::color_picker::Alpha::OnlyBlend).on_hover_text(t("Custom colour")).changed() {
            res = Some(Some(from_c32(c32)));
        }
    });
    res
}

fn seg<T: PartialEq + Copy>(ui: &mut Ui, pal: &Pal, cur: T, opts: &[(T, &str, &str)]) -> Option<T> {
    let mut out = None;
    let n = opts.len() as f32;
    let (rect, _) = ui.allocate_exact_size(vec2(34.0 * n, 26.0), Sense::hover());
    ui.painter().rect_filled(rect, 7.0, pal.panel2);
    ui.painter().rect_stroke(rect, 7.0, Stroke::new(1.0, pal.border), StrokeKind::Inside);
    for (i, (v, icon, tip)) in opts.iter().enumerate() {
        let r = Rect::from_min_size(rect.min + vec2(34.0 * i as f32, 0.0), vec2(34.0, 26.0));
        let resp = ui.interact(r, ui.id().with(("seg", tip, i)), Sense::click());
        let on = cur == *v;
        if on { ui.painter().rect_filled(r.shrink(2.0), 5.0, pal.accent_soft); } else if resp.hovered() { ui.painter().rect_filled(r.shrink(2.0), 5.0, pal.hover); }
        paint_icon(ui, icon, Rect::from_center_size(r.center(), vec2(16.0, 16.0)), if on { pal.accent } else { pal.muted });
        if tip_text(resp, tip).clicked() { out = Some(*v); }
    }
    out
}

fn section(ui: &mut Ui, pal: &Pal, title: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(4.0);
    egui::CollapsingHeader::new(crate::i18n::rich(tr(title).to_uppercase(), 11.0, Some(pal.muted), true)).id_salt(title).default_open(true).show(ui, |ui| {
        ui.add_space(2.0);
        crate::i18n::mirrored(ui, add);
        ui.add_space(6.0);
    });
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([pos2(r.min.x - 4.0, r.min.y), pos2(r.max.x + 4.0, r.min.y)], Stroke::new(1.0, pal.border));
}

fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(72.0, 22.0), Sense::hover());
        let (at, anchor) = if crate::i18n::rtl() { (r.right_center(), Align2::RIGHT_CENTER) } else { (r.left_center(), Align2::LEFT_CENTER) };
        crate::i18n::text(ui.painter(), at, anchor, tr(label), FontId::proportional(12.0), ui.visuals().weak_text_color());
        add(ui);
    });
}

// ------------------------------------------------------------------ title bar & ribbon
pub fn top(app: &mut App, ui: &mut Ui) {
    let pal = app.pal;
    egui::Panel::top("titlebar").exact_size(38.0).frame(egui::Frame::new().fill(pal.chrome).inner_margin(Margin::symmetric(10, 0))).show(ui, |ui| {
        let bar = ui.max_rect();
        let drag = ui.interact(bar, EId::new("title-drag"), Sense::click_and_drag());
        if drag.drag_started() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag); }
        if drag.double_clicked() { let m = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false)); ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!m)); }
        ui.with_layout(crate::i18n::row_layout(Align::Center), |ui| {
            if cfg!(target_os = "macos") && !crate::i18n::rtl() { ui.add_space(68.0); }
            // brand mark: the app icon itself
            let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
            let tex = brand_texture(ui.ctx());
            ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            ui.label(crate::i18n::rich(tr("Naqsha"), 13.5, None, true));
            ui.add_space(10.0);
            if tb(ui, &pal, "save", "", "Save (⌘S)", false).clicked() { app.save(false); }
            if ui.add_enabled_ui(!app.undo.is_empty(), |ui| tb(ui, &pal, "undo", "", "Undo (⌘Z)", false)).inner.clicked() { app.do_undo(); }
            if ui.add_enabled_ui(!app.redo.is_empty(), |ui| tb(ui, &pal, "redo", "", "Redo (⇧⌘Z)", false)).inner.clicked() { app.do_redo(); }
        });
        let title = format!("{}{}", app.title(), if app.dirty { format!("  — {}", tr("Edited")) } else { String::new() });
        crate::i18n::text(ui.painter(), bar.center(), Align2::CENTER_CENTER, &title, FontId::proportional(13.0), pal.text);
        // theme, shortcuts and language at the far end (the left in right-to-left languages, after
        // the macOS window buttons)
        let right = if crate::i18n::rtl() { Rect::from_min_max(pos2(bar.min.x + if cfg!(target_os = "macos") { 68.0 } else { 0.0 }, bar.min.y), pos2(bar.min.x + 210.0, bar.max.y)) } else { Rect::from_min_max(pos2(bar.max.x - 140.0, bar.min.y), bar.max) };
        ui.scope_builder(egui::UiBuilder::new().max_rect(right).layout(crate::i18n::end_layout(Align::Center)), |ui| {
            // one toggle: shows what you switch to; right-click to follow the system again
            let (icon, tip) = if pal.dark { ("sun", "Switch to light mode (⇧⌘D) — right-click to follow the system") } else { ("moon", "Switch to dark mode (⇧⌘D) — right-click to follow the system") };
            let r = tb(ui, &pal, icon, "", tip, false);
            if r.clicked() { toggle_theme(app); }
            r.context_menu(|ui| {
                if ui.radio(app.set.mode == Mode::System, t("Follow system appearance")).clicked() { app.set.mode = Mode::System; ui.close(); }
                if ui.radio(app.set.mode == Mode::Light, t("Light")).clicked() { app.set.mode = Mode::Light; ui.close(); }
                if ui.radio(app.set.mode == Mode::Dark, t("Dark")).clicked() { app.set.mode = Mode::Dark; ui.close(); }
            });
            if tb(ui, &pal, "keyboard", "", "Keyboard shortcuts", false).clicked() { app.modal = Some(Modal::Shortcuts); }
            let r = tb(ui, &pal, "globe", "", "Language", false);
            egui::Popup::menu(&r).show(|ui| language_menu(app, ui));
        });
    });

    egui::Panel::top("tabs").exact_size(30.0).frame(egui::Frame::new().fill(pal.chrome).inner_margin(Margin { left: 10, right: 10, top: 2, bottom: 0 })).show(ui, |ui| {
        ui.with_layout(crate::i18n::row_layout(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (t, n) in [(Tab::File, "File"), (Tab::Home, "Home"), (Tab::Insert, "Insert"), (Tab::Design, "Design"), (Tab::Arrange, "Arrange"), (Tab::View, "View"), (Tab::Pro, "Pro")] {
                let on = app.tab == t;
                let (rect, r) = ui.allocate_exact_size(vec2(70.0, 28.0), Sense::click());
                if on {
                    ui.painter().rect_filled(rect, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, pal.panel);
                    ui.painter().line_segment([rect.left_top() + vec2(6.0, 0.5), rect.right_top() + vec2(-6.0, 0.5)], Stroke::new(2.0, pal.accent));
                }
                let c = if on || t == Tab::File || (t == Tab::Pro && crate::pro::active(app)) { pal.accent } else if r.hovered() { pal.text } else { pal.muted };
                crate::i18n::text(ui.painter(), rect.center(), Align2::CENTER_CENTER, tr(n), FontId::proportional(12.5), c);
                if r.clicked() { app.tab = t; }
            }
        });
    });

    egui::Panel::top("ribbon").exact_size(92.0).frame(egui::Frame::new().fill(pal.panel).inner_margin(Margin::symmetric(10, 6)).stroke(Stroke::new(1.0, pal.border))).show(ui, |ui| {
        let ribbon = |app: &mut App, ui: &mut Ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 2.0);
            match app.tab {
                Tab::File => ribbon_file(app, ui, &pal),
                Tab::Home => ribbon_home(app, ui, &pal),
                Tab::Insert => ribbon_insert(app, ui, &pal),
                Tab::Design => ribbon_design(app, ui, &pal),
                Tab::Arrange => ribbon_arrange(app, ui, &pal),
                Tab::View => ribbon_view(app, ui, &pal),
                Tab::Pro => crate::pro::ribbon(app, ui, &pal),
            }
        };
        // right to left: groups start at the right edge (and a narrow window cuts the last ones off)
        if crate::i18n::rtl() { ui.with_layout(Layout::right_to_left(Align::Min), |ui| ribbon(app, ui)); }
        else {
            egui::ScrollArea::horizontal().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.horizontal_top(|ui| ribbon(app, ui));
            });
        }
    });
}

/// Interface language: the system's, or one of ours (each named in its own language).
pub fn language_menu(app: &mut App, ui: &mut Ui) {
    ui.set_min_width(170.0);
    if ui.radio(app.set.lang.is_empty(), crate::i18n::t("System language")).clicked() { set_language(app, ""); ui.close(); }
    ui.separator();
    for (code, name, _) in crate::i18n::LANGS {
        if ui.radio(app.set.lang == *code, crate::i18n::w(*name)).clicked() { set_language(app, code); ui.close(); }
    }
}

pub fn set_language(app: &mut App, code: &str) {
    app.set.lang = code.to_string();
    crate::i18n::set(code);
}

fn ribbon_file(app: &mut App, ui: &mut Ui, pal: &Pal) {
    group(ui, pal, "Document", |ui| {
        if big(ui, pal, "newdoc", "New", &trf("New diagram from a template — opens in a new {window} (⌘N)", &[("window", tr(crate::files::WINDOW))])).clicked() { app.modal = Some(Modal::NewDiagram); }
        if big(ui, pal, "open", "Open", &trf("Open… — opens in a new {window} unless this one is empty (⌘O)", &[("window", tr(crate::files::WINDOW))])).clicked() { app.open_dialog(); }
        if big(ui, pal, "save", "Save", "Save (⌘S)").clicked() { app.save(false); }
        if big(ui, pal, "saveAs", "Save As", "Save As… (⇧⌘S)").clicked() { app.save(true); }
        if !crate::files::WEB && big(ui, pal, "template", "As Template", "Save this diagram as a template — it appears under New › My Templates").clicked() { app.save_as_template(); }
    });
    group(ui, pal, "Export", |ui| {
        if big(ui, pal, "image", "PNG", "Export current page as PNG").clicked() { do_export(app, ui, "png"); }
        if big(ui, pal, "vector", "SVG", "Export current page as SVG").clicked() { do_export(app, ui, "svg"); }
        if big(ui, pal, "pdf", "PDF", "Export current page as PDF").clicked() { do_export(app, ui, "pdf"); }
        if big(ui, pal, "motion", "GIF", "Export current page as an animated GIF (one loop of its animations)").clicked() { do_export(app, ui, "gif"); }
        if big(ui, pal, "export", "Visio", "Export all pages as a Visio drawing (.vsdx)").clicked() { do_export(app, ui, "vsdx"); }
        if big(ui, pal, "export", "draw.io", "Export all pages as a draw.io diagram").clicked() { do_export(app, ui, "drawio"); }
    });
    if !crate::files::WEB { group(ui, pal, "Recent", |ui| {
        crate::i18n::vertical(ui, |ui| {
            if app.set.recent.is_empty() { ui.label(crate::i18n::rich(tr("No recent diagrams"), 12.5, Some(pal.faint), false)); }
            let mut open = None;
            for r in app.set.recent.iter().take(3) {
                let name = std::path::Path::new(r).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if ui.add(egui::Button::new(RichText::new(name.clone()).size(12.0)).frame(false)).on_hover_text(r).clicked() { open = Some(r.clone()); }
            }
            if let Some(p) = open { app.open_path_pending = Some(p.into()); app.guard(Pending::OpenPath); }
        });
    }); }
    group(ui, pal, "About", |ui| {
        if big(ui, pal, "info", "About", "About Naqsha").clicked() { app.modal = Some(Modal::About); }
        if !crate::files::WEB && big(ui, pal, "star", "Pro", "Naqsha Pro: licence, trial and plans").clicked() { app.modal = Some(Modal::Pro); }
        if big(ui, pal, "keyboard", "Shortcuts", "Keyboard shortcuts").clicked() { app.modal = Some(Modal::Shortcuts); }
    });
}

fn ribbon_home(app: &mut App, ui: &mut Ui, pal: &Pal) {
    let has = !app.sel.is_empty();
    group(ui, pal, "Clipboard", |ui| {
        if big(ui, pal, "paste", "Paste", "Paste (⌘V)").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste); }
        crate::i18n::vertical(ui, |ui| {
            if tb(ui, pal, "cut", "Cut", "Cut (⌘X)", false).clicked() { if let Some(j) = app.clip_json() { ui.ctx().copy_text(j); app.delete_sel(); } }
            if tb(ui, pal, "copy", "Copy", "Copy (⌘C)", false).clicked() { if let Some(j) = app.clip_json() { ui.ctx().copy_text(j); } }
            let on = app.painter_style.is_some();
            if tb(ui, pal, "painter", "Format Painter", "Copy formatting, then click a shape", on).clicked() {
                app.painter_style = if on { None } else { app.sel_shapes().first().map(|s| s.style.clone()) };
            }
        });
    });
    group(ui, pal, "Tools", |ui| {
        for (t, icon, l, tip) in [(Tool::Pointer, "pointer", "Pointer", "Pointer tool (V)"), (Tool::Connector, "connector", "Connector", "Connector tool (C)"), (Tool::Text, "text", "Text", "Text tool (T)"), (Tool::Pan, "pan", "Pan", "Pan tool (H) — or hold Space")] {
            let _ = l;
            crate::i18n::vertical(ui, |ui| {
                let on = app.tool == t;
                let (rect, r) = ui.allocate_exact_size(vec2(56.0, 62.0), Sense::click());
                if on { ui.painter().rect_filled(rect, 7.0, pal.accent_soft); } else if r.hovered() { ui.painter().rect_filled(rect, 7.0, pal.hover); }
                paint_icon(ui, icon, Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(22.0, 22.0)), if on { pal.accent } else { pal.muted });
                crate::i18n::text(ui.painter(), rect.center_bottom() - vec2(0.0, 13.0), Align2::CENTER_CENTER, tr(l), FontId::proportional(11.5), if on { pal.accent } else { pal.text });
                if tip_text(r, tip).clicked() { app.tool = t; }
            });
        }
    });
    // font
    let first = app.sel_shapes().first().map(|s| s.style.clone()).or_else(|| app.sel_conns().first().map(|c| c.style.clone()));
    group(ui, pal, "Font", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.horizontal(|ui| {
                let mut fs = first.as_ref().and_then(|s| s.font_size).unwrap_or(13.0);
                ui.add_enabled_ui(has, |ui| {
                    if ui.add(egui::DragValue::new(&mut fs).range(6.0..=144.0).speed(0.3).suffix(" pt")).on_hover_text(t("Font size")).changed() { app.apply_style("fs", |s| s.font_size = Some(fs)); }
                    if tb(ui, pal, "bigger", "", "Bigger", false).clicked() { app.apply_style("fs+", |s| s.font_size = Some((s.font_size.unwrap_or(13.0) + 2.0).min(144.0))); }
                    if tb(ui, pal, "smaller", "", "Smaller", false).clicked() { app.apply_style("fs-", |s| s.font_size = Some((s.font_size.unwrap_or(13.0) - 2.0).max(6.0))); }
                });
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(has, |ui| {
                    let b = first.as_ref().map_or(false, |s| s.bold);
                    let i = first.as_ref().map_or(false, |s| s.italic);
                    if tb(ui, pal, "bold", "", "Bold (⌘B)", b).clicked() { app.apply_style("bold", |s| s.bold = !b); }
                    if tb(ui, pal, "italic", "", "Italic (⌘I)", i).clicked() { app.apply_style("italic", |s| s.italic = !i); }
                    let ha = first.as_ref().and_then(|s| s.align).unwrap_or(HAlign::Center);
                    for (a, ic, tip) in [(HAlign::Left, "alignL", "Align left"), (HAlign::Center, "alignC", "Center"), (HAlign::Right, "alignR", "Align right")] {
                        if tb(ui, pal, ic, "", tip, ha == a).clicked() { app.apply_style("align", |s| s.align = Some(a)); }
                    }
                });
            });
        });
    });
    group(ui, pal, "Shape Styles", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.add_enabled_ui(has, |ui| {
                egui::Grid::new("qstyles").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                    for (i, (n, f, s, t, _)) in theme::DIAGRAM_THEMES.iter().enumerate() {
                        let (rect, r) = ui.allocate_exact_size(vec2(44.0, 24.0), Sense::click());
                        ui.painter().rect_filled(rect, 5.0, to_c32(Col::hex(f)));
                        ui.painter().rect_stroke(rect, 5.0, Stroke::new(if r.hovered() { 2.5 } else { 1.5 }, to_c32(Col::hex(s))), StrokeKind::Inside);
                        ui.painter().text(rect.center(), Align2::CENTER_CENTER, "Abc", FontId::proportional(11.0), to_c32(Col::hex(t)));
                        if r.on_hover_text(*n).clicked() {
                            let (f, s, t) = (Col::hex(f), Col::hex(s), Col::hex(t));
                            app.apply_style("qs", |st| { st.fill = Some(f); st.stroke = Some(s); st.text = Some(t); });
                        }
                        if i % 4 == 3 { ui.end_row(); }
                    }
                });
            });
        });
    });
    group(ui, pal, "Arrange", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.horizontal(|ui| {
                if tb(ui, pal, "front", "Front", "Bring to front (⇧⌘])", false).clicked() { app.z_order(true, false); }
                if tb(ui, pal, "back", "Back", "Send to back (⇧⌘[)", false).clicked() { app.z_order(false, false); }
            });
            ui.horizontal(|ui| {
                if tb(ui, pal, "group", "Group", "Group (⌘G)", false).clicked() { app.group(); }
                if tb(ui, pal, "ungroup", "Ungroup", "Ungroup (⇧⌘G)", false).clicked() { app.ungroup(); }
            });
        });
        let m = app.last_layout();
        if big(ui, pal, "autoArrange", "Auto Arrange", "Arrange shapes and connectors automatically (⇧⌘L). More styles on the Arrange tab.").clicked() { app.auto_arrange(m); }
    });
    group(ui, pal, "Editing", |ui| {
        crate::i18n::vertical(ui, |ui| {
            if tb(ui, pal, "selectAll", "Select All", "Select all (⌘A)", false).clicked() { app.select_all(); }
            if tb(ui, pal, "duplicate", "Duplicate", "Duplicate (⌘D)", false).clicked() { app.duplicate(); }
            if tb(ui, pal, "trash", "Delete", "Delete (⌫)", false).clicked() { app.delete_sel(); }
        });
    });
}

fn ribbon_insert(app: &mut App, ui: &mut Ui, pal: &Pal) {
    group(ui, pal, "Pages", |ui| {
        if big(ui, pal, "page", "New Page", "Insert a new page").clicked() { app.add_page(); }
        if big(ui, pal, "duplicate", "Duplicate", "Duplicate this page").clicked() { app.dup_page(); }
    });
    group(ui, pal, "Sub-diagrams", |ui| {
        let one = app.sel_shapes().len() == 1 && app.sel_conns().is_empty();
        let tip = "Give the selected shape its own page (or add a Subprocess shape that opens a new page)";
        if big(ui, pal, "subpage", "Sub-page", tip).clicked() {
            if one { let id = app.sel_shapes()[0].id; if app.link_target(id).is_some() { app.open_link(id); } else { app.create_subpage(id); } }
            else { let c = app.view_center_world(); let id = app.add_shape("fc_predefined", c); app.create_subpage(id); }
        }
        let n = app.sel_shapes().len();
        if ui.add_enabled_ui(n > 0, |ui| big(ui, pal, "subsel", "From Selection", "Move the selected shapes onto a new page and replace them with one Subprocess shape that opens it")).inner.clicked() { app.subpage_from_selection(); }
    });
    group(ui, pal, "Diagram Parts", |ui| {
        let c = app.view_center_world();
        for (k, icon, l) in [("container", "container", "Container"), ("lane_h", "swimlane", "Swimlane"), ("boundary", "boundary", "Boundary"), ("callout", "callout", "Callout")] {
            if big(ui, pal, icon, l, "").clicked() { let id = app.add_shape(k, c); app.select_only(id); }
        }
    });
    group(ui, pal, "Text & Media", |ui| {
        let c = app.view_center_world();
        if big(ui, pal, "text", "Text Box", "Insert a text box").clicked() { let id = app.add_shape("text", c); canvas::start_edit(app, id, false, None); }
        if big(ui, pal, "heading", "Heading", "Insert a heading").clicked() { let id = app.add_shape("heading", c); canvas::start_edit(app, id, false, None); }
        if big(ui, pal, "image", "Picture", "Insert a PNG or JPEG").clicked() { app.insert_image(); }
    });
    group(ui, pal, "Connectors", |ui| {
        if big(ui, pal, "connector", "Connector", "Connector tool (C)").clicked() { app.tool = Tool::Connector; }
    });
    group(ui, pal, "CI/CD", |ui| {
        if big(ui, pal, "pipeline", "Pipeline", "Draw a CI/CD pipeline from its definition file: GitHub Actions, GitLab CI, Azure Pipelines, Bitbucket Pipelines or a Jenkinsfile").clicked() { app.insert_pipeline_file(); }
        if big(ui, pal, "flows", "Node-RED", "Draw Node-RED flows from flows.json or an export (you can also paste a copied flow)").clicked() { app.insert_flows_file(); }
        if big(ui, pal, "script", "Script File", "Insert a script or config file as a code block").clicked() { app.insert_script_file(); }
    });
    if !crate::files::WEB { group(ui, pal, "Stencils", |ui| {
        if big(ui, pal, "open", "Add Folder", "Load a folder of SVG icons (for example an official cloud icon package) as shape groups").clicked() { app.add_stencil_folder(); }
    }); }
}

fn ribbon_design(app: &mut App, ui: &mut Ui, pal: &Pal) {
    group(ui, pal, "Themes", |ui| {
        let (rect, r) = ui.allocate_exact_size(vec2(66.0, 62.0), Sense::click());
        ui.painter().rect_filled(rect, 8.0, if r.hovered() { pal.hover } else { pal.panel2 });
        ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0, if r.hovered() { pal.accent } else { pal.border }), StrokeKind::Inside);
        ui.painter().rect_filled(Rect::from_center_size(rect.center() - vec2(0.0, 8.0), vec2(30.0, 18.0)), 4.0, pal.shape_fill);
        ui.painter().rect_stroke(Rect::from_center_size(rect.center() - vec2(0.0, 8.0), vec2(30.0, 18.0)), 4.0, Stroke::new(1.5, pal.shape_stroke), StrokeKind::Inside);
        crate::i18n::text(ui.painter(), rect.center_bottom() - vec2(0.0, 11.0), Align2::CENTER_CENTER, tr("Adaptive"), FontId::proportional(10.5), pal.muted);
        if r.on_hover_text(t("Theme-adaptive colours (follow light/dark)")).clicked() { app.apply_diagram_theme(usize::MAX); }
        for (i, (n, f, s, t, _)) in theme::DIAGRAM_THEMES.iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(vec2(66.0, 62.0), Sense::click());
            ui.painter().rect_filled(rect, 8.0, if r.hovered() { pal.hover } else { pal.panel2 });
            ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0, if r.hovered() { pal.accent } else { pal.border }), StrokeKind::Inside);
            for (k, dx) in [(0, -14.0), (1, 0.0), (2, 14.0)] {
                let rr = Rect::from_center_size(rect.center() + vec2(dx, -8.0), vec2(11.0, 20.0));
                let fill = if k == 1 { to_c32(Col::hex(s)) } else { to_c32(Col::hex(f)) };
                ui.painter().rect_filled(rr, 3.0, fill);
                ui.painter().rect_stroke(rr, 3.0, Stroke::new(1.5, to_c32(Col::hex(s))), StrokeKind::Inside);
            }
            let _ = t;
            crate::i18n::text(ui.painter(), rect.center_bottom() - vec2(0.0, 11.0), Align2::CENTER_CENTER, tr(n), FontId::proportional(10.5), pal.muted);
            if r.on_hover_text(w(trf("Apply the {name} theme to this page", &[("name", tr(n))]))).clicked() { app.apply_diagram_theme(i); }
        }
    });
    group(ui, pal, "Connectors", |ui| {
        crate::i18n::vertical(ui, |ui| {
            let r = app.set.routing;
            for (v, ic, l) in [(Routing::Orthogonal, "routeOrtho", "Right-angle"), (Routing::Straight, "routeStraight", "Straight"), (Routing::Curved, "routeCurve", "Curved")] {
                if tb(ui, pal, ic, l, "Set connector style (selected, or all on page)", r == v).clicked() { app.set_routing_all(v); }
            }
        });
        crate::i18n::vertical(ui, |ui| {
            ui.spacing_mut().interact_size.y = 18.0;
            let (mut j, mut a) = (app.page().line_jumps, app.page().avoid_shapes);
            if ui.checkbox(&mut j, t("Line jumps")).on_hover_text(t("Draw a hop where connectors cross")).changed() { app.checkpoint(); app.page_mut().line_jumps = j; }
            if ui.checkbox(&mut a, t("Route around shapes")).on_hover_text(t("Right-angle connectors avoid shapes in their way")).changed() { app.checkpoint(); app.page_mut().avoid_shapes = a; }
        });
    });
    group(ui, pal, "Page Setup", |ui| {
        crate::i18n::vertical(ui, |ui| {
            page_size_combo(app, ui);
            if tb(ui, pal, "sizeFit", "Fit to drawing", "Resize page to fit the drawing", false).clicked() { app.fit_page_to_content(); }
        });
    });
    auto_arrange_group(app, ui, pal);
}

fn auto_arrange_group(app: &mut App, ui: &mut Ui, pal: &Pal) {
    use crate::layout::Mode as L;
    let scope = if app.sel_shapes().len() >= 2 { "the selected shapes" } else { "the page" };
    group(ui, pal, "Auto Arrange", |ui| {
        if big(ui, pal, "layout", "Top-Down", &trf("Arrange {scope} top to bottom (⇧⌘L repeats)", &[("scope", tr(scope))])).clicked() { app.auto_arrange(L::TopDown); }
        if big(ui, pal, "layoutLR", "Left-Right", &trf("Arrange {scope} left to right", &[("scope", tr(scope))])).clicked() { app.auto_arrange(L::LeftRight); }
        if big(ui, pal, "gridLayout", "Grid", &trf("Arrange {scope} in a tidy grid", &[("scope", tr(scope))])).clicked() { app.auto_arrange(L::Grid); }
        if big(ui, pal, "tidy", "Tidy Lines", "Reset connector routes (selected connectors, or all) and route them around shapes").clicked() { app.tidy_connectors(); }
    });
}

const PAGE_SIZES: &[(&str, f32, f32)] = &[("Letter landscape", 1056.0, 816.0), ("Letter portrait", 816.0, 1056.0), ("A4 landscape", 1123.0, 794.0), ("A4 portrait", 794.0, 1123.0), ("A3 landscape", 1587.0, 1123.0), ("Tabloid landscape", 1632.0, 1056.0), ("Wide canvas", 1400.0, 990.0), ("Large board", 2400.0, 1600.0), ("16:9 slide", 1280.0, 720.0)];

fn page_size_combo(app: &mut App, ui: &mut Ui) {
    let (w, h) = (app.page().width, app.page().height);
    let cur = PAGE_SIZES.iter().find(|p| p.1 == w && p.2 == h).map(|p| tr(p.0).to_string()).unwrap_or(trf("Custom {w}×{h}", &[("w", &(w as i32).to_string()), ("h", &(h as i32).to_string())]));
    egui::ComboBox::from_id_salt("pagesize").selected_text(crate::i18n::w(cur)).width(170.0).show_ui(ui, |ui| {
        for (n, pw, ph) in PAGE_SIZES {
            if ui.selectable_label(w == *pw && h == *ph, t(n)).clicked() {
                app.checkpoint();
                let p = app.page_mut();
                p.width = *pw;
                p.height = *ph;
                app.fit_pending = true;
            }
        }
    });
}

fn ribbon_arrange(app: &mut App, ui: &mut Ui, pal: &Pal) {
    group(ui, pal, "Align", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.horizontal(|ui| {
                if tb(ui, pal, "sAlignL", "Left", "Align left", false).clicked() { app.align(0); }
                if tb(ui, pal, "sAlignC", "Center", "Align center", false).clicked() { app.align(1); }
                if tb(ui, pal, "sAlignR", "Right", "Align right", false).clicked() { app.align(2); }
            });
            ui.horizontal(|ui| {
                if tb(ui, pal, "sAlignT", "Top", "Align top", false).clicked() { app.align(3); }
                if tb(ui, pal, "sAlignM", "Middle", "Align middle", false).clicked() { app.align(4); }
                if tb(ui, pal, "sAlignB", "Bottom", "Align bottom", false).clicked() { app.align(5); }
            });
        });
    });
    group(ui, pal, "Distribute", |ui| {
        crate::i18n::vertical(ui, |ui| {
            if tb(ui, pal, "distH", "Horizontally", "Distribute horizontally (3+ shapes)", false).clicked() { app.distribute(true); }
            if tb(ui, pal, "distV", "Vertically", "Distribute vertically (3+ shapes)", false).clicked() { app.distribute(false); }
        });
    });
    group(ui, pal, "Order", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.horizontal(|ui| {
                if tb(ui, pal, "front", "To Front", "Bring to front (⇧⌘])", false).clicked() { app.z_order(true, false); }
                if tb(ui, pal, "forward", "Forward", "Bring forward (⌘])", false).clicked() { app.z_order(true, true); }
            });
            ui.horizontal(|ui| {
                if tb(ui, pal, "back", "To Back", "Send to back (⇧⌘[)", false).clicked() { app.z_order(false, false); }
                if tb(ui, pal, "backward", "Backward", "Send backward (⌘[)", false).clicked() { app.z_order(false, true); }
            });
        });
    });
    group(ui, pal, "Rotate & Flip", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.horizontal(|ui| {
                if tb(ui, pal, "rotate", "90°", "Rotate right 90°", false).clicked() { app.rotate_sel(90.0); }
                if tb(ui, pal, "rotateL", "90°", "Rotate left 90°", false).clicked() { app.rotate_sel(-90.0); }
            });
            ui.horizontal(|ui| {
                if tb(ui, pal, "flipH", "Flip H", "Flip horizontal", false).clicked() { app.flip(true); }
                if tb(ui, pal, "flipV", "Flip V", "Flip vertical", false).clicked() { app.flip(false); }
            });
        });
    });
    group(ui, pal, "Group & Lock", |ui| {
        crate::i18n::vertical(ui, |ui| {
            if tb(ui, pal, "group", "Group", "Group (⌘G)", false).clicked() { app.group(); }
            if tb(ui, pal, "ungroup", "Ungroup", "Ungroup (⇧⌘G)", false).clicked() { app.ungroup(); }
            if tb(ui, pal, "lock", "Lock", "Lock / unlock position (⌘L)", false).clicked() { app.toggle_lock(); }
        });
    });
    auto_arrange_group(app, ui, pal);
}

fn ribbon_view(app: &mut App, ui: &mut Ui, pal: &Pal) {
    group(ui, pal, "Show", |ui| {
        ui.spacing_mut().interact_size.y = 18.0;
        ui.spacing_mut().item_spacing.y = 2.0;
        crate::i18n::vertical(ui, |ui| {
            ui.checkbox(&mut app.set.rulers, t("Rulers"));
            ui.checkbox(&mut app.set.grid, t("Grid"));
            ui.checkbox(&mut app.set.guides, t("Dynamic guides"));
        });
        crate::i18n::vertical(ui, |ui| {
            ui.checkbox(&mut app.set.snap, t("Snap to grid"));
            ui.checkbox(&mut app.set.show_left, t("Shapes panel"));
            ui.checkbox(&mut app.set.show_right, t("Format pane"));
        });
    });
    group(ui, pal, "Animation", |ui| {
        let (icon, label, tip) = if app.set.animate { ("pause", "Pause", "Stop animations moving while you edit") } else { ("play", "Play", "Play animations (set them in the Format pane)") };
        if big(ui, pal, icon, label, tip).clicked() { app.set.animate = !app.set.animate; }
    });
    group(ui, pal, "Grid", |ui| {
        crate::i18n::vertical(ui, |ui| {
            ui.label(crate::i18n::rich(tr("Grid spacing"), 12.5, Some(pal.muted), false));
            ui.add(egui::DragValue::new(&mut app.set.grid_size).range(5.0..=100.0).speed(1.0).suffix(" px"));
        });
    });
    group(ui, pal, "Zoom", |ui| {
        if big(ui, pal, "fit", "Fit Page", "Fit page to window (⌘1)").clicked() { app.fit(); }
        if big(ui, pal, "actual", "100%", "Actual size (⌘0)").clicked() { app.set_zoom(1.0); }
        crate::i18n::vertical(ui, |ui| {
            if tb(ui, pal, "zoomIn", "Zoom In", "⌘=", false).clicked() { app.zoom_center(1.25); }
            if tb(ui, pal, "zoomOut", "Zoom Out", "⌘-", false).clicked() { app.zoom_center(0.8); }
        });
    });
}

pub fn do_export(app: &mut App, ui: &Ui, ext: &str) {
    let painter = ui.painter().clone();
    let moves = crate::anim::page_moves(app.page());
    if ext == "gif" && !moves {
        app.notify(tr("Nothing on this page moves yet: select a shape or connector and pick an animation in the Format pane"));
        return;
    }
    // an SVG file keeps the animations (as CSS); pictures and PDFs show the first frame
    let (svg, _, _) = if ext == "svg" && moves {
        export::page_svg_with(Some(&app.doc), app.page(), &app.pal, &painter, export::content_bounds(app.page(), 24.0), export::Motion::Css)
    } else { export::page_svg(Some(&app.doc), app.page(), &app.pal, &painter, 24.0) };
    let name = if ext == "vsdx" || ext == "drawio" { format!("{}.{ext}", app.title()) } else { format!("{} - {}.{ext}", app.title(), app.page().name) };
    #[cfg(not(target_arch = "wasm32"))]
    let Some(path) = rfd::FileDialog::new().add_filter(ext.to_uppercase(), &[ext]).set_file_name(&name).save_file() else { return };
    #[cfg(target_arch = "wasm32")]
    let path = std::path::PathBuf::from(&name);
    let bytes = match ext {
        "vsdx" | "drawio" => crate::app::foreign_bytes(&app.doc, &path),
        "svg" => Ok(svg.into_bytes()),
        "png" => export::svg_to_png(&svg, 2.0),
        "gif" => export::page_gif(Some(&app.doc), app.page(), &app.pal, &painter, 24.0, 1.0),
        _ => export::svg_to_pdf(&svg),
    };
    let res = bytes.and_then(|b| crate::files::deliver(&path, &b));
    match res { Ok(_) => app.notify(trf("Exported {file}", &[("file", &path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())])), Err(e) => app.notify(trf("Export failed: {error}", &[("error", &e.to_string())])) }
}

// ------------------------------------------------------------------ shapes panel
pub fn left(app: &mut App, ui: &mut Ui) {
    let pal = app.pal;
    if !app.set.show_left { return; }
    crate::i18n::start_panel("shapes").exact_size(236.0).frame(egui::Frame::new().fill(pal.panel).stroke(Stroke::new(1.0, pal.border))).show(ui, |ui| crate::i18n::mirrored(ui, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(crate::i18n::rich(tr("SHAPES"), 11.0, Some(pal.muted), true));
            ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| { ui.add_space(6.0); if tb(ui, &pal, if crate::i18n::rtl() { "chevR" } else { "chevL" }, "", "Hide shapes panel", false).clicked() { app.set.show_left = false; } });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.add(egui::TextEdit::singleline(&mut app.search).hint_text(t("Search shapes")).desired_width(204.0));
        });
        ui.add_space(6.0);
        let q = app.search.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(2.0);
            for (ci, cat) in CATEGORIES.iter().enumerate() {
                let items: Vec<&library::Def> = DEFS.iter().filter(|d| d.cat == ci && (q.is_empty() || d.name.to_lowercase().contains(&q) || tr(d.name).to_lowercase().contains(&q) || d.id.contains(&q))).collect();
                if items.is_empty() { continue; }
                let hdr = egui::CollapsingHeader::new(crate::i18n::rich(tr(cat), 12.5, None, true)).id_salt(cat).default_open(ci < 3 || !q.is_empty());
                let hdr = if !q.is_empty() { hdr.open(Some(true)) } else { hdr };
                hdr.show(ui, |ui| {
                    egui::Grid::new(("grid", ci)).spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                        for (i, d) in items.iter().enumerate() {
                            tile(app, ui, d);
                            if i % 3 == 2 { ui.end_row(); }
                        }
                    });
                });
            }
            // engineering symbol groups from Naqsha Pro
            crate::pro::stencils(app, ui, &q);
            // groups from the user's stencil folders
            if !app.user_lib_loaded { app.load_user_lib(); }
            let mut remove: Option<String> = None;
            let groups = app.user_lib.clone();
            for (gi, g) in groups.iter().enumerate() {
                let icons: Vec<&crate::userlib::UserIcon> = g.icons.iter().filter(|i| q.is_empty() || i.name.to_lowercase().contains(&q)).collect();
                if icons.is_empty() { continue; }
                let hdr = egui::CollapsingHeader::new(crate::i18n::rich(g.name.as_str(), 12.5, None, true)).id_salt(("user", &g.root, &g.name)).default_open(false);
                let hdr = if !q.is_empty() { hdr.open(Some(true)) } else { hdr };
                let r = hdr.show(ui, |ui| {
                    egui::Grid::new(("ugrid", gi)).spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                        for (i, icon) in icons.iter().take(if q.is_empty() { 600 } else { 120 }).enumerate() {
                            user_tile(app, ui, icon);
                            if i % 3 == 2 { ui.end_row(); }
                        }
                    });
                });
                r.header_response.on_hover_text(w(trf("{n} icons from {folder}", &[("n", &g.icons.len().to_string()), ("folder", &g.root)]))).context_menu(|ui| {
                    if ui.button(t("Remove this stencil folder")).clicked() { remove = Some(g.root.clone()); ui.close(); }
                });
            }
            if let Some(r) = remove { app.remove_stencil_folder(&r); }
            ui.add_space(8.0);
            if !crate::files::WEB { ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui.button(t("+  Add stencil folder…")).on_hover_text(t("Load a folder of SVG icons (for example an official cloud icon package) as shape groups")).clicked() { app.add_stencil_folder(); }
            }); }
            ui.add_space(30.0);
        });
    }));
}

fn preview_shape(kind: &str, rect: Rect) -> (Shape, View) {
    let d = library::def(kind);
    let s = Shape { kind: kind.into(), w: d.w, h: d.h, style: library::default_style(kind), ..Default::default() };
    let sc = (rect.width() / d.w).min(rect.height() / d.h);
    let off = rect.center() - vec2(d.w, d.h) * sc / 2.0;
    (s, View { origin: off, pan: egui::Vec2::ZERO, zoom: sc })
}

fn user_tile(app: &mut App, ui: &mut Ui, icon: &crate::userlib::UserIcon) {
    let pal = app.pal;
    let (rect, r) = ui.allocate_exact_size(vec2(68.0, 64.0), Sense::click_and_drag());
    if r.hovered() { ui.painter().rect_filled(rect, 8.0, pal.hover); }
    let icon_rect = Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(36.0, 36.0));
    // rasterise lazily, only once the tile scrolls into view
    if ui.is_rect_visible(rect) && !app.thumbs.contains_key(&icon.path) {
        let tex = crate::userlib::rasterize(&icon.path, 96).map(|(px, w, h)| {
            ui.ctx().load_texture(format!("thumb-{}", icon.path.display()), egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &px), egui::TextureOptions::LINEAR)
        });
        app.thumbs.insert(icon.path.clone(), tex);
    }
    match app.thumbs.get(&icon.path) {
        Some(Some(t)) => {
            let sz = t.size_vec2();
            let s = 36.0 / sz.x.max(sz.y);
            ui.painter().image(t.id(), Rect::from_center_size(icon_rect.center(), sz * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        Some(None) => { ui.painter().text(icon_rect.center(), Align2::CENTER_CENTER, "?", FontId::proportional(18.0), pal.faint); }
        None => {}
    }
    tile_label(ui, rect, &icon.name, if r.hovered() { pal.text } else { pal.muted });
    let r = r.on_hover_text(w(trf("{name} — drag onto the page or click to add", &[("name", &icon.name)])));
    let kind = format!("user:{}", icon.path.to_string_lossy());
    if r.drag_started() { app.stencil_drag = Some(kind.clone()); }
    if r.clicked() {
        ui.ctx().memory_mut(|m| m.stop_text_input());
        let c = app.view_center_world();
        let id = app.add_user_icon(&icon.path, &icon.name, c);
        if id != 0 { app.select_only(id); }
    }
}

/// A stencil tile's name, made to fit its 68 px: smaller type first, then cut short.
pub(crate) fn tile_label(ui: &Ui, rect: Rect, name: &str, color: Color32) {
    let p = ui.painter();
    let max = rect.width() - 2.0;
    let mut g = crate::i18n::galley(p, name, FontId::proportional(10.5), color);
    if g.size().x > max { g = crate::i18n::galley(p, name, FontId::proportional(9.5), color); }
    let mut n = name.chars().count();
    while g.size().x > max && n > 3 {
        n -= 1;
        g = crate::i18n::galley(p, &truncate(name, n), FontId::proportional(9.5), color);
    }
    let at = rect.center_bottom() - vec2(g.size().x / 2.0, 9.0 + g.size().y / 2.0);
    p.galley(at, g, color);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

fn tile(app: &mut App, ui: &mut Ui, d: &library::Def) {
    let pal = app.pal;
    let (rect, r) = ui.allocate_exact_size(vec2(68.0, 64.0), Sense::click_and_drag());
    if r.hovered() { ui.painter().rect_filled(rect, 8.0, pal.hover); }
    let icon_rect = Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(46.0, 32.0));
    let (mut s, v) = preview_shape(d.id, icon_rect);
    if library::is_textlike(d.id) { s.text = "Aa".into(); s.style.font_size = Some(if d.id == "heading" { 34.0 } else { 26.0 }); }
    let empty = HashMap::new();
    let cx = Ctx { painter: ui.painter(), view: v, pal: &pal, images: &empty, doc: None, depth: 0, time: Some(0.0) };
    let mut out = vec![];
    render::draw_shape(&cx, &mut out, &s, library::is_textlike(d.id));
    ui.painter().extend(out);
    tile_label(ui, rect, tr(d.name), if r.hovered() { pal.text } else { pal.muted });
    let r = r.on_hover_text(w(trf("{name} — drag onto the page or click to add", &[("name", tr(d.name))])));
    if r.drag_started() { app.stencil_drag = Some(d.id.to_string()); }
    if r.clicked() {
        ui.ctx().memory_mut(|m| m.stop_text_input());
        let c = app.view_center_world();
        let n = app.page().shapes.len() as f32;
        let id = app.add_shape(d.id, c + vec2((n % 5.0) * 12.0, (n % 5.0) * 12.0));
        app.select_only(id);
    }
}

/// Ghost of the shape being dragged from the stencil.
pub fn stencil_ghost(app: &App, ctx: &egui::Context) {
    let Some(kind) = &app.stencil_drag else { return };
    let Some(p) = ctx.pointer_latest_pos() else { return };
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, EId::new("ghost")));
    if let Some(path) = kind.strip_prefix("user:") {
        if let Some(Some(t)) = app.thumbs.get(std::path::Path::new(path)) {
            let z = if app.canvas_rect.contains(p) { app.zoom } else { 0.6 };
            let sz = t.size_vec2();
            let s = 64.0 * z / sz.x.max(sz.y);
            painter.image(t.id(), Rect::from_center_size(p, sz * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_white_alpha(220));
        }
        return;
    }
    let d = library::def(kind);
    let z = if app.canvas_rect.contains(p) { app.zoom } else { 0.6 };
    let s = crate::pro::symbol(kind).unwrap_or_else(|| Shape { kind: kind.clone(), w: d.w, h: d.h, text: tr(d.text0).into(), style: library::default_style(kind), ..Default::default() });
    let v = View { origin: p - vec2(s.w, s.h) * z / 2.0, pan: egui::Vec2::ZERO, zoom: z };
    let pal = app.pal;
    let empty = HashMap::new();
    let cx = Ctx { painter: &painter, view: v, pal: &pal, images: &empty, doc: None, depth: 0, time: Some(0.0) };
    let mut out = vec![];
    render::draw_shape(&cx, &mut out, &s, true);
    painter.extend(out);
}

// ------------------------------------------------------------------ format pane
pub fn right(app: &mut App, ui: &mut Ui) {
    let pal = app.pal;
    if !app.set.show_right { return; }
    crate::i18n::end_panel("format").exact_size(284.0).frame(egui::Frame::new().fill(pal.panel).stroke(Stroke::new(1.0, pal.border)).inner_margin(Margin { left: 12, right: 10, top: 8, bottom: 8 })).show(ui, |ui| crate::i18n::mirrored(ui, |ui| {
        let ns = app.sel_shapes().len();
        let nc = app.sel_conns().len();
        let title = if ns == 1 && nc == 0 { tr(library::def(&app.sel_shapes()[0].kind).name).to_string() } else if ns + nc == 0 { tr("Page").into() } else if nc > 0 && ns == 0 { if nc == 1 { tr("Connector").into() } else { trf("{n} connectors", &[("n", &nc.to_string())]) } } else { trf("{n} items", &[("n", &(ns + nc).to_string())]) };
        ui.horizontal(|ui| {
            ui.label(crate::i18n::rich(format!("{} · {}", tr("FORMAT"), title.to_uppercase()), 11.0, Some(pal.muted), true));
            ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| { if tb(ui, &pal, if crate::i18n::rtl() { "chevL" } else { "chevR" }, "", "Hide format pane", false).clicked() { app.set.show_right = false; } });
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if ns == 0 && nc == 0 { page_pane(app, ui, &pal); }
            if ns > 0 { shape_pane(app, ui, &pal); }
            if nc > 0 { conn_pane(app, ui, &pal, ns == 0); if ns == 0 { conn_anim_pane(app, ui, &pal); } }
            if ns + nc == 1 { data_pane(app, ui, &pal); }
            ui.add_space(30.0);
        });
    }));
}

fn page_pane(app: &mut App, ui: &mut Ui, pal: &Pal) {
    section(ui, pal, "Page", |ui| {
        let mut name = app.page().name.clone();
        row(ui, "Name", |ui| { if ui.add(egui::TextEdit::singleline(&mut name).desired_width(170.0)).changed() { app.checkpoint_key("pname"); app.page_mut().name = name; } });
        row(ui, "Size", |ui| page_size_combo(app, ui));
        let (mut w, mut h) = (app.page().width, app.page().height);
        row(ui, "W × H", |ui| {
            let a = ui.add(egui::DragValue::new(&mut w).range(200.0..=20000.0).speed(4.0));
            ui.label(t("×"));
            let b = ui.add(egui::DragValue::new(&mut h).range(200.0..=20000.0).speed(4.0));
            if a.changed() || b.changed() { app.checkpoint_key("psize"); let p = app.page_mut(); p.width = w; p.height = h; }
        });
    });
    layers_section(app, ui, pal);
    section(ui, pal, "Background", |ui| {
        let cur = app.page().bg;
        if let Some(v) = color_row(ui, pal, cur, pal.page, false) { app.checkpoint(); app.page_mut().bg = v; }
    });
    section(ui, pal, "Diagram", |ui| {
        let p = app.page();
        ui.label(crate::i18n::rich(trf("{shapes} shapes · {connectors} connectors · {pages} pages", &[("shapes", &p.shapes.len().to_string()), ("connectors", &p.conns.len().to_string()), ("pages", &app.doc.pages.len().to_string())]), 12.5, Some(pal.muted), false));
        ui.label(crate::i18n::rich(tr("Tip: hover a shape and click a blue arrow to add a connected copy (AutoConnect). Drag from an × to draw a connector."), 11.5, Some(pal.faint), false));
    });
}

fn subdiagram_section(app: &mut App, ui: &mut Ui, pal: &Pal, s: &Shape) {
    section(ui, pal, "Sub-diagram", |ui| {
        let here = app.page().id;
        let target = s.link.and_then(|id| app.doc.page_by_id(id)).map(|p| p.name.clone());
        let mut pick: Option<Option<Id>> = None;
        ui.horizontal(|ui| {
            ui.label(crate::i18n::rich(tr("Opens"), 12.5, Some(pal.muted), false));
            egui::ComboBox::from_id_salt("sublink").width(170.0).selected_text(crate::i18n::w(target.clone().unwrap_or_else(|| tr("Nothing").into()))).show_ui(ui, |ui| {
                if ui.selectable_label(s.link.is_none(), t("Nothing")).clicked() { pick = Some(None); }
                for p in app.doc.pages.iter().filter(|p| p.id != here) {
                    if ui.selectable_label(s.link == Some(p.id), &p.name).clicked() { pick = Some(Some(p.id)); }
                }
            });
        });
        if let Some(l) = pick { app.set_link(s.id, l); }
        ui.horizontal(|ui| {
            if target.is_some() {
                if ui.button(w(platform::keys(&format!("{}  ⌘↓", tr("Open"))))).clicked() { app.open_link(s.id); }
                let mut show = !s.no_preview;
                if ui.add_enabled(library::shows_preview(&Shape { no_preview: false, ..s.clone() }), egui::Checkbox::new(&mut show, t("Preview inside"))).changed() { app.toggle_preview(s.id); }
            } else if ui.add_enabled(!s.locked, egui::Button::new(t("Create sub-diagram page"))).clicked() { app.create_subpage(s.id); }
        });
    });
}

fn shape_pane(app: &mut App, ui: &mut Ui, pal: &Pal) {
    let first = app.sel_shapes()[0].clone();
    let single = app.sel_shapes().len() == 1;
    let st = first.style.clone();
    section(ui, pal, "Fill", |ui| {
        if let Some(v) = color_row(ui, pal, st.fill, render::fill_of(&Shape { style: Style::default(), ..first.clone() }, pal), true) { app.apply_style("fill", |s| s.fill = v); }
    });
    section(ui, pal, "Line", |ui| {
        if let Some(v) = color_row(ui, pal, st.stroke, pal.shape_stroke, true) { app.apply_style("stroke", |s| s.stroke = v); }
        line_controls(app, ui, &st);
    });
    section(ui, pal, "Text", |ui| {
        if single {
            let mut t = first.text.clone();
            if ui.add(egui::TextEdit::multiline(&mut t).desired_rows(2).desired_width(f32::INFINITY).hint_text(crate::i18n::t("Shape text"))).changed() {
                app.checkpoint_key("text");
                if let Some(s) = app.page_mut().shape_mut(first.id) { s.text = t; }
            }
            if library::has_compartments(&first.kind) { ui.label(crate::i18n::rich(tr("Use a line with -- to start a new compartment."), 11.0, Some(pal.faint), false)); }
        }
        if let Some(v) = color_row(ui, pal, st.text, pal.shape_text, false) { app.apply_style("tcol", |s| s.text = v); }
        row(ui, "Size", |ui| {
            let mut fs = st.font_size.unwrap_or(13.0);
            if ui.add(egui::DragValue::new(&mut fs).range(6.0..=144.0).speed(0.3).suffix(" pt")).changed() { app.apply_style("fs", |s| s.font_size = Some(fs)); }
            if tb(ui, pal, "bold", "", "Bold", st.bold).clicked() { let b = !st.bold; app.apply_style("bold", |s| s.bold = b); }
            if tb(ui, pal, "italic", "", "Italic", st.italic).clicked() { let i = !st.italic; app.apply_style("italic", |s| s.italic = i); }
            if tb(ui, pal, "script", "", "Code font (monospace)", st.mono).clicked() { let m = !st.mono; app.apply_style("mono", |s| s.mono = m); }
        });
        row(ui, "Horizontal", |ui| {
            if let Some(a) = seg(ui, pal, st.align.unwrap_or(HAlign::Center), &[(HAlign::Left, "alignL", "Left"), (HAlign::Center, "alignC", "Center"), (HAlign::Right, "alignR", "Right")]) { app.apply_style("align", |s| s.align = Some(a)); }
        });
        row(ui, "Vertical", |ui| {
            if let Some(a) = seg(ui, pal, st.valign.unwrap_or(VAlign::Middle), &[(VAlign::Top, "vTop", "Top"), (VAlign::Middle, "vMid", "Middle"), (VAlign::Bottom, "vBot", "Bottom")]) { app.apply_style("valign", |s| s.valign = Some(a)); }
        });
    });
    layer_assign(app, ui, pal);
    if single { subdiagram_section(app, ui, pal, &first); }
    section(ui, pal, "Effects", |ui| {
        let mut sh = st.shadow;
        if ui.checkbox(&mut sh, t("Drop shadow")).changed() { app.apply_style("shadow", |s| s.shadow = sh); }
        row(ui, "Opacity", |ui| {
            let mut o = st.opacity.unwrap_or(1.0) * 100.0;
            if ui.add(egui::Slider::new(&mut o, 5.0..=100.0).suffix("%")).changed() { app.apply_style("opacity", |s| s.opacity = Some(o / 100.0)); }
        });
    });
    section(ui, pal, "Animation", |ui| anim_controls(app, ui, pal, &st, &crate::anim::FOR_SHAPES, "ashape"));
    if single {
        section(ui, pal, "Size & Position", |ui| {
            let mut s2 = first.clone();
            let mut changed = false;
            egui::Grid::new("geom").num_columns(4).spacing(vec2(6.0, 6.0)).show(ui, |ui| {
                ui.label(t("X")); changed |= ui.add(egui::DragValue::new(&mut s2.x).speed(1.0)).changed();
                ui.label(t("Y")); changed |= ui.add(egui::DragValue::new(&mut s2.y).speed(1.0)).changed(); ui.end_row();
                ui.label(t("W")); changed |= ui.add(egui::DragValue::new(&mut s2.w).range(4.0..=20000.0).speed(1.0)).changed();
                ui.label(t("H")); changed |= ui.add(egui::DragValue::new(&mut s2.h).range(4.0..=20000.0).speed(1.0)).changed(); ui.end_row();
                ui.label(t("Angle")); changed |= ui.add(egui::DragValue::new(&mut s2.rot).range(0.0..=359.9).speed(1.0).suffix("°")).changed(); ui.end_row();
            });
            if changed && !first.locked {
                app.checkpoint_key("geom");
                if let Some(s) = app.page_mut().shape_mut(first.id) { s.x = s2.x; s.y = s2.y; s.w = s2.w; s.h = s2.h; s.rot = s2.rot.rem_euclid(360.0); }
            }
            ui.horizontal(|ui| {
                if tb(ui, pal, "flipH", "", "Flip horizontal", first.flip_h).clicked() { app.flip(true); }
                if tb(ui, pal, "flipV", "", "Flip vertical", first.flip_v).clicked() { app.flip(false); }
                if tb(ui, pal, "rotate", "", "Rotate 90°", false).clicked() { app.rotate_sel(90.0); }
                if tb(ui, pal, "lock", "", if first.locked { "Unlock" } else { "Lock position" }, first.locked).clicked() { app.toggle_lock(); }
            });
        });
    }
}

fn line_controls(app: &mut App, ui: &mut Ui, st: &Style) {
    row(ui, "Weight", |ui| {
        let mut w = st.width.unwrap_or(1.5);
        if ui.add(egui::DragValue::new(&mut w).range(0.0..=16.0).speed(0.1).suffix(" px")).changed() { app.apply_style("width", |s| s.width = Some(w)); }
    });
    row(ui, "Pattern", |ui| {
        if let Some(d) = seg(ui, &app.pal.clone(), st.dash, &[(Dash::Solid, "solid", "Solid"), (Dash::Dashed, "dashed", "Dashed"), (Dash::Dotted, "dotted", "Dotted")]) { app.apply_style("dash", |s| s.dash = d); }
    });
}

/// Animation kind, speed and colour for the selection.
fn anim_controls(app: &mut App, ui: &mut Ui, pal: &Pal, st: &Style, kinds: &[Anim], id: &str) {
    use crate::anim;
    row(ui, "Effect", |ui| {
        egui::ComboBox::from_id_salt(id).selected_text(t(anim::name(st.anim))).width(130.0).show_ui(ui, |ui| {
            for &a in kinds {
                if ui.selectable_label(a == st.anim, t(anim::name(a))).clicked() { app.apply_style("anim", |s| s.anim = a); }
            }
        });
    });
    if st.anim == Anim::None {
        ui.label(crate::i18n::rich(tr("Make flows, alarms and running machines move. View › Play starts and stops them; File › GIF exports one loop."), 11.0, Some(pal.faint), false));
        return;
    }
    row(ui, "Speed", |ui| {
        let cur = anim::speed(st);
        for (v, name) in anim::SPEEDS {
            if ui.selectable_label((cur - v).abs() < 1e-3, t(name)).clicked() { app.apply_style("aspeed", |s| s.anim_speed = if v == 1.0 { None } else { Some(v) }); }
        }
    });
    if matches!(st.anim, Anim::Flow | Anim::Dots | Anim::Glow) {
        let line = render::stroke_of(st, pal.conn);
        ui.label(crate::i18n::rich(tr(if st.anim == Anim::Glow { "Glow colour" } else { "Colour of the moving part" }), 11.0, Some(pal.muted), false));
        if let Some(v) = color_row(ui, pal, st.anim_color, line, false) { app.apply_style("acol", |s| s.anim_color = v); }
    }
    if !app.set.animate {
        ui.label(crate::i18n::rich(tr("Animations are paused (View › Play)."), 11.0, Some(pal.faint), false));
    }
}

fn arrow_combo(ui: &mut Ui, id: &str, cur: Arrow) -> Option<Arrow> {
    let mut out = None;
    let name = |a: Arrow| match a { Arrow::None => "None", Arrow::Filled => "Arrow (filled)", Arrow::Open => "Arrow (open)", Arrow::Diamond => "Diamond", Arrow::Circle => "Circle" };
    egui::ComboBox::from_id_salt(id).selected_text(t(name(cur))).width(130.0).show_ui(ui, |ui| {
        for a in [Arrow::None, Arrow::Filled, Arrow::Open, Arrow::Diamond, Arrow::Circle] {
            if ui.selectable_label(a == cur, t(name(a))).clicked() { out = Some(a); }
        }
    });
    out
}

fn conn_pane(app: &mut App, ui: &mut Ui, pal: &Pal, only_conns: bool) {
    let c = app.sel_conns()[0].clone();
    section(ui, pal, "Connector", |ui| {
        if only_conns {
            if let Some(v) = color_row(ui, pal, c.style.stroke, pal.conn, false) { app.apply_style("cstroke", |s| s.stroke = v); }
            line_controls(app, ui, &c.style);
        }
        row(ui, "Style", |ui| {
            if let Some(r) = seg(ui, pal, c.routing, &[(Routing::Orthogonal, "routeOrtho", "Right-angle"), (Routing::Straight, "routeStraight", "Straight"), (Routing::Curved, "routeCurve", "Curved")]) {
                app.checkpoint();
                let sel = app.sel.clone();
                for cc in app.page_mut().conns.iter_mut().filter(|cc| sel.contains(&cc.id)) { cc.routing = r; cc.bends = None; }
            }
        });
        row(ui, "Begin", |ui| {
            if let Some(a) = arrow_combo(ui, "abeg", c.arrow_start) { app.checkpoint(); let sel = app.sel.clone(); for cc in app.page_mut().conns.iter_mut().filter(|cc| sel.contains(&cc.id)) { cc.arrow_start = a; } }
        });
        row(ui, "End", |ui| {
            if let Some(a) = arrow_combo(ui, "aend", c.arrow_end) { app.checkpoint(); let sel = app.sel.clone(); for cc in app.page_mut().conns.iter_mut().filter(|cc| sel.contains(&cc.id)) { cc.arrow_end = a; } }
        });
        if app.sel_conns().len() == 1 && only_conns {
            let mut t = c.text.clone();
            row(ui, "Label", |ui| {
                if ui.add(egui::TextEdit::singleline(&mut t).desired_width(170.0)).changed() { app.checkpoint_key("ctext"); if let Some(cc) = app.page_mut().conn_mut(c.id) { cc.text = t; } }
            });
            if c.bends.is_some() {
                if ui.button(crate::i18n::t("Reset route")).on_hover_text(crate::i18n::t("Return to automatic routing")).clicked() { app.reset_routes(); }
            } else if c.routing != Routing::Curved {
                ui.label(crate::i18n::rich(tr("Drag the small handles on a segment to reshape the route."), 11.0, Some(pal.faint), false));
            }
            let glued = |e: &End| matches!(e, End::Glued { .. });
            ui.label(crate::i18n::rich(trf("Begin: {begin} · End: {end}", &[("begin", tr(if glued(&c.from) { "glued" } else { "free" })), ("end", tr(if glued(&c.to) { "glued" } else { "free" }))]), 11.0, Some(pal.faint), false));
        }
    });
}

fn conn_anim_pane(app: &mut App, ui: &mut Ui, pal: &Pal) {
    let st = app.sel_conns()[0].style.clone();
    section(ui, pal, "Animation", |ui| anim_controls(app, ui, pal, &st, &crate::anim::FOR_CONNS, "aconn"));
}

fn data_pane(app: &mut App, ui: &mut Ui, pal: &Pal) {
    let id = app.sel[0];
    let (is_conn, data) = match app.page().shape(id) { Some(s) => (false, s.data.clone()), None => (true, app.page().conn(id).map(|c| c.data.clone()).unwrap_or_default()) };
    section(ui, pal, "Shape Data", |ui| {
        let mut d = data.clone();
        let mut remove = None;
        for (i, (k, v)) in d.iter_mut().enumerate() {
            // the app's own bookkeeping (naqsha:…) isn't for editing
            if k.starts_with("naqsha:") { continue; }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(k).desired_width(86.0).hint_text(t("Field")));
                ui.add(egui::TextEdit::singleline(v).desired_width(110.0).hint_text(t("Value")));
                if ui.small_button(t("x")).on_hover_text(t("Remove field")).clicked() { remove = Some(i); }
            });
        }
        if let Some(i) = remove { d.remove(i); }
        if ui.button(t("+  Add field")).clicked() { d.push((trf("Field {n}", &[("n", &(d.len() + 1).to_string())]), String::new())); }
        if d != data {
            app.checkpoint_key("data");
            let p = app.page_mut();
            if is_conn { if let Some(c) = p.conn_mut(id) { c.data = d; } } else if let Some(s) = p.shape_mut(id) { s.data = d; }
        }
        if data.is_empty() { ui.label(crate::i18n::rich(tr("Attach key/value data such as owner, cost, IP or status."), 11.0, Some(pal.faint), false)); }
    });
}

// ------------------------------------------------------------------ status bar
pub fn bottom(app: &mut App, ui: &mut Ui) {
    let pal = app.pal;
    egui::Panel::bottom("status").exact_size(32.0).frame(egui::Frame::new().fill(pal.chrome).stroke(Stroke::new(1.0, pal.border)).inner_margin(Margin::symmetric(8, 3))).show(ui, |ui| crate::i18n::mirrored(ui, |ui| {
        ui.horizontal_centered(|ui| {
            let mut go = None;
            let mut menu_action: Option<(usize, &str)> = None;
            for i in 0..app.doc.pages.len() {
                if let Some((ri, buf)) = app.renaming.as_mut().filter(|(ri, _)| *ri == i) {
                    let _ = ri;
                    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(90.0));
                    r.request_focus();
                    if r.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                        let name = buf.trim().to_string();
                        app.renaming = None;
                        if !name.is_empty() { app.checkpoint(); app.doc.pages[i].name = name; }
                    }
                    continue;
                }
                let on = i == app.pi;
                let name = app.doc.pages[i].name.clone();
                let r = ui.add(egui::Button::selectable(on, crate::i18n::rich(name.as_str(), 12.0, Some(if on { pal.text } else { pal.muted }), false)).min_size(vec2(60.0, 24.0)));
                if r.clicked() { go = Some(i); }
                if r.double_clicked() { app.renaming = Some((i, name.clone())); }
                r.context_menu(|ui| {
                    if ui.button(t("Rename")).clicked() { menu_action = Some((i, "rename")); ui.close(); }
                    if ui.button(t("Duplicate")).clicked() { menu_action = Some((i, "dup")); ui.close(); }
                    if ui.button(t("Delete")).clicked() { menu_action = Some((i, "del")); ui.close(); }
                });
            }
            if tb(ui, &pal, "plus", "", "Insert page", false).clicked() { app.add_page(); }
            if let Some(i) = go { if i != app.pi { app.go_page(i); } }
            if let Some((i, a)) = menu_action {
                app.go_page(i);
                match a { "rename" => app.renaming = Some((i, app.doc.pages[i].name.clone())), "dup" => app.dup_page(), _ => app.del_page() }
            }

            ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| {
                if tb(ui, &pal, "fit", "", "Fit page (⌘1)", false).clicked() { app.fit(); }
                let pct = format!("{:.0}%", app.zoom * 100.0);
                if ui.add(egui::Button::new(RichText::new(pct).size(12.0)).frame(false).min_size(vec2(44.0, 22.0))).on_hover_text(t("Actual size (⌘0)")).clicked() { app.set_zoom(1.0); }
                let mut z = app.zoom * 100.0;
                if ui.add(egui::Slider::new(&mut z, 10.0..=800.0).logarithmic(true).show_value(false)).changed() { app.set_zoom(z / 100.0); }
                ui.add_space(12.0);
                let ss = app.sel_shapes();
                let status = if ss.len() == 1 { let s = ss[0]; format!("W {:.0}  H {:.0}  ∠ {:.0}°", s.w, s.h, s.rot) } else if !app.sel.is_empty() { trf("{n} selected", &[("n", &app.sel.len().to_string())]) } else { String::new() };
                let mouse = app.mouse_world.map(|m| format!("X {:.0}  Y {:.0}", m.x, m.y)).unwrap_or_default();
                ui.label(RichText::new(format!("{status}     {mouse}")).size(11.5).color(pal.muted).monospace());
            });
        });
    }));
}

// ------------------------------------------------------------------ context menu
pub fn context_menu(app: &mut App, ctx: &egui::Context) {
    let Some(pos) = app.ctx_menu else { return };
    let pal = app.pal;
    let mut close = false;
    let has = !app.sel.is_empty();
    let resp = egui::Area::new(EId::new("ctxmenu")).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).fill(pal.panel).show(ui, |ui| {
            ui.set_min_width(200.0);
            let item = |ui: &mut Ui, label: &str, key: &str, enabled: bool| -> bool {
                ui.add_enabled(enabled, egui::Button::new(t(label)).shortcut_text(platform::keys(key)).frame(false).min_size(vec2(190.0, 22.0))).clicked()
            };
            if item(ui, "Cut", "⌘X", has) { if let Some(j) = app.clip_json() { ctx.copy_text(j); app.delete_sel(); } close = true; }
            if item(ui, "Copy", "⌘C", has) { if let Some(j) = app.clip_json() { ctx.copy_text(j); } close = true; }
            if item(ui, "Paste", "⌘V", true) { ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste); close = true; }
            if item(ui, "Duplicate", "⌘D", has) { app.duplicate(); close = true; }
            ui.separator();
            let one = app.sel.len() == 1;
            if item(ui, "Edit Text", "↩", one) { let id = app.sel[0]; let conn = app.page().conn(id).is_some(); canvas::start_edit(app, id, conn, None); close = true; }
            if item(ui, "Bring to Front", "⇧⌘]", has) { app.z_order(true, false); close = true; }
            if item(ui, "Send to Back", "⇧⌘[", has) { app.z_order(false, false); close = true; }
            if item(ui, "Group", "⌘G", app.sel_shapes().len() > 1) { app.group(); close = true; }
            if item(ui, "Ungroup", "⇧⌘G", has) { app.ungroup(); close = true; }
            if item(ui, "Lock / Unlock", "⌘L", has) { app.toggle_lock(); close = true; }
            if item(ui, if app.sel_shapes().len() >= 2 { "Auto Arrange Selection" } else { "Auto Arrange Page" }, "⇧⌘L", true) { let m = app.last_layout(); app.auto_arrange(m); close = true; }
            if item(ui, "Reset Connector Route", "", app.sel_conns().iter().any(|c| c.bends.is_some())) { app.reset_routes(); close = true; }
            if item(ui, "Copy Formatting", "", app.sel_shapes().len() == 1) { app.painter_style = app.sel_shapes().first().map(|s| s.style.clone()); close = true; }
            ui.separator();
            let single = if app.sel_shapes().len() == 1 && app.sel_conns().is_empty() { Some(app.sel_shapes()[0].clone()) } else { None };
            if let Some(s) = &single {
                if app.link_target(s.id).is_some() {
                    if item(ui, "Open Sub-diagram", "⌘↓", true) { app.open_link(s.id); close = true; }
                    if item(ui, if s.no_preview { "Show Preview" } else { "Hide Preview" }, "", library::shows_preview(&Shape { no_preview: false, ..s.clone() })) { app.toggle_preview(s.id); close = true; }
                } else if item(ui, "Create Sub-diagram Page", "", !s.locked) { app.create_subpage(s.id); close = true; }
                let here = app.page().id;
                let pages: Vec<(Id, String)> = app.doc.pages.iter().filter(|p| p.id != here).map(|p| (p.id, p.name.clone())).collect();
                ui.add_enabled_ui(!pages.is_empty() && !s.locked, |ui| {
                    ui.menu_button(t("Link to Page"), |ui| {
                        for (pid, name) in &pages {
                            if ui.radio(s.link == Some(*pid), w(name.as_str())).clicked() { app.set_link(s.id, Some(*pid)); close = true; ui.close(); }
                        }
                    });
                });
                if s.link.is_some() && item(ui, "Remove Link", "", true) { app.set_link(s.id, None); close = true; }
            }
            if item(ui, "Move Selection to Sub-diagram", "", !app.sel_shapes().is_empty()) { app.subpage_from_selection(); close = true; }
            if app.parent_link().is_some() && item(ui, "Back to Parent Page", "⌘↑", true) { app.go_back(); close = true; }
            ui.separator();
            if item(ui, "Delete", "⌫", has) { app.delete_sel(); close = true; }
        });
    });
    // dismiss: a press anywhere outside the menu (its submenus are popups in front of the canvas
    // and panels, so a press on them doesn't count), Esc, or switching page
    let opened = app.ctx_menu_t;
    let outside = ctx.input(|i| (i.pointer.any_pressed() && i.pointer.press_start_time().map_or(false, |t| t > opened)).then(|| i.pointer.press_origin()).flatten()).map_or(false, |p| {
        !resp.response.rect.contains(p) && ctx.layer_id_at(p).map_or(true, |l| matches!(l.order, egui::Order::Background | egui::Order::Middle))
    });
    // raw events: a focused menu button would otherwise swallow Esc to drop its own focus
    let esc = ctx.input(|i| i.raw.events.iter().any(|e| matches!(e, egui::Event::Key { key: Key::Escape, pressed: true, .. })));
    if close || outside || esc || app.pi != app.ctx_menu_page {
        app.ctx_menu = None;
    }
}

// ------------------------------------------------------------------ new diagram / templates

/// Draw a page scaled into `rect` (template thumbnails).
fn page_thumb(painter: &egui::Painter, doc: &Document, page: &Page, rect: Rect, pal: &Pal) {
    let Some(b) = geom::content_bounds(page) else { return };
    let b = b.expand(10.0);
    let k = (rect.width() / b.width()).min(rect.height() / b.height()).min(0.9);
    let size = b.size() * k;
    let at = rect.center() - size / 2.0;
    let v = View { origin: at - b.min.to_vec2() * k, pan: egui::Vec2::ZERO, zoom: k };
    let empty = HashMap::new();
    let cx = Ctx { painter, view: v, pal, images: &empty, doc: Some(doc), depth: 1, time: Some(0.0) };
    let mut out = vec![];
    for s in page.shapes.iter().filter(|s| page.shape_shown(s)) {
        let legible = s.style.font_size.unwrap_or(13.0) * k >= 4.5;
        render::draw_shape(&cx, &mut out, s, legible);
    }
    let bg = page.bg.map(theme::to_c32).unwrap_or(pal.page);
    for (ci, route) in render::page_routes(page) {
        let c = &page.conns[ci];
        render::draw_conn(&cx, &mut out, c, route, bg, false);
    }
    painter.extend(out);
}

/// The New Diagram dialog. Returns false when it should close.
fn gallery(app: &mut App, ctx: &egui::Context) -> bool {
    let pal = app.pal;
    if app.gallery.entries.is_none() {
        let mut v = crate::templates::builtin();
        v.extend(crate::pro::templates());
        if let Some(d) = crate::templates::user_dir() { v.extend(crate::templates::user(&d)); }
        app.gallery.entries = Some(v);
    }
    let mut open = true;
    let mut create: Option<(Document, String)> = None;
    let mut create_id = String::new();
    let mut remove: Option<std::path::PathBuf> = None;
    let mut open_file = false;
    let mut open_recent: Option<std::path::PathBuf> = None;
    // the browser version has no templates folder of its own
    let cats = if crate::files::WEB { &crate::templates::CATS[..crate::templates::CATS.len() - 1] } else { crate::templates::CATS };
    let r = egui::Modal::new(EId::new("new-diagram")).show(ctx, |ui| {
        let avail = ctx.content_rect();
        let w = (avail.width() - 80.0).clamp(640.0, 1060.0);
        let h = (avail.height() - 330.0).clamp(260.0, 560.0);
        ui.set_width(w);
        ui.horizontal(|ui| {
            ui.label(crate::i18n::rich(tr("New diagram"), 17.0, None, true));
            ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| {
                let te = ui.add(egui::TextEdit::singleline(&mut app.gallery.search).hint_text(t("Search templates")).desired_width(220.0));
                if te.changed() { app.gallery.cat = 0; }
            });
        });
        let hint = if crate::embed::active() { tr("Pick a starting point. It replaces the diagram you're editing.").to_string() }
            else { trf("Pick a starting point. It opens in a new {window} if this one already has a diagram.", &[("window", tr(crate::files::WINDOW))]) };
        ui.label(crate::i18n::rich(hint, 12.0, Some(pal.muted), false));
        ui.add_space(8.0);
        let entries = app.gallery.entries.as_ref().unwrap();
        let q = app.gallery.search.trim().to_lowercase();
        let shown: Vec<usize> = entries.iter().enumerate().filter(|(_, e)| {
            (app.gallery.cat == 0 || cats[app.gallery.cat] == e.cat) && (q.is_empty() || e.name.to_lowercase().contains(&q) || e.desc.to_lowercase().contains(&q) || e.cat.to_lowercase().contains(&q) || tr(&e.name).to_lowercase().contains(&q) || tr(&e.desc).to_lowercase().contains(&q))
        }).map(|(i, _)| i).collect();
        if app.gallery.sel.as_ref().map_or(true, |id| !shown.iter().any(|i| &entries[*i].id == id)) { app.gallery.sel = shown.first().map(|i| entries[*i].id.clone()); }
        ui.horizontal_top(|ui| {
            // categories
            crate::i18n::vertical(ui, |ui| {
                ui.set_width(186.0);
                for (i, c) in cats.iter().enumerate() {
                    let n = if i == 0 { entries.len() } else { entries.iter().filter(|e| e.cat == *c).count() };
                    // categories filled by Naqsha Pro stay hidden in a build without it
                    if n == 0 && (*c == "Water & Process" || *c == "Electrical" || *c == "Manufacturing" || *c == "IoT & Automation") { continue; }
                    let on = app.gallery.cat == i;
                    let (rect, resp) = ui.allocate_exact_size(vec2(180.0, 28.0), Sense::click());
                    if on { ui.painter().rect_filled(rect, 6.0, with_alpha(pal.accent, 0.14)); } else if resp.hovered() { ui.painter().rect_filled(rect, 6.0, pal.hover); }
                    crate::i18n::text(ui.painter(), rect.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, tr(c), FontId::proportional(13.0), if on { pal.accent } else { pal.text });
                    ui.painter().text(rect.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, n.to_string(), FontId::proportional(11.0), pal.faint);
                    if resp.clicked() { app.gallery.cat = i; app.gallery.search.clear(); }
                }
                ui.add_space(10.0);
                if ui.add(egui::Button::new(t("Open existing file…")).min_size(vec2(180.0, 26.0))).clicked() { open_file = true; }
                // recent diagrams, for picking up where you left off
                let recent: Vec<String> = app.set.recent.iter().filter(|r| std::path::Path::new(r).exists()).take(6).cloned().collect();
                if !recent.is_empty() {
                    ui.add_space(12.0);
                    ui.label(crate::i18n::rich(tr("RECENT"), 10.5, Some(pal.faint), true));
                    for r in &recent {
                        let pth = std::path::Path::new(r);
                        let name = pth.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| r.clone());
                        let (rect, resp) = ui.allocate_exact_size(vec2(180.0, 24.0), Sense::click());
                        if resp.hovered() { ui.painter().rect_filled(rect, 6.0, pal.hover); }
                        let g = ui.painter().layout_no_wrap(name, FontId::proportional(12.5), pal.text);
                        let clip = ui.painter().with_clip_rect(rect.shrink2(vec2(8.0, 0.0)));
                        clip.galley(pos2(rect.min.x + 10.0, rect.center().y - g.size().y / 2.0), g, pal.text);
                        if resp.on_hover_text(r).clicked() { open_recent = Some(pth.to_path_buf()); }
                    }
                }
            });
            // fixed-height divider: a separator here would stretch to the dialog's previous height
            // and make the dialog grow a little every frame
            let (div, _) = ui.allocate_exact_size(vec2(9.0, h), Sense::hover());
            ui.painter().vline(div.center().x, div.y_range(), Stroke::new(1.0, pal.border));
            // cards
            let grid_w = w - 186.0 - 24.0;
            ui.allocate_ui_with_layout(vec2(grid_w, h), Layout::top_down(Align::Min), |ui| {
            ui.set_width(grid_w);
            ui.set_height(h);
            egui::ScrollArea::vertical().max_height(h).max_width(grid_w).auto_shrink([false, false]).show(ui, |ui| {
                let cols = (((grid_w - 14.0 + 12.0) / (212.0 + 12.0)).floor() as usize).max(1);
                let cw = ((grid_w - 14.0 - 12.0 * (cols as f32 - 1.0)) / cols as f32).floor();
                if shown.is_empty() {
                    ui.add_space(30.0);
                    ui.label(crate::i18n::rich(tr(if cats[app.gallery.cat] == "My Templates" { "No templates of your own yet. Open a diagram and use File › Save as Template." } else { "Nothing matches your search." }), 12.5, Some(pal.muted), false));
                }
                for row in shown.chunks(cols) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for &i in row {
                            let e = &entries[i];
                            let sel = app.gallery.sel.as_deref() == Some(e.id.as_str());
                            let (rect, resp) = ui.allocate_exact_size(vec2(cw, 210.0), Sense::click());
                            let thumb = Rect::from_min_size(rect.min, vec2(cw, 138.0));
                            let p = ui.painter_at(rect);
                            p.rect_filled(thumb, 8.0, pal.page);
                            p.rect_stroke(thumb, 8.0, Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { pal.accent } else if resp.hovered() { with_alpha(pal.accent, 0.6) } else { pal.border }), StrokeKind::Inside);
                            if e.doc.pages.iter().all(|pg| pg.shapes.is_empty()) {
                                p.text(thumb.center(), Align2::CENTER_CENTER, "+", FontId::proportional(40.0), pal.faint);
                            } else {
                                page_thumb(&ui.painter_at(thumb.shrink(4.0)), &e.doc, &e.doc.pages[0], thumb.shrink(10.0), &pal);
                            }
                            if e.doc.pages.len() > 1 {
                                let t = trf("{n} pages", &[("n", &e.doc.pages.len().to_string())]);
                                let br = Rect::from_min_size(thumb.right_top() + vec2(-64.0, 6.0), vec2(58.0, 18.0));
                                p.rect_filled(br, 9.0, with_alpha(pal.accent, 0.9));
                                crate::i18n::text(&p, br.center(), Align2::CENTER_CENTER, &t, FontId::proportional(10.5), Color32::WHITE);
                            }
                            crate::i18n::text(&p, thumb.left_bottom() + vec2(2.0, 8.0), Align2::LEFT_TOP, tr(&e.name), FontId::proportional(13.0), if sel { pal.accent } else { pal.text });
                            // at most three lines of description; the full text is in the tooltip
                            let mut desc = tr(&e.desc).to_string();
                            loop {
                                let g = crate::i18n::wrapped(&p, &desc, FontId::proportional(11.0), pal.muted, cw - 4.0);
                                if g.rows.len() <= 3 || desc.len() < 8 { p.galley(thumb.left_bottom() + vec2(2.0, 28.0), g, pal.muted); break; }
                                let cut: String = desc.trim_end_matches('…').chars().collect::<Vec<_>>().iter().rev().skip(8).collect::<Vec<_>>().into_iter().rev().collect();
                                desc = format!("{}…", cut.trim_end());
                            }
                            let resp = resp.on_hover_text(crate::i18n::w(tr(&e.desc)));
                            if resp.clicked() { app.gallery.sel = Some(e.id.clone()); }
                            if resp.double_clicked() { create = Some((e.doc.clone(), e.name.clone())); create_id = e.id.clone(); }
                        }
                    });
                    ui.add_space(10.0);
                }
            });
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            let cur = app.gallery.sel.as_ref().and_then(|id| entries.iter().find(|e| &e.id == id));
            if let Some(e) = cur {
                ui.label(crate::i18n::rich(format!("{} · {}", tr(&e.cat), tr(&e.name)), 12.5, Some(pal.muted), false));
                if let Some(p) = &e.path { if ui.small_button(t("Remove template")).on_hover_text(p.display().to_string()).clicked() { remove = Some(p.clone()); } }
            }
            ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| {
                let ok = ui.add_enabled(cur.is_some(), egui::Button::new(crate::i18n::rich(tr("Create"), 12.5, Some(Color32::WHITE), true)).fill(pal.accent).min_size(vec2(90.0, 28.0)));
                if ok.clicked() || (cur.is_some() && ui.input(|i| i.key_pressed(Key::Enter))) { if let Some(e) = cur { create = Some((e.doc.clone(), e.name.clone())); create_id = e.id.clone(); } }
                if ui.add(egui::Button::new(t("Cancel")).min_size(vec2(80.0, 28.0))).clicked() { open = false; }
            });
        });
    });
    if r.should_close() { open = false; }
    if let Some(p) = remove { let _ = std::fs::remove_file(&p); app.gallery.entries = None; }
    if open_file { app.open_dialog(); return false; }
    if let Some(p) = open_recent { app.open_or_spawn(p); return false; }
    if let Some((doc, name)) = create {
        // Pro templates need Pro (a licence or the trial)
        if create_id.starts_with("pro:") && !crate::pro::active(app) { app.modal = Some(Modal::Pro); return false; }
        app.new_from(doc, &name);
        return false;
    }
    open
}

// ------------------------------------------------------------------ dialogs
pub fn modals(app: &mut App, ctx: &egui::Context) {
    let pal = app.pal;
    let Some(m) = app.modal.take() else { return };
    let mut keep = true;
    match &m {
        Modal::NewDiagram => { if !gallery(app, ctx) { keep = false; } }
        Modal::Unsaved(p) => {
            let p = *p;
            egui::Modal::new(EId::new("unsaved")).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.label(crate::i18n::rich(trf("Save changes to “{name}”?", &[("name", &app.title())]), 15.0, None, true));
                ui.add_space(4.0);
                ui.label(crate::i18n::rich(tr("Your changes will be lost if you don't save them."), 12.5, Some(pal.muted), false));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(t("Don't Save")).clicked() { keep = false; app.dirty = false; app.run_pending(p); }
                    ui.with_layout(crate::i18n::end_layout(Align::Center), |ui| {
                        if ui.add(egui::Button::new(crate::i18n::rich(tr("Save"), 12.5, Some(Color32::WHITE), false)).fill(pal.accent)).clicked() { keep = false; if app.save(false) { app.run_pending(p); } }
                        if ui.button(t("Cancel")).clicked() { keep = false; }
                    });
                });
            });
        }
        Modal::About => {
            let r = egui::Modal::new(EId::new("about")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.vertical_centered(|ui| {
                    let tex = brand_texture(ctx);
                    ui.add(egui::Image::new((tex.id(), vec2(88.0, 88.0))));
                    ui.label(crate::i18n::rich(tr("Naqsha"), 22.0, Some(pal.accent), true));
                    ui.label(w(trf("Version {v}", &[("v", env!("CARGO_PKG_VERSION"))])));
                    if !crate::files::WEB { ui.label(RichText::new(crate::pro::status_line(app)).color(pal.muted)); }
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(tr("naqsha (نقشه): a plan, a map, a blueprint.")).italics().color(pal.muted));
                    ui.add_space(4.0);
                    ui.label(crate::i18n::rich(tr("Diagrams and architecture for Mac, Windows, Linux and the web.\nA Nixpx product."), 12.5, Some(pal.muted), false));
                    ui.add_space(10.0);
                    if ui.button(t("Close")).clicked() { keep = false; }
                });
            });
            if r.should_close() { keep = false; }
        }
        Modal::Ext(name) => {
            let name: &'static str = name;
            let r = egui::Modal::new(EId::new(("ext", name))).show(ctx, |ui| { if !crate::pro::ext_dialog(app, ui, &pal, name) { keep = false; } });
            if r.should_close() { keep = false; }
        }
        Modal::Pro => {
            let r = egui::Modal::new(EId::new("pro")).show(ctx, |ui| { if !crate::pro::dialog(app, ui, &pal) { keep = false; } });
            if r.should_close() { keep = false; }
        }
        Modal::Shortcuts => {
            let r = egui::Modal::new(EId::new("keys")).show(ctx, |ui| {
                ui.set_width(460.0);
                ui.label(crate::i18n::rich(tr("Keyboard shortcuts"), 16.0, None, true));
                ui.add_space(6.0);
                egui::Grid::new("kgrid").num_columns(4).spacing(vec2(14.0, 5.0)).show(ui, |ui| {
                    let rows = [
                        ("Pointer / Connector / Text / Pan", "V C T H"), ("Pan canvas", "Space-drag, scroll"), ("Zoom", "⌘ scroll, pinch, ⌘= ⌘-"), ("Fit page / 100%", "⌘1 / ⌘0"),
                        ("Undo / Redo", "⌘Z / ⇧⌘Z"), ("Cut / Copy / Paste", "⌘X ⌘C ⌘V"), ("Duplicate", "⌘D or ⌥-drag"), ("Select all", "⌘A"),
                        ("Delete", "⌫"), ("Nudge", "Arrows (⇧ ×10)"), ("Group / Ungroup", "⌘G / ⇧⌘G"), ("Front / Back", "⇧⌘] / ⇧⌘["),
                        ("Forward / Backward", "⌘] / ⌘["), ("Bold / Italic", "⌘B / ⌘I"), ("Lock", "⌘L"), ("Edit text", "Double-click or just type"),
                        ("Finish editing", "Esc or ⌘↩"), ("Keep proportions", "⇧ while resizing"), ("Rotate in 15° steps", "⇧ while rotating"), ("New / Open / Save", "⌘N ⌘O ⌘S"),
                        ("Auto Arrange", "⇧⌘L"), ("Light / dark", "⇧⌘D"), ("Open sub-diagram", "⌘↓"), ("Back to parent page", "⌘↑"),
                        ("Cancel a drag", "Esc"), ("Close menu", "Esc"),
                    ];
                    for (i, (a, b)) in rows.iter().enumerate() {
                        ui.label(crate::i18n::rich(tr(a), 12.5, Some(pal.muted), false));
                        ui.label(crate::i18n::rich(platform::keys(tr(b)), 12.5, None, true));
                        if i % 2 == 1 { ui.end_row(); }
                    }
                });
                ui.add_space(10.0);
                if ui.button(t("Close")).clicked() { keep = false; }
            });
            if r.should_close() { keep = false; }
        }
    }
    if keep && app.modal.is_none() { app.modal = Some(m); }
}

pub fn toast(app: &mut App, ctx: &egui::Context) {
    let Some((msg, t)) = app.toast.clone() else { return };
    let age = app.now - t;
    if age > 3.5 { app.toast = None; return; }
    let pal = app.pal;
    let screen = ctx.content_rect();
    egui::Area::new(EId::new("toast")).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -48.0)).order(egui::Order::Tooltip).interactable(false).show(ctx, |ui| {
        egui::Frame::new().fill(if pal.dark { Color32::from_rgb(0x2A, 0x2A, 0x44) } else { theme::NX_NIGHT }).corner_radius(10).inner_margin(Margin::symmetric(14, 9)).show(ui, |ui| {
            ui.label(crate::i18n::rich(tr(&msg).to_string(), 12.5, Some(Color32::WHITE), false));
        });
    });
    let _ = screen;
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
}

// ------------------------------------------------------------------ keyboard
pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    // clipboard events arrive even while nothing is focused
    let events = ctx.input(|i| i.events.clone());
    let typing = ctx.egui_wants_keyboard_input() || app.edit.is_some();
    for e in &events {
        match e {
            egui::Event::Copy if !typing => { if let Some(j) = app.clip_json() { ctx.copy_text(j); } }
            egui::Event::Cut if !typing => { if let Some(j) = app.clip_json() { ctx.copy_text(j); app.delete_sel(); } }
            egui::Event::Paste(t) if !typing => app.paste_str(t),
            egui::Event::Text(t) if !typing && app.sel.len() == 1 && !ctx.input(|i| i.modifiers.command) && app.tool == Tool::Pointer => {
                let id = app.sel[0];
                if app.page().shape(id).is_some() && !t.trim().is_empty() {
                    canvas::start_edit(app, id, false, Some(t.clone()));
                }
            }
            _ => {}
        }
    }
    let cmd = egui::Modifiers::COMMAND;
    let cs = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
    let sc = |m: egui::Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
    // file-level shortcuts work even while typing
    if sc(cs, Key::S) { app.save(true); }
    if sc(cmd, Key::S) { app.save(false); }
    if sc(cmd, Key::O) { app.open_dialog(); }
    if sc(cmd, Key::N) { app.modal = Some(Modal::NewDiagram); }
    if typing { return; }
    if sc(cs, Key::Z) || sc(cmd, Key::Y) { app.do_redo(); }
    if sc(cmd, Key::Z) { app.do_undo(); }
    if sc(cmd, Key::A) { app.select_all(); }
    if sc(cmd, Key::D) { app.duplicate(); }
    if sc(cs, Key::G) { app.ungroup(); }
    if sc(cmd, Key::G) { app.group(); }
    if sc(cs, Key::L) { let m = app.last_layout(); app.auto_arrange(m); }
    if sc(cs, Key::D) { toggle_theme(app); }
    if sc(cmd, Key::L) { app.toggle_lock(); }
    if sc(cmd, Key::B) { let b = !app.sel_shapes().first().map_or(false, |s| s.style.bold); app.apply_style("bold", |s| s.bold = b); }
    if sc(cmd, Key::I) { let b = !app.sel_shapes().first().map_or(false, |s| s.style.italic); app.apply_style("italic", |s| s.italic = b); }
    if sc(cs, Key::CloseBracket) { app.z_order(true, false); }
    if sc(cs, Key::OpenBracket) { app.z_order(false, false); }
    if sc(cmd, Key::CloseBracket) { app.z_order(true, true); }
    if sc(cmd, Key::OpenBracket) { app.z_order(false, true); }
    if sc(cmd, Key::Equals) || sc(cmd, Key::Plus) { app.zoom_center(1.25); }
    if sc(cmd, Key::Minus) { app.zoom_center(0.8); }
    if sc(cmd, Key::Num0) { app.set_zoom(1.0); }
    if sc(cmd, Key::Num1) { app.fit(); }
    if sc(cmd, Key::ArrowUp) { app.go_back(); }
    if sc(cmd, Key::ArrowDown) { if let [id] = app.sel[..] { app.open_link(id); } }
    let none = egui::Modifiers::NONE;
    if sc(none, Key::Delete) || sc(none, Key::Backspace) { app.delete_sel(); }
    if app.ctx_menu.is_some() && sc(none, Key::Escape) { app.ctx_menu = None; }
    if sc(none, Key::Escape) { if app.painter_style.is_some() { app.painter_style = None; } else if app.tool != Tool::Pointer { app.tool = Tool::Pointer; } else { app.sel.clear(); } }
    if sc(none, Key::Enter) && app.sel.len() == 1 { let id = app.sel[0]; let conn = app.page().conn(id).is_some(); canvas::start_edit(app, id, conn, None); }
    if app.sel.len() != 1 || app.page().shape(app.sel[0]).is_none() {
        if sc(none, Key::V) { app.tool = Tool::Pointer; }
        if sc(none, Key::C) { app.tool = Tool::Connector; }
        if sc(none, Key::T) { app.tool = Tool::Text; }
        if sc(none, Key::H) { app.tool = Tool::Pan; }
    }
    let step = if ctx.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
    for (k, d) in [(Key::ArrowLeft, vec2(-1.0, 0.0)), (Key::ArrowRight, vec2(1.0, 0.0)), (Key::ArrowUp, vec2(0.0, -1.0)), (Key::ArrowDown, vec2(0.0, 1.0))] {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k) || i.consume_key(egui::Modifiers::SHIFT, k)) { app.nudge(d * step); }
    }
    let _ = geom::snap(0.0, 1.0);
}

// ------------------------------------------------------------------ layers
fn layers_section(app: &mut App, ui: &mut Ui, pal: &Pal) {
    section(ui, pal, "Layers", |ui| {
        let layers = app.page().layers.clone();
        let active = app.page().active_layer;
        let mut change: Option<Box<dyn FnOnce(&mut App)>> = None;
        if layers.is_empty() {
            ui.label(crate::i18n::rich(tr("No layers yet. Shapes without a layer are always visible."), 11.5, Some(pal.faint), false));
        }
        egui::Grid::new("layers").num_columns(5).spacing(vec2(6.0, 4.0)).show(ui, |ui| {
            if !layers.is_empty() {
                for h in ["Active", "Show", "Lock", "Name", ""] { ui.label(crate::i18n::rich(tr(h), 10.5, Some(pal.faint), false)); }
                ui.end_row();
            }
            for l in &layers {
                let id = l.id;
                if ui.radio(active == Some(id), t("")).on_hover_text(t("New shapes go on this layer")).clicked() {
                    change = Some(Box::new(move |a: &mut App| { a.page_mut().active_layer = Some(id); }));
                }
                let mut vis = l.visible;
                if ui.checkbox(&mut vis, t("")).on_hover_text(t("Visible")).changed() {
                    change = Some(Box::new(move |a: &mut App| { a.checkpoint(); if let Some(l) = a.page_mut().layers.iter_mut().find(|l| l.id == id) { l.visible = vis; } a.sel.clear(); }));
                }
                let mut lock = l.locked;
                if ui.checkbox(&mut lock, t("")).on_hover_text(t("Locked: shapes can't be selected")).changed() {
                    change = Some(Box::new(move |a: &mut App| { a.checkpoint(); if let Some(l) = a.page_mut().layers.iter_mut().find(|l| l.id == id) { l.locked = lock; } a.sel.clear(); }));
                }
                let mut name = l.name.clone();
                if ui.add(egui::TextEdit::singleline(&mut name).desired_width(74.0)).changed() {
                    change = Some(Box::new(move |a: &mut App| { a.checkpoint_key("lname"); if let Some(l) = a.page_mut().layers.iter_mut().find(|l| l.id == id) { l.name = name; } }));
                }
                if ui.small_button(t("x")).on_hover_text(t("Delete layer (its shapes are kept, without a layer)")).clicked() {
                    change = Some(Box::new(move |a: &mut App| {
                        a.checkpoint();
                        let p = a.page_mut();
                        p.layers.retain(|l| l.id != id);
                        for s in p.shapes.iter_mut() { if s.layer == Some(id) { s.layer = None; } }
                        for c in p.conns.iter_mut() { if c.layer == Some(id) { c.layer = None; } }
                        if p.active_layer == Some(id) { p.active_layer = None; }
                    }));
                }
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            if ui.button(t("+  New layer")).clicked() {
                change = Some(Box::new(|a: &mut App| {
                    a.checkpoint();
                    let id = a.doc.new_id();
                    let n = a.page().layers.len() + 1;
                    let p = a.page_mut();
                    p.layers.push(Layer { id, name: trf("Layer {n}", &[("n", &n.to_string())]), ..Default::default() });
                    p.active_layer = Some(id);
                }));
            }
            if active.is_some() && ui.button(t("No active layer")).on_hover_text(t("New shapes get no layer")).clicked() {
                change = Some(Box::new(|a: &mut App| a.page_mut().active_layer = None));
            }
        });
        if let Some(f) = change { f(app); }
    });
}

fn layer_assign(app: &mut App, ui: &mut Ui, pal: &Pal) {
    let layers = app.page().layers.clone();
    if layers.is_empty() { return; }
    section(ui, pal, "Layer", |ui| {
        let cur = app.sel_shapes().first().and_then(|s| s.layer).or_else(|| app.sel_conns().first().and_then(|c| c.layer));
        let name = layers.iter().find(|l| Some(l.id) == cur).map(|l| l.name.clone()).unwrap_or("(none)".into());
        let mut pick: Option<Option<Id>> = None;
        egui::ComboBox::from_id_salt("layer-assign").selected_text(w(name)).width(180.0).show_ui(ui, |ui| {
            if ui.selectable_label(cur.is_none(), t("(none)")).clicked() { pick = Some(None); }
            for l in &layers { if ui.selectable_label(cur == Some(l.id), &l.name).clicked() { pick = Some(Some(l.id)); } }
        });
        if let Some(l) = pick {
            app.checkpoint();
            let sel = app.sel.clone();
            let p = app.page_mut();
            for s in p.shapes.iter_mut().filter(|s| sel.contains(&s.id)) { s.layer = l; }
            for c in p.conns.iter_mut().filter(|c| sel.contains(&c.id)) { c.layer = l; }
        }
    });
}

// ------------------------------------------------------------------ native menu bar commands
pub fn menu_command(app: &mut App, ui: &mut Ui, id: &str) {
    let ctx = ui.ctx().clone();
    let typing = ctx.egui_wants_keyboard_input() || app.edit.is_some();
    match id {
        "new" => app.modal = Some(Modal::NewDiagram),
        "open" => app.open_dialog(),
        "save_template" => app.save_as_template(),
        "save" => { app.save(false); }
        "save_as" => { app.save(true); }
        "quit" => app.guard(Pending::Close),
        "shortcuts" => app.modal = Some(Modal::Shortcuts),
        // clipboard goes through egui so text fields keep working
        "cut" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestCut),
        "copy" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestCopy),
        "paste" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
        _ if typing => {} // remaining commands act on the diagram, not on text being edited
        "undo" => app.do_undo(),
        "redo" => app.do_redo(),
        "duplicate" => app.duplicate(),
        "delete" => app.delete_sel(),
        "select_all" => app.select_all(),
        "painter" => app.painter_style = app.sel_shapes().first().map(|s| s.style.clone()),
        "zoom_in" => app.zoom_center(1.25),
        "zoom_out" => app.zoom_center(0.8),
        "zoom_100" => app.set_zoom(1.0),
        "fit" => app.fit(),
        "toggle:rulers" => app.set.rulers = !app.set.rulers,
        "toggle:grid" => app.set.grid = !app.set.grid,
        "toggle:guides" => app.set.guides = !app.set.guides,
        "toggle:snap" => app.set.snap = !app.set.snap,
        "toggle:left" => app.set.show_left = !app.set.show_left,
        "toggle:right" => app.set.show_right = !app.set.show_right,
        "toggle:jumps" => { app.checkpoint(); let v = !app.page().line_jumps; app.page_mut().line_jumps = v; }
        "toggle:avoid" => { app.checkpoint(); let v = !app.page().avoid_shapes; app.page_mut().avoid_shapes = v; }
        "mode:toggle" => toggle_theme(app),
        "mode:system" => app.set.mode = Mode::System,
        "page:new" => app.add_page(),
        "page:dup" => app.dup_page(),
        "page:del" => app.del_page(),
        "insert:image" => app.insert_image(),
        "insert:pipeline" => app.insert_pipeline_file(),
        "insert:flows" => app.insert_flows_file(),
        "insert:script" => app.insert_script_file(),
        "stencil:add" => app.add_stencil_folder(),
        "layer:new" => {
            app.checkpoint();
            let id = app.doc.new_id();
            let n = app.page().layers.len() + 1;
            let p = app.page_mut();
            p.layers.push(Layer { id, name: trf("Layer {n}", &[("n", &n.to_string())]), ..Default::default() });
            p.active_layer = Some(id);
            app.set.show_right = true;
        }
        "front" => app.z_order(true, false),
        "back" => app.z_order(false, false),
        "forward" => app.z_order(true, true),
        "backward" => app.z_order(false, true),
        "group" => app.group(),
        "ungroup" => app.ungroup(),
        "rotate" => app.rotate_sel(90.0),
        "rotate_l" => app.rotate_sel(-90.0),
        "flip_h" => app.flip(true),
        "flip_v" => app.flip(false),
        "lock" => app.toggle_lock(),
        "reset_route" => app.reset_routes(),
        "sub:open" => { if let [id] = app.sel[..] { if !app.open_link(id) { if app.page().shape(id).is_some() { app.create_subpage(id); } } } }
        "sub:back" => { app.go_back(); }
        "sub:selection" => app.subpage_from_selection(),
        "layout:auto" => { let m = app.last_layout(); app.auto_arrange(m); }
        "layout:tb" => app.auto_arrange(crate::layout::Mode::TopDown),
        "layout:lr" => app.auto_arrange(crate::layout::Mode::LeftRight),
        "layout:grid" => app.auto_arrange(crate::layout::Mode::Grid),
        "layout:tidy" => app.tidy_connectors(),
        "dist:h" => app.distribute(true),
        "dist:v" => app.distribute(false),
        _ => {
            if let Some(k) = id.strip_prefix("export:") { do_export(app, ui, k); }
            else if let Some(n) = id.strip_prefix("align:").and_then(|n| n.parse::<u8>().ok()) { app.align(n); }
            else if let Some(k) = id.strip_prefix("insert:") {
                let c = app.view_center_world();
                let sid = app.add_shape(k, c);
                if k == "text" { canvas::start_edit(app, sid, false, None); } else { app.select_only(sid); }
            } else if let Some(i) = id.strip_prefix("recent:").and_then(|n| n.parse::<usize>().ok()) {
                if let Some(p) = app.set.recent.get(i).cloned() { app.open_path_pending = Some(p.into()); app.guard(Pending::OpenPath); }
            }
        }
    }
}

/// Flip between light and dark (from whatever is showing now, including "follow system").
pub fn toggle_theme(app: &mut App) {
    app.set.mode = if app.pal.dark { Mode::Light } else { Mode::Dark };
}

/// The app icon as a texture (title bar, About), loaded once per context.
pub fn brand_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let id = EId::new("naqsha-brand-texture");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) { return t; }
    let img = image::load_from_memory(include_bytes!("../assets/icon-512.png")).map(|i| i.resize(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8());
    let ci = match img {
        Ok(i) => egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw()),
        Err(_) => egui::ColorImage::new([1, 1], vec![Color32::TRANSPARENT]),
    };
    let t = ctx.load_texture("naqsha-brand", ci, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}
