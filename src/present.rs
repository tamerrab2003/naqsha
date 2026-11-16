//! Present mode: the diagram full screen, its story played step by step. Arrow keys, Space, a
//! presentation clicker or a click on the canvas go forward; past a page's last step the next page
//! comes on. The view glides to what each step frames, new items fade in, and the step's caption
//! shows along the bottom. Esc ends it.
use crate::app::{App, Present};
use crate::i18n::{tr, trf};
use crate::model::*;
use crate::story;
use egui::*;

/// Seconds the view takes to move to a new step.
pub const MOVE: f32 = 0.8;

pub fn ease(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 } }

fn first_step(p: &Page) -> Option<usize> { (!p.story.is_empty()).then_some(0) }

/// Starts presenting this page, from `step` (or its first step).
pub fn start(app: &mut App, ctx: &Context, step: Option<usize>) {
    if let Some(e) = app.edit.take() { let _ = e; }
    let full = ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
    let step = step.filter(|s| *s < app.page().story.len()).or_else(|| first_step(app.page()));
    app.present = Some(Present { step, since: app.now, from: (app.pan, app.zoom), restore: (app.sel.clone(), app.pi, full), end_hint: 0.0 });
    app.sel.clear();
    app.hover = None;
    if !full { fullscreen(ctx, true); }
}

pub fn stop(app: &mut App, ctx: &Context) {
    let Some(p) = app.present.take() else { return };
    let (sel, _pi, was_full) = p.restore;
    app.sel = sel.into_iter().filter(|id| app.page().shape(*id).is_some() || app.page().conn(*id).is_some()).collect();
    if !was_full { fullscreen(ctx, false); }
    app.fit_pending = true;
}

fn fullscreen(ctx: &Context, on: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    ctx.send_viewport_cmd(ViewportCommand::Fullscreen(on));
    #[cfg(target_arch = "wasm32")]
    {
        let _ = ctx;
        // the browser only allows this right after a click or key press, which is when it's called
        if let Some(d) = web_sys::window().and_then(|w| w.document()) {
            if on { if let Some(e) = d.document_element() { let _ = e.request_fullscreen(); } } else if d.fullscreen_element().is_some() { d.exit_fullscreen(); }
        }
    }
}

/// Moves by one step (`dir` 1 or -1), on to the next or previous page at either end.
pub fn go(app: &mut App, dir: i32) {
    let Some(p) = &app.present else { return };
    let (n, step) = (app.page().story.len(), p.step);
    let next = match (dir > 0, step) {
        (true, Some(k)) if k + 1 < n => Some((app.pi, Some(k + 1))),
        (false, Some(k)) if k > 0 => Some((app.pi, Some(k - 1))),
        (true, _) if app.pi + 1 < app.doc.pages.len() => Some((app.pi + 1, first_step(&app.doc.pages[app.pi + 1]))),
        (false, _) if app.pi > 0 => { let q = &app.doc.pages[app.pi - 1]; Some((app.pi - 1, q.story.len().checked_sub(1))) }
        _ => None,
    };
    let now = app.now;
    let (pan, zoom) = (app.pan, app.zoom);
    match next {
        Some((pi, step)) => {
            let page_change = pi != app.pi;
            if page_change { app.pi = pi; app.story_step = None; }
            let p = app.present.as_mut().unwrap();
            p.step = step;
            p.since = now;
            p.from = (pan, zoom);
            // a new page cuts straight to its first view
            if page_change { p.from = (Vec2::ZERO, 0.0); }
        }
        None if dir > 0 => { app.present.as_mut().unwrap().end_hint = now + 2.5; }
        None => {}
    }
}

/// What the step frames: its focus items, or the whole page.
pub fn target_rect(page: &Page, step: Option<usize>) -> Rect {
    let focus = step.and_then(|k| page.story.get(k)).map(|s| s.focus.clone()).unwrap_or_default();
    let mut r: Option<Rect> = None;
    for id in &focus {
        if let Some(s) = page.shape(*id) { r = crate::geom::rect_union(r, crate::geom::bbox(s)); }
        if let Some(c) = page.conn(*id) { for p in crate::geom::route(page, c) { r = crate::geom::rect_union(r, Rect::from_center_size(p, vec2(8.0, 8.0))); } }
    }
    r.map(|r| r.expand(24.0)).unwrap_or_else(|| {
        let mut b = Rect::from_min_size(Pos2::ZERO, vec2(page.width, page.height));
        for s in &page.shapes { b = b.union(crate::geom::bbox(s)); }
        b
    })
}

/// One frame of presenting: keys, the view's move, the canvas and the caption.
pub fn frame(app: &mut App, ui: &mut Ui, ctx: &Context) {
    // keys: forward, back, ends, out
    let (fwd, back, home, end, esc) = ctx.input_mut(|i| {
        let mut k = |keys: &[Key]| keys.iter().fold(false, |a, k| i.consume_key(Modifiers::NONE, *k) || a);
        (k(&[Key::ArrowRight, Key::ArrowDown, Key::Space, Key::PageDown, Key::Enter, Key::N]),
         k(&[Key::ArrowLeft, Key::ArrowUp, Key::PageUp, Key::Backspace, Key::P]),
         k(&[Key::Home]), k(&[Key::End]), k(&[Key::Escape]))
    });
    if esc { stop(app, ctx); return; }
    if fwd { go(app, 1); }
    if back { go(app, -1); }
    if home || end {
        let n = app.page().story.len();
        let now = app.now;
        let (pan, zoom) = (app.pan, app.zoom);
        let p = app.present.as_mut().unwrap();
        p.step = if n == 0 { None } else if home { Some(0) } else { Some(n - 1) };
        p.since = now;
        p.from = (pan, zoom);
    }
    // a step that moves on by itself
    let (step, since) = { let p = app.present.as_ref().unwrap(); (p.step, p.since) };
    let t = (app.now - since) as f32;
    if let Some(auto) = step.and_then(|k| app.page().story.get(k)).map(|s| s.auto).filter(|a| *a > 0.0) {
        if t >= auto { go(app, 1); } else { ctx.request_repaint_after(std::time::Duration::from_secs_f32((auto - t).max(0.02))); }
    }
    // the view glides to what the step frames
    let (step, since, from) = { let p = app.present.as_ref().unwrap(); (p.step, p.since, p.from) };
    let t = (app.now - since) as f32;
    if app.canvas_rect.is_finite() && app.canvas_rect.width() > 50.0 {
        let (pan1, z1) = app.frame_rect(target_rect(app.page(), step), 28.0);
        let k = if from.1 <= 0.0 { 1.0 } else { ease(t / MOVE) };
        if k >= 1.0 { app.pan = pan1; app.zoom = z1; }
        else {
            // move the centre in a straight line and the zoom evenly (in log scale)
            let size = app.canvas_rect.size();
            let c0 = (size / 2.0 - from.0) / from.1;
            let c1 = (size / 2.0 - pan1) / z1;
            let z = (from.1.ln() + (z1.ln() - from.1.ln()) * k).exp();
            let c = c0 + (c1 - c0) * k;
            app.zoom = z;
            app.pan = size / 2.0 - c * z;
        }
    }
    let settling = t < MOVE + 0.05 || step.map_or(false, |k| story::settling(app.page(), k, t));
    if settling { ctx.request_repaint(); }

    CentralPanel::no_frame().show(ui, |ui| crate::canvas::canvas(app, ui));
    overlay(app, ctx, step, t);
}

fn overlay(app: &mut App, ctx: &Context, step: Option<usize>, t: f32) {
    let screen = ctx.content_rect();
    let scale = (screen.height() / 900.0).clamp(0.75, 1.6);
    let caption = step.and_then(|k| app.page().story.get(k)).map(|s| s.caption.trim().to_string()).unwrap_or_default();
    if !caption.is_empty() {
        let a = (t / 0.35).clamp(0.0, 1.0);
        Area::new(egui::Id::new("present-caption")).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -36.0 * scale)).order(Order::Foreground).interactable(false).show(ctx, |ui| {
            ui.set_max_width(screen.width() * 0.78);
            Frame::new().fill(Color32::from_rgba_unmultiplied(16, 12, 36, (215.0 * a) as u8)).corner_radius(14).inner_margin(Margin::symmetric((22.0 * scale) as i8, (13.0 * scale) as i8)).show(ui, |ui| {
                let font = FontId::proportional(24.0 * scale);
                let g = crate::i18n::wrapped(ui.painter(), &caption, font, Color32::from_white_alpha((255.0 * a) as u8), screen.width() * 0.78 - 44.0 * scale);
                let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                ui.painter().galley(r.min, g, Color32::WHITE);
            });
        });
    }
    // controls: back, where you are, forward, end
    let n = app.page().story.len();
    let label = match step { Some(k) => trf("Step {k} of {n}", &[("k", &(k + 1).to_string()), ("n", &n.to_string())]), None => app.page().name.clone() };
    let pages = if app.doc.pages.len() > 1 { format!(" · {}", trf("Page {k} of {n}", &[("k", &(app.pi + 1).to_string()), ("n", &app.doc.pages.len().to_string())])) } else { String::new() };
    let mut act = 0;
    Area::new(egui::Id::new("present-controls")).anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -14.0)).order(Order::Foreground).show(ctx, |ui| {
        let hot = ui.rect_contains_pointer(ui.max_rect().expand(40.0));
        let fg = Color32::from_white_alpha(if hot { 235 } else { 120 });
        Frame::new().fill(Color32::from_rgba_unmultiplied(16, 12, 36, if hot { 210 } else { 90 })).corner_radius(10).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let b = |ui: &mut Ui, s: &str, tip: &str| ui.add(Button::new(RichText::new(s).size(15.0).color(fg)).frame(false)).on_hover_text(tr(tip)).clicked();
                if b(ui, "‹", "Back (←)") { act = -1; }
                ui.label(RichText::new(format!("{label}{pages}")).size(12.0).color(fg));
                if b(ui, "›", "Next (→, Space)") { act = 1; }
                ui.add_space(6.0);
                if b(ui, "✕", "End the presentation (Esc)") { act = 9; }
            });
        });
    });
    match act { 1 => go(app, 1), -1 => go(app, -1), 9 => stop(app, ctx), _ => {} }
    if let Some(p) = &app.present {
        if app.now < p.end_hint {
            Area::new(egui::Id::new("present-end")).anchor(Align2::CENTER_TOP, vec2(0.0, 24.0)).order(Order::Foreground).interactable(false).show(ctx, |ui| {
                Frame::new().fill(Color32::from_rgba_unmultiplied(16, 12, 36, 215)).corner_radius(10).inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.label(RichText::new(tr("That was the last step · Esc to finish")).size(13.0).color(Color32::WHITE));
                });
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}
