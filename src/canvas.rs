//! The drawing canvas: rendering, hit-testing and all pointer interaction.
use crate::app::*;
use crate::geom::{self, rot};
use crate::library;
use crate::model::*;
use crate::render::{self, with_alpha, Ctx, View};
use crate::theme::to_c32;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Id as EId, Key, PointerButton, Pos2, Rect, Sense, Shape as EShape, Stroke, StrokeKind, Ui, Vec2};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Hit {
    Handle(i32, i32),
    MHandle(i32, i32),
    Rotate(Id),
    ConnEnd(Id, bool),
    /// Segment handle: connector id, segment index in its route.
    Seg(Id, usize),
    Port(Id, i32),
    Auto(Id, Vec2),
    Conn(Id),
    Shape(Id),
    /// The open-sub-diagram badge of a linked shape.
    Link(Id),
    None,
}

const DIRS: [Vec2; 4] = [Vec2 { x: 0.0, y: -1.0 }, Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 0.0, y: 1.0 }, Vec2 { x: -1.0, y: 0.0 }];

fn handle_pos(s: &Shape, hx: i32, hy: i32) -> Pos2 {
    let (cx, cy) = s.center();
    pos2(cx, cy) + rot(vec2(hx as f32 * s.w / 2.0, hy as f32 * s.h / 2.0), s.rot)
}
fn rot_handle(s: &Shape, z: f32) -> Pos2 {
    let (cx, cy) = s.center();
    pos2(cx, cy) + rot(vec2(0.0, -s.h / 2.0 - 26.0 / z), s.rot)
}
fn auto_pos(s: &Shape, d: Vec2, z: f32) -> Pos2 {
    let (cx, cy) = s.center();
    let b = geom::bbox(s);
    pos2(cx, cy) + vec2(d.x * (b.width() / 2.0 + 22.0 / z), d.y * (b.height() / 2.0 + 22.0 / z))
}

fn shape_hit(s: &Shape, p: Pos2, z: f32) -> bool {
    let pad = if s.w < 20.0 || s.h < 20.0 { 6.0 / z } else { 2.0 / z };
    if geom::contains(s, p, pad) { return true; }
    if (library::is_icon(&s.kind) || s.kind == "image") && !s.text.is_empty() {
        let tb = library::shape_text_box(s);
        let lr = Rect::from_min_size(pos2(s.x + tb.rect.min.x + 20.0, s.y + tb.rect.min.y), vec2(tb.rect.width() - 40.0, 18.0));
        return lr.contains(p);
    }
    false
}

fn find_glue(page: &Page, p: Pos2, z: f32, exclude: Option<Id>) -> Option<(Id, i32)> {
    let th = 10.0 / z;
    for s in page.shapes.iter().rev().filter(|s| page.shape_shown(s) && !page.in_background(s)) {
        if Some(s.id) == exclude { continue; }
        for (i, pt) in geom::ports(s).iter().enumerate() {
            if (pt.pos - p).length() < th { return Some((s.id, i as i32)); }
        }
    }
    for pass in 0..2 {
        for s in page.shapes.iter().rev().filter(|s| page.shape_shown(s) && !page.in_background(s)) {
            if Some(s.id) == exclude || library::is_container(&s.kind) != (pass == 1) { continue; }
            if geom::contains(s, p, 0.0) { return Some((s.id, -1)); }
        }
    }
    None
}

fn hit_test(app: &App, v: &View, sp: Pos2) -> Hit {
    let page = app.page();
    let z = v.zoom;
    let wp = v.w(sp);
    let near = |a: Pos2| (v.s(a) - sp).length() <= 7.0;
    let shapes = app.sel_shapes();
    if app.tool == Tool::Pointer {
        for s in page.shapes.iter().rev().filter(|s| page.shape_shown(s) && app.doc.linked_page(s).is_some()) {
            if badge_visible(s, z) && near_r(v, sp, App::link_badge_pos(s, z), 9.0) { return Hit::Link(s.id); }
        }
        if shapes.len() == 1 && !shapes[0].locked {
            let s = shapes[0];
            if s.kind != "phase" && near(rot_handle(s, z)) { return Hit::Rotate(s.id); }
            for hy in -1..=1 { for hx in -1..=1 { if (hx, hy) != (0, 0) && near(handle_pos(s, hx, hy)) { return Hit::Handle(hx, hy); } } }
        } else if shapes.len() > 1 {
            if let Some(b) = app.sel_bbox() {
                for hy in -1..=1 { for hx in -1..=1 {
                    if (hx, hy) == (0, 0) { continue; }
                    let p = pos2(b.center().x + hx as f32 * b.width() / 2.0, b.center().y + hy as f32 * b.height() / 2.0);
                    if near(p) { return Hit::MHandle(hx, hy); }
                } }
            }
        }
        for c in app.sel_conns() {
            let r = geom::route(page, c);
            if let (Some(a), Some(b)) = (r.first(), r.last()) {
                if near(*b) { return Hit::ConnEnd(c.id, true); }
                if near(*a) { return Hit::ConnEnd(c.id, false); }
            }
        }
        if app.sel_conns().len() == 1 && app.sel_shapes().is_empty() {
            let c = app.sel_conns()[0];
            for (k, m) in seg_handles(c, &geom::route(page, c), z) {
                if (v.s(m) - sp).length() <= 7.0 { return Hit::Seg(c.id, k); }
            }
        }
    }
    if let Some(hid) = app.hover {
        if let Some(s) = page.shape(hid) {
            if matches!(app.tool, Tool::Pointer | Tool::Connector) && !s.locked {
                for (i, p) in geom::ports(s).iter().enumerate() {
                    if (v.s(p.pos) - sp).length() <= 6.0 { return Hit::Port(s.id, i as i32); }
                }
            }
            if app.tool == Tool::Pointer && !library::is_container(&s.kind) && s.kind != "phase" && app.sel.len() <= 1 {
                for d in DIRS { if (v.s(auto_pos(s, d, z)) - sp).length() <= 8.0 { return Hit::Auto(s.id, d); } }
            }
        }
    }
    for c in page.conns.iter().rev().filter(|c| page.conn_editable(c)) {
        let pts: Vec<Pos2> = geom::route(page, c).iter().map(|p| v.s(*p)).collect();
        if geom::dist_poly(sp, &pts) <= 6.0 { return Hit::Conn(c.id); }
        if !c.text.is_empty() && (v.s(geom::poly_mid(&geom::route(page, c))) - sp).length() < 14.0 { return Hit::Conn(c.id); }
    }
    for s in page.shapes.iter().rev().filter(|s| page.shape_editable(s)) {
        if shape_hit(s, wp, z) { return Hit::Shape(s.id); }
    }
    Hit::None
}

fn near_r(v: &View, sp: Pos2, w: Pos2, r: f32) -> bool { (v.s(w) - sp).length() <= r }

/// Linked shapes show their open badge once they are big enough on screen to fit it.
fn badge_visible(s: &Shape, z: f32) -> bool { s.w * z >= 34.0 && s.h * z >= 26.0 }

fn hover_shape(app: &App, v: &View, sp: Pos2) -> Option<Id> {
    let page = app.page();
    let wp = v.w(sp);
    let under = page.shapes.iter().rev().find(|s| page.shape_editable(s) && shape_hit(s, wp, v.zoom) && !library::is_container(&s.kind))
        .or_else(|| page.shapes.iter().rev().find(|s| page.shape_editable(s) && shape_hit(s, wp, v.zoom)))
        .map(|s| s.id);
    if under.is_some() { return under; }
    // keep the previous hover while the pointer travels to its ports / AutoConnect arrows
    if let Some(h) = app.hover {
        if let Some(s) = page.shape(h) {
            if geom::bbox(s).expand(34.0 / v.zoom).contains(wp) { return Some(h); }
        }
    }
    None
}

pub fn start_edit(app: &mut App, id: Id, conn: bool, initial: Option<String>) {
    let text = if conn { app.page().conn(id).map(|c| c.text.clone()) } else { app.page().shape(id).map(|s| s.text.clone()) };
    let Some(text) = text else { return };
    if !conn && app.page().shape(id).map_or(false, |s| s.locked) { return; }
    app.edit = Some(Edit { id, conn, buf: initial.unwrap_or(text), before: app.doc.clone(), focus: true });
    app.sel = vec![id];
}

pub fn commit_edit(app: &mut App) {
    let Some(e) = app.edit.take() else { return };
    let p = app.page_mut();
    if e.conn { if let Some(c) = p.conn_mut(e.id) { c.text = e.buf; } } else if let Some(s) = p.shape_mut(e.id) { s.text = e.buf; }
    app.push_before(e.before);
}

pub fn canvas(app: &mut App, ui: &mut Ui) {
    let full = ui.available_rect_before_wrap();
    let rs = if app.set.rulers { 20.0 } else { 0.0 };
    let crect = Rect::from_min_max(full.min + vec2(rs, rs), full.max);
    // a live view keeps the whole page in sight when its frame is resized
    if app.viewer && app.canvas_rect.size() != crect.size() { app.fit_pending = true; }
    app.canvas_rect = crect;
    if app.fit_pending { app.fit(); }
    if let Some(id) = app.focus_shape.take() {
        // coming back from a sub-diagram: centre the shape we left from, at a readable zoom
        if let Some(s) = app.page().shape(id) {
            let (cx0, cy0) = s.center();
            app.zoom = app.zoom.max(0.6).min(1.5);
            app.pan = crect.size() / 2.0 - vec2(cx0, cy0) * app.zoom;
        }
    }
    let resp = ui.interact(crect, EId::new("naqsha-canvas"), Sense::click_and_drag());
    let _pal = app.pal;
    let painter = ui.painter_at(crect);

    // ---------------- zoom & scroll
    if resp.hovered() || resp.dragged() {
        let (zd, scroll, ptr) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta(), i.pointer.hover_pos()));
        if zd != 1.0 {
            app.zoom_at(zd, ptr.unwrap_or(crect.center()));
        } else if scroll != Vec2::ZERO {
            app.pan += scroll;
        }
    }
    let view = View { origin: crect.min, pan: app.pan, zoom: app.zoom };
    if app.viewer {
        viewer_input(app, ui, &resp, &view);
        paint(app, ui, &painter, &view, crect);
        live_badges(app, &painter, &view);
        breadcrumb_bar(app, ui, crect);
        return;
    }
    let z = view.zoom;
    let ptr = resp.hover_pos().or_else(|| ui.input(|i| i.pointer.latest_pos()));
    app.mouse_world = ptr.filter(|p| crect.contains(*p)).map(|p| view.w(p));
    let shift = ui.input(|i| i.modifiers.shift);
    let alt = ui.input(|i| i.modifiers.alt);
    let space = ui.input(|i| i.key_down(Key::Space)) && app.edit.is_none() && !ui.ctx().egui_wants_keyboard_input();

    // clicking the canvas takes keyboard focus away from side-panel text fields
    if (resp.drag_started() || resp.clicked() || resp.secondary_clicked()) && app.edit.is_none() {
        ui.ctx().memory_mut(|m| m.stop_text_input());
    }
    // ---------------- stencil drop
    if let Some(kind) = app.stencil_drag.clone() {
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if crect.contains(p) {
                    ui.ctx().memory_mut(|m| m.stop_text_input());
                    let id = app.add_shape(&kind, view.w(p));
                    app.select_only(id);
                }
            }
            app.stencil_drag = None;
        }
    }

    // ---------------- hover
    if matches!(app.drag, Drag::None) {
        app.hover = match ptr { Some(p) if crect.contains(p) => hover_shape(app, &view, p), _ => None };
    }

    // ---------------- drag start
    if resp.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
        let wp = view.w(origin);
        let middle = resp.drag_started_by(PointerButton::Middle);
        if app.edit.is_some() { commit_edit(app); }
        if middle || space || app.tool == Tool::Pan {
            app.drag = Drag::Pan { start: origin, pan0: app.pan };
        } else if resp.drag_started_by(PointerButton::Primary) {
            let hit = hit_test(app, &view, origin);
            begin_drag(app, hit, wp, shift, alt);
        }
    }

    // ---------------- dragging
    if resp.dragged() {
        if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
            let wp = view.w(p);
            let cmd = ui.input(|i| i.modifiers.command);
            update_drag(app, p, wp, z, shift, cmd);
        }
    }
    // Esc while dragging puts everything back the way it was
    if !matches!(app.drag, Drag::None | Drag::Pan { .. } | Drag::Marquee { .. }) && ui.input(|i| i.key_pressed(Key::Escape)) {
        cancel_drag(app);
    }
    if resp.drag_stopped() {
        end_drag(app, z);
    }

    // ---------------- clicks
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if app.edit.is_some() { commit_edit(app); }
            let hit = hit_test(app, &view, p);
            click(app, hit, view.w(p), shift);
        }
    }
    if resp.double_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let cmd = ui.input(|i| i.modifiers.command);
            match hit_test(app, &view, p) {
                Hit::Shape(id) | Hit::Link(id) if cmd && app.link_target(id).is_some() => { app.open_link(id); }
                Hit::Shape(id) => start_edit(app, id, false, None),
                Hit::Conn(id) => start_edit(app, id, true, None),
                _ => {}
            }
        }
    }
    if resp.secondary_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            match hit_test(app, &view, p) {
                Hit::Shape(id) | Hit::Conn(id) => if !app.is_sel(id) { app.select_only(id); },
                _ => {}
            }
            app.ctx_menu = Some(p);
            app.ctx_menu_page = app.pi;
            app.ctx_menu_t = ui.input(|i| i.time);
        }
    }

    // ---------------- cursor
    if let Some(p) = ptr.filter(|p| crect.contains(*p)) {
        let icon = if matches!(app.drag, Drag::Pan { .. }) { egui::CursorIcon::Grabbing } else if space || app.tool == Tool::Pan { egui::CursorIcon::Grab } else if app.tool == Tool::Text { egui::CursorIcon::Text } else if app.tool == Tool::Connector { egui::CursorIcon::Crosshair } else {
            match hit_test(app, &view, p) {
                Hit::Handle(hx, hy) | Hit::MHandle(hx, hy) => if hx == 0 { egui::CursorIcon::ResizeVertical } else if hy == 0 { egui::CursorIcon::ResizeHorizontal } else if hx == hy { egui::CursorIcon::ResizeNwSe } else { egui::CursorIcon::ResizeNeSw },
                Hit::Rotate(_) => egui::CursorIcon::Alias,
                Hit::Port(..) => egui::CursorIcon::Crosshair,
                Hit::Auto(..) | Hit::Conn(_) | Hit::Link(_) => egui::CursorIcon::PointingHand,
                Hit::ConnEnd(..) => egui::CursorIcon::Move,
                Hit::Seg(id, k) => {
                    let r = app.page().conn(id).map(|c| geom::route(app.page(), c)).unwrap_or_default();
                    match (r.get(k), r.get(k + 1), app.page().conn(id).map(|c| c.routing)) {
                        (Some(a), Some(b), Some(Routing::Orthogonal)) => if (a.y - b.y).abs() < 0.5 { egui::CursorIcon::ResizeVertical } else { egui::CursorIcon::ResizeHorizontal },
                        _ => egui::CursorIcon::Move,
                    }
                }
                Hit::Shape(id) => if app.page().shape(id).map_or(false, |s| s.locked) { egui::CursorIcon::Default } else { egui::CursorIcon::Move },
                Hit::None => egui::CursorIcon::Default,
            }
        };
        ui.ctx().set_cursor_icon(icon);
    }

    // ---------------- paint
    paint(app, ui, &painter, &view, crect);
    if rs > 0.0 { paint_rulers(app, ui, full, crect, &view); }
    edit_overlay(app, ui, &view);
    breadcrumb_bar(app, ui, crect);
}

/// A live view: drag to pan, scroll or pinch to zoom, click a shape with a sub-page to open it.
fn viewer_input(app: &mut App, ui: &mut Ui, resp: &egui::Response, view: &View) {
    app.hover = None;
    app.sel.clear();
    if resp.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
        app.drag = Drag::Pan { start: origin, pan0: app.pan };
    }
    if resp.dragged() {
        if let (Drag::Pan { start, pan0 }, Some(p)) = (&app.drag, ui.input(|i| i.pointer.latest_pos())) { app.pan = *pan0 + (p - *start); }
    }
    if resp.drag_stopped() { app.drag = Drag::None; }
    let over_link = resp.hover_pos().and_then(|p| match hit_test(app, view, p) { Hit::Shape(id) | Hit::Link(id) if app.link_target(id).is_some() => Some(id), _ => None });
    if over_link.is_some() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    else if matches!(app.drag, Drag::Pan { .. }) { ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing); }
    if resp.clicked() { if let Some(id) = over_link { app.open_link(id); } }
}

/// Live values: a badge with the value on each item, a green / grey dot for running / stopped,
/// red for an alarm.
fn live_badges(app: &App, painter: &egui::Painter, v: &View) {
    use crate::live::{self, State};
    if app.live.is_empty() { return; }
    let page = app.page();
    let pal = app.pal;
    let red = Color32::from_rgb(220, 38, 38);
    let badge = |at: Pos2, text: &str, alarm: bool| {
        let g = painter.layout_no_wrap(text.to_string(), FontId::proportional(12.0), if alarm { Color32::WHITE } else { pal.text });
        let r = Rect::from_center_size(at, g.size() + vec2(12.0, 6.0));
        painter.rect_filled(r.translate(vec2(0.0, 1.0)), 6.0, Color32::from_black_alpha(30));
        painter.rect_filled(r, 6.0, if alarm { red } else { pal.panel });
        painter.rect_stroke(r, 6.0, Stroke::new(1.0, if alarm { red } else { pal.border }), StrokeKind::Inside);
        painter.galley(r.min + vec2(6.0, 3.0), g, pal.text);
    };
    let dot = |at: Pos2, st: State| {
        let c = match st { State::Run => Color32::from_rgb(16, 185, 129), State::Stop => Color32::from_rgb(148, 163, 184), State::Alarm => red, State::None => return };
        painter.circle_filled(at, 5.5, Color32::WHITE);
        painter.circle_filled(at, 4.5, c);
    };
    for s in page.shapes.iter().filter(|s| page.shape_shown(s)) {
        let Some(l) = live::shape_key(s).and_then(|k| app.live.get(&k)) else { continue };
        let b = geom::bbox(s);
        dot(v.s(b.left_top()), l.state);
        if let Some(t) = live::badge_text(l, &s.data) { badge(v.s(pos2(b.center().x, b.min.y)) - vec2(0.0, 14.0), &t, l.state == State::Alarm); }
    }
    for c in &page.conns {
        let Some(l) = live::conn_key(c).and_then(|k| app.live.get(&k)) else { continue };
        let route = geom::route(page, c);
        if route.len() < 2 { continue; }
        let mid = v.s(geom::poly_mid(&route));
        // a connector's state shows in its own animation (flowing or still): only values get a badge
        if let Some(t) = live::badge_text(l, &c.data) { badge(mid - vec2(0.0, 18.0), &t, l.state == State::Alarm); }
    }
}

/// On a sub-diagram page: a Back button and the trail of parent pages, top-left of the canvas.
fn breadcrumb_bar(app: &mut App, ui: &mut Ui, crect: Rect) {
    let crumbs = app.breadcrumbs();
    if crumbs.is_empty() { return; }
    let pal = app.pal;
    let mut go: Option<(usize, Id)> = None;
    let mut back = false;
    egui::Area::new(EId::new("subnav")).fixed_pos(crect.min + vec2(10.0, 10.0)).order(egui::Order::Middle).show(ui.ctx(), |ui| {
        egui::Frame::new().fill(pal.panel).stroke(Stroke::new(1.0, pal.border)).corner_radius(8.0).inner_margin(egui::Margin::symmetric(6, 4)).shadow(egui::epaint::Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(if pal.dark { 70 } else { 22 }) }).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let b = ui.add(egui::Button::new(egui::RichText::new("‹  Back").color(Color32::WHITE).strong()).fill(pal.accent).corner_radius(6.0)).on_hover_text(crate::platform::keys("Back to the parent page (⌘↑)"));
                if b.clicked() { back = true; }
                ui.add_space(4.0);
                for (i, sid) in &crumbs {
                    let name = app.doc.pages[*i].name.clone();
                    if ui.add(egui::Button::new(egui::RichText::new(name).color(pal.muted)).frame(false)).on_hover_text("Go to this page").clicked() { go = Some((*i, *sid)); }
                    ui.label(egui::RichText::new("›").color(pal.muted));
                }
                ui.label(egui::RichText::new(app.page().name.clone()).strong().color(pal.text));
            });
        });
    });
    if back { app.go_back(); } else if let Some((i, sid)) = go { app.go_crumb(i, sid); }
}

fn begin_drag(app: &mut App, hit: Hit, wp: Pos2, shift: bool, alt: bool) {
    let before = app.doc.clone();
    let tool = app.tool;
    match (tool, hit) {
        (Tool::Pointer, Hit::Handle(hx, hy)) => {
            let s = app.sel_shapes()[0].clone();
            app.drag = Drag::Resize { id: s.id, hx, hy, orig: s, before };
        }
        (Tool::Pointer, Hit::MHandle(hx, hy)) => {
            let orig: Vec<Shape> = app.sel_shapes().into_iter().cloned().collect();
            let bbox = app.sel_bbox().unwrap_or(Rect::NOTHING);
            app.drag = Drag::MResize { hx, hy, bbox, orig, before };
        }
        (Tool::Pointer, Hit::Rotate(id)) => app.drag = Drag::Rotate { id, before },
        (Tool::Pointer, Hit::ConnEnd(id, to)) => app.drag = Drag::ConnEnd { id, to, before },
        (Tool::Pointer, Hit::Seg(id, k)) => begin_seg(app, id, k, wp, before),
        (Tool::Pointer | Tool::Connector, Hit::Port(sid, port)) => new_conn(app, End::Glued { shape: sid, port }, wp, before),
        (Tool::Connector, Hit::Shape(sid)) | (Tool::Pointer, Hit::Auto(sid, _)) => new_conn(app, End::Glued { shape: sid, port: -1 }, wp, before),
        (Tool::Connector, _) => new_conn(app, End::Free { x: wp.x, y: wp.y }, wp, before),
        (Tool::Pointer, Hit::Conn(id)) if !shift && !alt && grab_segment(app, id, wp).is_some() => {
            // dragging the line of a glued connector moves the segment under the pointer,
            // like its handle, rather than sliding the whole route off its shapes
            let k = grab_segment(app, id, wp).unwrap();
            app.select_only(id);
            begin_seg(app, id, k, wp, before);
        }
        (Tool::Pointer, Hit::Shape(id)) | (Tool::Pointer, Hit::Conn(id)) | (Tool::Pointer, Hit::Link(id)) => {
            if !app.is_sel(id) {
                if shift { app.sel.push(id); app.expand_groups(); } else { app.select_only(id); }
            }
            if alt && !app.sel_shapes().is_empty() {
                // ⌥-drag duplicates, then moves the copies
                if let Some(j) = app.clip_json() {
                    app.paste_count = -1.0;
                    app.paste_str(&j);
                    app.paste_count = 0.0;
                }
            }
            start_move(app, wp, before);
        }
        (Tool::Pointer, Hit::None) | (Tool::Text, Hit::None) => {
            if !shift { app.sel.clear(); }
            app.drag = Drag::Marquee { start: wp, cur: wp, additive: shift };
        }
        _ => {}
    }
}

/// The segment of a glued connector nearest to `wp`, when the drag should move just that segment.
fn grab_segment(app: &App, id: Id, wp: Pos2) -> Option<usize> {
    let page = app.page();
    let c = page.conn(id)?;
    if c.routing == Routing::Curved || !(matches!(c.from, End::Glued { .. }) || matches!(c.to, End::Glued { .. })) { return None; }
    // part of a larger selection: move everything together
    if app.sel.len() > 1 && app.is_sel(id) { return None; }
    let r = geom::route(page, c);
    r.windows(2).enumerate().min_by(|a, b| geom::dist_seg(wp, a.1[0], a.1[1]).total_cmp(&geom::dist_seg(wp, b.1[0], b.1[1]))).map(|(k, _)| k)
}

fn new_conn(app: &mut App, from: End, wp: Pos2, before: Document) {
    let id = app.doc.new_id();
    let routing = app.set.routing;
    let layer = app.page().active_layer;
    app.page_mut().conns.push(Conn { id, from, to: End::Free { x: wp.x, y: wp.y }, routing, layer, ..Default::default() });
    app.sel = vec![id];
    app.drag = Drag::NewConn { id, before };
}

/// Segment handles for a selected connector: (segment index, midpoint) for segments long enough to grab.
fn seg_handles(c: &Conn, route: &[Pos2], z: f32) -> Vec<(usize, Pos2)> {
    if c.routing == Routing::Curved || route.len() < 2 { return vec![]; }
    route.windows(2).enumerate().filter(|(_, w)| (w[1] - w[0]).length() * z >= 22.0).map(|(k, w)| (k, w[0] + (w[1] - w[0]) * 0.5)).collect()
}

fn begin_seg(app: &mut App, id: Id, k: usize, wp: Pos2, before: Document) {
    let page = app.page();
    let Some(c) = page.conn(id) else { return };
    let route = geom::route(page, c);
    if k + 1 >= route.len() { return; }
    // pin dynamic glue to the port in use, so the ends don't jump while the route changes
    let hint_a = route.get(1).copied().unwrap_or(route[0]);
    let hint_b = route.get(route.len().saturating_sub(2)).copied().unwrap_or(route[0]);
    let pin = |e: End, hint: Pos2| match e {
        End::Glued { shape, port: -1 } => End::Glued { shape, port: geom::resolve_at(page, &e, hint).2 },
        e => e,
    };
    let (from, to) = (pin(c.from, hint_a), pin(c.to, hint_b));
    let mut pts = route.clone();
    let (j, horizontal, free);
    if c.routing == Routing::Straight {
        // insert a bend at the grabbed point; it then moves freely
        pts.insert(k + 1, route[k] + (route[k + 1] - route[k]) * 0.5);
        j = k + 1;
        horizontal = false;
        free = true;
    } else {
        horizontal = (route[k].y - route[k + 1].y).abs() < (route[k].x - route[k + 1].x).abs();
        let mut jj = k;
        // dragging an end segment: split off a short stub at the shape so the moved part jogs
        // away from the port instead of sliding along the shape's edge
        // (only when the end segment is long enough to split; a short one simply moves)
        if jj == 0 && (pts[1] - pts[0]).length() >= 30.0 {
            let d = pts[1] - pts[0];
            let stub = (d.length() / 2.0).min(20.0);
            pts.insert(1, pts[0] + d.normalized() * stub);
            jj = 1;
        }
        let n = pts.len();
        if jj + 1 == n - 1 && (pts[n - 1] - pts[n - 2]).length() >= 30.0 {
            let d = pts[n - 1] - pts[n - 2];
            let stub = (d.length() / 2.0).min(20.0);
            pts.insert(n - 1, pts[n - 1] - d.normalized() * stub);
        }
        j = jj;
        free = false;
    }
    let ends = (c.from, c.to);
    if let Some(c) = app.page_mut().conn_mut(id) { c.from = from; c.to = to; }
    app.drag = Drag::Seg { id, start: wp, pts, j, horizontal, free, before, ends };
}

fn start_move(app: &mut App, wp: Pos2, before: Document) {
    let page = app.page();
    let mut ids: Vec<Id> = app.sel_shapes().iter().filter(|s| !s.locked).map(|s| s.id).collect();
    // containers carry the shapes that sit inside them
    let conts: Vec<Rect> = app.sel_shapes().iter().filter(|s| library::is_container(&s.kind) && !s.locked).map(|s| geom::bbox(s)).collect();
    for s in &page.shapes {
        if ids.contains(&s.id) || s.locked { continue; }
        let b = geom::bbox(s);
        if conts.iter().any(|c| c.contains_rect(b)) { ids.push(s.id); }
    }
    let orig: Vec<(Id, f32, f32)> = ids.iter().filter_map(|id| page.shape(*id)).map(|s| (s.id, s.x, s.y)).collect();
    // selected connectors move whole; connectors between two moving shapes keep their manual route
    let moving = |e: &End| matches!(e, End::Glued { shape, .. } if ids.contains(shape));
    let free: Vec<(Id, End, End, Option<Vec<[f32; 2]>>)> = page
        .conns
        .iter()
        .filter(|c| app.sel.contains(&c.id) || (c.has_bends() && moving(&c.from) && moving(&c.to)))
        .map(|c| (c.id, c.from, c.to, c.bends.clone()))
        .collect();
    app.drag = Drag::Move { start: wp, orig, free, before, dup_done: false };
}

fn update_drag(app: &mut App, sp: Pos2, wp: Pos2, z: f32, shift: bool, cmd: bool) {
    let snap_on = app.set.snap;
    let g = app.set.grid_size;
    let guides_on = app.set.guides;
    let drag = std::mem::replace(&mut app.drag, Drag::None);
    let drag = match drag {
        Drag::Pan { start, pan0 } => { app.pan = pan0 + (sp - start); Drag::Pan { start, pan0 } }
        Drag::Move { start, orig, free, before, dup_done } => {
            let mut d = wp - start;
            if shift { if d.x.abs() > d.y.abs() { d.y = 0.0 } else { d.x = 0.0 } }
            app.guides.clear();
            let ids: Vec<Id> = orig.iter().map(|o| o.0).collect();
            let mut bb: Option<Rect> = None;
            for (id, x, y) in &orig {
                if let Some(s) = app.page().shape(*id) { let mut b = geom::bbox(s); b = b.translate(vec2(x - s.x, y - s.y)); bb = geom::rect_union(bb, b); }
            }
            if let Some(b0) = bb {
                let th = 6.0 / z;
                let others: Vec<Rect> = app.page().shapes.iter().filter(|s| !ids.contains(&s.id)).map(geom::bbox).collect();
                let mv = b0.translate(d);
                let mut snapped = (false, false);
                if guides_on {
                    let (mut bx, mut by) = (th, th);
                    let (mut gx, mut gy) = (None, None);
                    for o in &others {
                        for ov in [o.min.x, o.center().x, o.max.x] { for mvv in [mv.min.x, mv.center().x, mv.max.x] { let dd = ov - mvv; if dd.abs() < bx { bx = dd.abs(); gx = Some((dd, ov, *o)); } } }
                        for ov in [o.min.y, o.center().y, o.max.y] { for mvv in [mv.min.y, mv.center().y, mv.max.y] { let dd = ov - mvv; if dd.abs() < by { by = dd.abs(); gy = Some((dd, ov, *o)); } } }
                    }
                    if let Some((dd, x, o)) = gx { d.x += dd; snapped.0 = true; let m2 = b0.translate(d); app.guides.push((pos2(x, o.min.y.min(m2.min.y) - 10.0), pos2(x, o.max.y.max(m2.max.y) + 10.0))); }
                    if let Some((dd, y, o)) = gy { d.y += dd; snapped.1 = true; let m2 = b0.translate(d); app.guides.push((pos2(o.min.x.min(m2.min.x) - 10.0, y), pos2(o.max.x.max(m2.max.x) + 10.0, y))); }
                }
                if snap_on {
                    if !snapped.0 { d.x = geom::snap(b0.min.x + d.x, g / 2.0) - b0.min.x; }
                    if !snapped.1 { d.y = geom::snap(b0.min.y + d.y, g / 2.0) - b0.min.y; }
                }
            }
            let p = app.page_mut();
            for (id, x, y) in &orig { if let Some(s) = p.shape_mut(*id) { s.x = x + d.x; s.y = y + d.y; } }
            for (cid, f, t, bends) in &free {
                if let Some(c) = p.conn_mut(*cid) {
                    let mv = |e: End| match e { End::Free { x, y } => End::Free { x: x + d.x, y: y + d.y }, e => e };
                    c.from = mv(*f);
                    c.to = mv(*t);
                    c.bends = bends.as_ref().map(|b| b.iter().map(|p| [p[0] + d.x, p[1] + d.y]).collect());
                }
            }
            Drag::Move { start, orig, free, before, dup_done }
        }
        Drag::Resize { id, hx, hy, orig, before } => {
            let mut p = wp;
            if snap_on && orig.rot == 0.0 { p = pos2(geom::snap(p.x, g / 2.0), geom::snap(p.y, g / 2.0)); }
            let (ocx, ocy) = orig.center();
            let oc = pos2(ocx, ocy);
            let m = rot(p - oc, -orig.rot);
            let (mut l, mut r, mut t, mut b) = (-orig.w / 2.0, orig.w / 2.0, -orig.h / 2.0, orig.h / 2.0);
            if hx == 1 { r = m.x.max(l + 8.0); } else if hx == -1 { l = m.x.min(r - 8.0); }
            if hy == 1 { b = m.y.max(t + 8.0); } else if hy == -1 { t = m.y.min(b - 8.0); }
            let keep = shift || library::is_icon(&orig.kind) && hx != 0 && hy != 0 && !cmd;
            if keep && hx != 0 && hy != 0 {
                let ratio = orig.w / orig.h;
                let (nw, nh) = (r - l, b - t);
                if nw / nh > ratio { let nw2 = nh * ratio; if hx == 1 { r = l + nw2 } else { l = r - nw2 } } else { let nh2 = nw / ratio; if hy == 1 { b = t + nh2 } else { t = b - nh2 } }
            }
            let nc = oc + rot(vec2((l + r) / 2.0, (t + b) / 2.0), orig.rot);
            if let Some(s) = app.page_mut().shape_mut(id) {
                s.w = (r - l).round();
                s.h = (b - t).round();
                s.x = nc.x - s.w / 2.0;
                s.y = nc.y - s.h / 2.0;
            }
            Drag::Resize { id, hx, hy, orig, before }
        }
        Drag::MResize { hx, hy, bbox, orig, before } => {
            let (mut l, mut r, mut t, mut b) = (bbox.min.x, bbox.max.x, bbox.min.y, bbox.max.y);
            let p = if snap_on { pos2(geom::snap(wp.x, g / 2.0), geom::snap(wp.y, g / 2.0)) } else { wp };
            if hx == 1 { r = p.x.max(l + 10.0) } else if hx == -1 { l = p.x.min(r - 10.0) }
            if hy == 1 { b = p.y.max(t + 10.0) } else if hy == -1 { t = p.y.min(b - 10.0) }
            let (sx, sy) = ((r - l) / bbox.width(), (b - t) / bbox.height());
            let pg = app.page_mut();
            for o in &orig {
                if let Some(s) = pg.shape_mut(o.id) {
                    s.x = l + (o.x - bbox.min.x) * sx;
                    s.y = t + (o.y - bbox.min.y) * sy;
                    s.w = (o.w * sx).max(4.0);
                    s.h = (o.h * sy).max(4.0);
                }
            }
            Drag::MResize { hx, hy, bbox, orig, before }
        }
        Drag::Rotate { id, before } => {
            if let Some(s) = app.page_mut().shape_mut(id) {
                let (cx, cy) = s.center();
                let v = wp - pos2(cx, cy);
                let mut a = v.y.atan2(v.x).to_degrees() + 90.0;
                a = if shift { (a / 15.0).round() * 15.0 } else { a.round() };
                let near = (a / 90.0).round() * 90.0;
                if (a - near).abs() < 4.0 { a = near; }
                s.rot = a.rem_euclid(360.0);
            }
            Drag::Rotate { id, before }
        }
        Drag::Marquee { start, additive, .. } => Drag::Marquee { start, cur: wp, additive },
        Drag::Seg { id, start, pts, j, horizontal, free, before, ends } => {
            let d = wp - start;
            let mut q = pts.clone();
            if free {
                // straight connector: move one bend point anywhere
                let mut p = q[j] + d;
                if snap_on { p = pos2(geom::snap(p.x, g / 2.0), geom::snap(p.y, g / 2.0)); }
                q[j] = p;
            } else {
                // line up with the other parallel runs (and the end points) before snapping to grid,
                // so a nearly straight connector becomes exactly straight instead of a small jog
                let axis_of = |p: Pos2| if horizontal { p.y } else { p.x };
                let want = axis_of(q[j]) + if horizontal { d.y } else { d.x };
                let others: Vec<f32> = pts.iter().enumerate().filter(|(i, _)| *i != j && *i != j + 1).map(|(_, p)| axis_of(*p)).collect();
                let near = others.iter().copied().filter(|o| (o - want).abs() <= 8.0 / z).min_by(|a, b| (a - want).abs().total_cmp(&(b - want).abs()));
                let v = match near { Some(o) => o, None if snap_on => geom::snap(want, g / 2.0), None => want };
                let delta = v - axis_of(q[j]);
                if horizontal { q[j].y += delta; q[j + 1].y += delta; } else { q[j].x += delta; q[j + 1].x += delta; }
            }
            // never let the drag push the route into (or backwards out of) an end shape: stop at
            // the last position that is still a valid route, so what you see is what you keep
            let to_bends = |q: &[Pos2]| -> Vec<[f32; 2]> { q[1..q.len() - 1].iter().map(|p| [p.x, p.y]).collect() };
            let valid = |q: &[Pos2]| app.page().conn(id).map_or(true, |c| {
                let mut c2 = c.clone();
                c2.bends = Some(to_bends(q));
                geom::manual_route(app.page(), &c2).is_some()
            });
            if !valid(&q) && valid(&pts) {
                let (mut lo, mut hi) = (0.0f32, 1.0f32);
                for _ in 0..12 {
                    let mid = (lo + hi) / 2.0;
                    let t: Vec<Pos2> = pts.iter().zip(&q).map(|(a, b)| *a + (*b - *a) * mid).collect();
                    if valid(&t) { lo = mid } else { hi = mid }
                }
                q = pts.iter().zip(&q).map(|(a, b)| *a + (*b - *a) * lo).collect();
            }
            let bends = to_bends(&q);
            if let Some(c) = app.page_mut().conn_mut(id) { c.bends = Some(bends); }
            Drag::Seg { id, start, pts, j, horizontal, free, before, ends }
        }
        Drag::NewConn { id, before } => {
            let exclude = app.page().conn(id).and_then(|c| match c.from { End::Glued { shape, .. } => Some(shape), _ => None });
            let gl = find_glue(app.page(), wp, z, exclude);
            app.glue = gl;
            if let Some(c) = app.page_mut().conn_mut(id) { c.to = match gl { Some((s, p)) => End::Glued { shape: s, port: p }, None => End::Free { x: wp.x, y: wp.y } }; }
            Drag::NewConn { id, before }
        }
        Drag::ConnEnd { id, to, before } => {
            let other = app.page().conn(id).and_then(|c| match if to { c.from } else { c.to } { End::Glued { shape, .. } => Some(shape), _ => None });
            let gl = find_glue(app.page(), wp, z, other);
            app.glue = gl;
            if let Some(c) = app.page_mut().conn_mut(id) {
                let e = match gl { Some((s, p)) => End::Glued { shape: s, port: p }, None => End::Free { x: wp.x, y: wp.y } };
                if to { c.to = e } else { c.from = e }
            }
            Drag::ConnEnd { id, to, before }
        }
        d => d,
    };
    app.drag = drag;
}

fn cancel_drag(app: &mut App) {
    let drag = std::mem::replace(&mut app.drag, Drag::None);
    let before = match drag {
        Drag::Move { before, .. } | Drag::Resize { before, .. } | Drag::MResize { before, .. } | Drag::Rotate { before, .. }
        | Drag::ConnEnd { before, .. } | Drag::Seg { before, .. } | Drag::NewConn { before, .. } => before,
        d => { app.drag = d; return; }
    };
    let sel = app.sel.clone();
    app.doc = before;
    let page = app.page();
    app.sel = sel.into_iter().filter(|id| page.shape(*id).is_some() || page.conn(*id).is_some()).collect();
    app.guides.clear();
    app.glue = None;
}

/// Manual routes that no longer make sense after shapes moved go back to automatic routing.
fn drop_stale_routes(app: &mut App) {
    let page = app.page();
    let stale: Vec<Id> = page.conns.iter().filter(|c| c.has_bends() && geom::manual_route(page, c).is_none()).map(|c| c.id).collect();
    for id in stale { if let Some(c) = app.page_mut().conn_mut(id) { c.bends = None; } }
}

fn end_drag(app: &mut App, z: f32) {
    let drag = std::mem::replace(&mut app.drag, Drag::None);
    match drag {
        Drag::Move { before, .. } | Drag::Resize { before, .. } | Drag::MResize { before, .. } | Drag::Rotate { before, .. } | Drag::ConnEnd { before, .. } => {
            drop_stale_routes(app);
            app.push_before(before);
        }
        Drag::Seg { id, before, ends, .. } => {
            // tidy: drop bends that ended up collinear with their neighbours
            let page = app.page();
            if let Some(c) = page.conn(id) {
                let r = geom::route(page, c);
                let tidy: Vec<[f32; 2]> = r[1..r.len().saturating_sub(1)].iter().map(|p| [p.x, p.y]).collect();
                // a manual route that ends up within a few pixels of the automatic one is just noise:
                // drop it, so the automatic (straightened) route takes over
                let auto = { let mut c2 = c.clone(); c2.bends = None; c2.from = ends.0; c2.to = ends.1; geom::route(page, &c2) };
                let close = |p: &[Pos2], q: &[Pos2]| p.iter().all(|x| geom::dist_poly(*x, q) <= 6.0);
                let redundant = close(&r, &auto) && close(&auto, &r);
                let keep = c.routing == Routing::Orthogonal && geom::manual_route(page, c).is_some() && !redundant;
                let invalid = geom::manual_route(page, c).is_none() || redundant;
                if let Some(c) = app.page_mut().conn_mut(id) {
                    // ends were pinned only for the drag; give back the original (usually dynamic) glue
                    c.from = ends.0;
                    c.to = ends.1;
                    if keep { c.bends = Some(tidy); } else if invalid { c.bends = None; }
                }
            }
            app.push_before(before);
        }
        Drag::NewConn { id, before } => {
            let tiny = app.page().conn(id).map_or(true, |c| { let r = geom::route(app.page(), c); geom::poly_len(&r) < 12.0 / z && !matches!(c.to, End::Glued { .. }) });
            if tiny { app.page_mut().conns.retain(|c| c.id != id); app.sel.clear(); } else { app.sel = vec![id]; app.push_before(before); }
        }
        Drag::Marquee { start, cur, additive } => {
            let r = Rect::from_two_pos(start, cur);
            let page = app.page();
            let mut ids: Vec<Id> = page.shapes.iter().filter(|s| page.shape_editable(s) && r.contains_rect(geom::bbox(s))).map(|s| s.id).collect();
            ids.extend(page.conns.iter().filter(|c| page.conn_editable(c)).filter(|c| geom::route(page, c).iter().all(|p| r.contains(*p))).map(|c| c.id));
            if additive { for id in ids { if !app.sel.contains(&id) { app.sel.push(id); } } } else { app.sel = ids; }
            app.expand_groups();
        }
        _ => {}
    }
    app.guides.clear();
    app.glue = None;
}

fn click(app: &mut App, hit: Hit, wp: Pos2, shift: bool) {
    if let (Some(st), Hit::Shape(id)) = (app.painter_style.clone(), hit) {
        app.checkpoint();
        if let Some(s) = app.page_mut().shape_mut(id) { s.style = st; }
        app.painter_style = None;
        return;
    }
    match app.tool {
        Tool::Text => {
            match hit {
                Hit::Shape(id) => start_edit(app, id, false, None),
                _ => {
                    let id = app.add_shape("text", wp);
                    if let Some(s) = app.page_mut().shape_mut(id) { s.text.clear(); }
                    start_edit(app, id, false, Some(String::new()));
                }
            }
            app.tool = Tool::Pointer;
            return;
        }
        Tool::Pan => return,
        _ => {}
    }
    match hit {
        Hit::Link(id) => { app.open_link(id); }
        Hit::Auto(id, d) => app.auto_connect(id, d),
        Hit::Shape(id) | Hit::Conn(id) => {
            if shift {
                if app.is_sel(id) { app.sel.retain(|x| *x != id); } else { app.sel.push(id); app.expand_groups(); }
            } else if !(app.is_sel(id) && app.sel.len() > 1) || true {
                app.select_only(id);
            }
        }
        Hit::None => if !shift { app.sel.clear(); },
        _ => {}
    }
}

// ============================================================ painting

fn paint(app: &mut App, ui: &Ui, painter: &egui::Painter, v: &View, crect: Rect) {
    let pal = app.pal;
    let z = v.zoom;
    painter.rect_filled(crect, 0.0, pal.canvas);
    let live_page = crate::live::apply(app.page(), &app.live);
    let page: &Page = &live_page;
    let page_bg = page.bg.map(to_c32).unwrap_or(pal.page);
    let pr = v.rect(Rect::from_min_size(Pos2::ZERO, vec2(page.width, page.height)));
    for i in 1..=4 {
        let f = i as f32;
        painter.rect_filled(pr.translate(vec2(0.0, f)).expand(f), CornerRadius::same(i as u8), Color32::from_black_alpha(if pal.dark { 30 } else { 7 }));
    }
    painter.rect_filled(pr, 0.0, page_bg);
    painter.rect_stroke(pr, 0.0, Stroke::new(1.0, pal.border), StrokeKind::Outside);

    if app.set.grid {
        let g = app.set.grid_size;
        let vis = pr.intersect(crect);
        let mut step = g;
        while step * z < 7.0 { step *= 5.0; }
        let major = step * 5.0;
        let first_x = ((v.w(vis.min).x / step).floor() * step).max(0.0);
        let first_y = ((v.w(vis.min).y / step).floor() * step).max(0.0);
        let mut x = first_x;
        while x <= page.width && v.s(pos2(x, 0.0)).x <= vis.max.x {
            let sx = v.s(pos2(x, 0.0)).x;
            let c = if (x / major).fract().abs() < 1e-4 { pal.grid_major } else { pal.grid_minor };
            painter.line_segment([pos2(sx, pr.min.y), pos2(sx, pr.max.y)], Stroke::new(1.0, c));
            x += step;
        }
        let mut y = first_y;
        while y <= page.height && v.s(pos2(0.0, y)).y <= vis.max.y {
            let sy = v.s(pos2(0.0, y)).y;
            let c = if (y / major).fract().abs() < 1e-4 { pal.grid_major } else { pal.grid_minor };
            painter.line_segment([pos2(pr.min.x, sy), pos2(pr.max.x, sy)], Stroke::new(1.0, c));
            y += step;
        }
    }

    let tex: HashMap<Id, egui::TextureId> = app.images.iter().map(|(k, (_, t))| (*k, t.id())).collect();
    let cx = Ctx { painter, view: *v, pal: &pal, images: &tex, doc: Some(&app.doc), depth: 0, time: Some(app.anim_time(ui.ctx())) };
    let mut out: Vec<EShape> = Vec::with_capacity(512);
    let editing = app.edit.as_ref().map(|e| (e.id, e.conn));
    let vis_world = Rect::from_min_max(v.w(crect.min), v.w(crect.max)).expand(60.0);
    for s in &page.shapes {
        if !page.shape_shown(s) || !geom::bbox(s).intersects(vis_world) { continue; }
        render::draw_shape(&cx, &mut out, s, editing != Some((s.id, false)) && !render::label_drawn_last(s));
    }
    for (ci, route) in render::page_routes(page) {
        let c = &page.conns[ci];
        render::draw_conn(&cx, &mut out, c, route, page_bg, editing != Some((c.id, true)));
    }
    for s in page.shapes.iter().filter(|s| render::label_drawn_last(s) && editing != Some((s.id, false))) {
        if !page.shape_shown(s) || !geom::bbox(s).expand(60.0).intersects(vis_world) { continue; }
        out.extend(render::masked_label(&cx, s, page_bg));
    }
    painter.extend(out);
    if app.set.animate && crate::anim::page_moves(page) { ui.ctx().request_repaint_after(std::time::Duration::from_millis(33)); }

    // ---------------- sub-diagram badges
    let hp = ui.input(|i| i.pointer.hover_pos());
    for s in page.shapes.iter().filter(|s| page.shape_shown(s) && badge_visible(s, z)) {
        let Some(target) = app.doc.linked_page(s) else { continue };
        let c = v.s(App::link_badge_pos(s, z));
        if !crect.contains(c) { continue; }
        let hot = hp.map_or(false, |p| (p - c).length() <= 9.0);
        let r = if hot { 8.5 } else { 7.5 };
        painter.circle_filled(c + vec2(0.0, 1.0), r, Color32::from_black_alpha(40));
        painter.circle_filled(c, r, if hot { pal.accent } else { with_alpha(pal.accent, 0.88) });
        // ↗ "open" glyph
        let st = Stroke::new(1.5, Color32::WHITE);
        let k = 3.0;
        painter.line_segment([c + vec2(-k, k), c + vec2(k, -k)], st);
        painter.line_segment([c + vec2(k, -k), c + vec2(-0.5, -k)], st);
        painter.line_segment([c + vec2(k, -k), c + vec2(k, 0.5)], st);
        if hot {
            let g = painter.layout_no_wrap(crate::platform::keys(&format!("Open “{}”  ⌘↓", target.name)), FontId::proportional(11.5), pal.text);
            let r = Rect::from_min_size(c + vec2(13.0, -g.size().y / 2.0 - 4.0), g.size() + vec2(14.0, 8.0));
            painter.rect_filled(r.translate(vec2(0.0, 1.5)), 6.0, Color32::from_black_alpha(30));
            painter.rect_filled(r, 6.0, pal.panel);
            painter.rect_stroke(r, 6.0, Stroke::new(1.0, pal.border), StrokeKind::Inside);
            painter.galley(r.min + vec2(7.0, 4.0), g, pal.text);
        }
    }

    // ---------------- overlay
    let sel_c = pal.sel;
    let hs = 4.0;
    let handle = |p: Pos2| {
        painter.rect_filled(Rect::from_center_size(p, vec2(hs * 2.0, hs * 2.0)), 1.5, pal.panel);
        painter.rect_stroke(Rect::from_center_size(p, vec2(hs * 2.0, hs * 2.0)), 1.5, Stroke::new(1.3, sel_c), StrokeKind::Middle);
    };
    // hover outline + ports + AutoConnect
    let idle = matches!(app.drag, Drag::None);
    let show_ports = matches!(app.tool, Tool::Pointer | Tool::Connector) && (idle || matches!(app.drag, Drag::NewConn { .. } | Drag::ConnEnd { .. }));
    if let Some(h) = app.hover.filter(|_| show_ports) {
        if let Some(s) = page.shape(h) {
            if !app.is_sel(h) {
                let pts: Vec<Pos2> = [pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)].iter().map(|p| v.s(geom::l2w(s, *p))).collect();
                painter.add(EShape::closed_line(pts, Stroke::new(1.0, with_alpha(sel_c, 0.55))));
            }
            if !s.locked {
                for p in geom::ports(s) {
                    let c = v.s(p.pos);
                    let k = 2.6;
                    let col = with_alpha(sel_c, 0.85);
                    painter.line_segment([c + vec2(-k, -k), c + vec2(k, k)], Stroke::new(1.3, col));
                    painter.line_segment([c + vec2(-k, k), c + vec2(k, -k)], Stroke::new(1.3, col));
                }
            }
            if app.tool == Tool::Pointer && idle && !library::is_container(&s.kind) && s.kind != "phase" && app.sel.len() <= 1 && !s.locked {
                let hp = ui.input(|i| i.pointer.hover_pos());
                for d in DIRS {
                    let c = v.s(auto_pos(s, d, z));
                    let hot = hp.map_or(false, |p| (p - c).length() < 9.0);
                    let n = vec2(-d.y, d.x);
                    let a = if hot { 1.0 } else { 0.5 };
                    painter.add(EShape::convex_polygon(vec![c + d * 6.0, c - d * 4.0 + n * 6.0, c - d * 4.0 - n * 6.0], with_alpha(sel_c, a), Stroke::NONE));
                }
            }
        }
    }
    // glue target
    if let Some((sid, port)) = app.glue {
        if let Some(s) = page.shape(sid) {
            if port >= 0 {
                if let Some(p) = geom::ports(s).get(port as usize) {
                    painter.circle_filled(v.s(p.pos), 7.0, with_alpha(pal.glue, 0.22));
                    painter.circle_filled(v.s(p.pos), 3.5, pal.glue);
                }
            } else {
                let pts: Vec<Pos2> = [pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)].iter().map(|p| v.s(geom::l2w(s, *p))).collect();
                painter.add(EShape::closed_line(pts, Stroke::new(2.0, with_alpha(pal.glue, 0.8))));
            }
        }
    }
    // selection
    let ss = app.sel_shapes();
    for s in &ss {
        let pts: Vec<Pos2> = [pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)].iter().map(|p| v.s(geom::l2w(s, *p))).collect();
        let st = Stroke::new(1.2, sel_c);
        let mut closed = pts.clone();
        closed.push(pts[0]);
        painter.extend(EShape::dashed_line(&closed, st, 4.0, 3.0));
        if s.locked {
            let c = pts[1] + vec2(8.0, -8.0);
            crate::icons::get().paint(painter, "lock", Rect::from_center_size(c, vec2(13.0, 13.0)), pal.muted);
        }
    }
    if ss.len() == 1 && !ss[0].locked {
        let s = ss[0];
        let rh = v.s(rot_handle(s, z));
        if s.kind != "phase" {
            painter.line_segment([v.s(handle_pos(s, 0, -1)), rh], Stroke::new(1.0, sel_c));
            painter.circle_filled(rh, 5.0, sel_c);
            painter.circle_stroke(rh, 5.0, Stroke::new(1.5, pal.panel));
        }
        for hy in -1..=1 { for hx in -1..=1 { if (hx, hy) != (0, 0) { handle(v.s(handle_pos(s, hx, hy))); } } }
    } else if ss.len() > 1 {
        if let Some(b) = app.sel_bbox() {
            let r = v.rect(b);
            painter.rect_stroke(r, 0.0, Stroke::new(1.2, sel_c), StrokeKind::Middle);
            for hy in -1..=1 { for hx in -1..=1 { if (hx, hy) != (0, 0) { handle(pos2(r.center().x + hx as f32 * r.width() / 2.0, r.center().y + hy as f32 * r.height() / 2.0)); } } }
        }
    }
    let hp = ui.input(|i| i.pointer.hover_pos());
    let only_conn = app.sel_conns().len() == 1 && app.sel_shapes().is_empty();
    for c in app.sel_conns() {
        let rw = geom::route(page, c);
        let r: Vec<Pos2> = rw.iter().map(|p| v.s(*p)).collect();
        // soft highlight just wider than the line
        painter.add(EShape::line(r.clone(), Stroke::new(c.style.width.unwrap_or(1.5) * z + 3.5, with_alpha(sel_c, 0.18))));
        // segment handles: small pills along each segment (drag to move the segment)
        if only_conn {
            let dragging = matches!(app.drag, Drag::Seg { .. });
            for (k, m) in seg_handles(c, &rw, z) {
                let (a, b) = (r[k], r[k + 1]);
                let c0 = v.s(m);
                let hot = dragging || hp.map_or(false, |p| (p - c0).length() <= 7.0);
                let col = if hot { sel_c } else { with_alpha(sel_c, 0.9) };
                if c.routing == Routing::Orthogonal {
                    // a small pill lying along the segment
                    let horiz = (a.y - b.y).abs() < 0.5;
                    let (len, th) = (if hot { 14.0 } else { 12.0 }, if hot { 5.0 } else { 4.0 });
                    let size = if horiz { vec2(len, th) } else { vec2(th, len) };
                    let pill = Rect::from_center_size(c0, size);
                    painter.rect_filled(pill.expand(1.25), th, pal.panel);
                    painter.rect_filled(pill, th / 2.0, col);
                } else {
                    painter.circle_filled(c0, if hot { 4.5 } else { 3.75 }, pal.panel);
                    painter.circle_filled(c0, if hot { 3.5 } else { 2.75 }, col);
                }
            }
        }
        // end points: small dots with a soft halo (green = glued, hollow = free)
        for (p, e) in [(r.first(), c.from), (r.last(), c.to)] {
            if let Some(p) = p {
                let glued = matches!(e, End::Glued { .. });
                let hot = hp.map_or(false, |q| (q - *p).length() <= 7.0);
                let col = if glued { pal.glue } else { sel_c };
                painter.circle_filled(*p, if hot { 7.0 } else { 6.0 }, with_alpha(col, 0.18));
                if glued {
                    painter.circle_filled(*p, 3.5, col);
                    painter.circle_stroke(*p, 3.5, Stroke::new(1.0, with_alpha(pal.panel, 0.9)));
                } else {
                    painter.circle_filled(*p, 3.2, pal.panel);
                    painter.circle_stroke(*p, 3.2, Stroke::new(1.3, col));
                }
            }
        }
    }
    for (a, b) in &app.guides { painter.line_segment([v.s(*a), v.s(*b)], Stroke::new(1.0, pal.guide)); }
    if let Drag::Marquee { start, cur, .. } = &app.drag {
        let r = Rect::from_two_pos(v.s(*start), v.s(*cur));
        painter.rect_filled(r, 0.0, with_alpha(sel_c, 0.08));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, sel_c), StrokeKind::Middle);
    }
    if let Some(_) = app.painter_style {
        if let Some(p) = ui.input(|i| i.pointer.hover_pos()) { crate::icons::get().paint(painter, "painter", Rect::from_min_size(p + vec2(14.0, 14.0), vec2(16.0, 16.0)), pal.accent); }
    }
    if page.shapes.is_empty() && page.conns.is_empty() {
        painter.text(crect.center() - vec2(0.0, 14.0), Align2::CENTER_CENTER, "Drag shapes from the left panel onto the page", FontId::proportional(15.0), pal.faint);
        painter.text(crect.center() + vec2(0.0, 10.0), Align2::CENTER_CENTER, "or click a shape to drop it in the centre", FontId::proportional(12.5), pal.faint);
    }
}

fn paint_rulers(app: &App, ui: &Ui, full: Rect, crect: Rect, v: &View) {
    let pal = app.pal;
    let p = ui.painter_at(full);
    let top = Rect::from_min_max(pos2(crect.min.x, full.min.y), pos2(full.max.x, crect.min.y));
    let left = Rect::from_min_max(pos2(full.min.x, crect.min.y), pos2(crect.min.x, full.max.y));
    p.rect_filled(top, 0.0, pal.ruler);
    p.rect_filled(left, 0.0, pal.ruler);
    p.rect_filled(Rect::from_min_max(full.min, crect.min), 0.0, pal.ruler);
    let z = v.zoom;
    // selection extent
    if let Some(b) = app.sel_bbox() {
        let r = v.rect(b);
        p.rect_filled(Rect::from_x_y_ranges(r.min.x.max(top.min.x)..=r.max.x.min(top.max.x), top.y_range()), 0.0, with_alpha(pal.accent, 0.14));
        p.rect_filled(Rect::from_x_y_ranges(left.x_range(), r.min.y.max(left.min.y)..=r.max.y.min(left.max.y)), 0.0, with_alpha(pal.accent, 0.14));
    }
    let steps = [5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0];
    let step = *steps.iter().find(|s| *s * z >= 64.0).unwrap_or(&5000.0);
    let minor = step / 5.0;
    let font = FontId::proportional(9.0);
    let tc = pal.ruler_text;
    let x0 = (v.w(top.min).x / minor).floor() * minor;
    let mut x = x0;
    while v.s(pos2(x, 0.0)).x < top.max.x {
        let sx = v.s(pos2(x, 0.0)).x;
        let is_major = (x / step).round() * step == x || ((x / step).fract().abs() < 1e-3);
        let h = if is_major { 10.0 } else { 4.0 };
        if sx >= top.min.x { p.line_segment([pos2(sx, top.max.y - h), pos2(sx, top.max.y)], Stroke::new(1.0, tc)); }
        if is_major && sx >= top.min.x { p.text(pos2(sx + 3.0, top.min.y + 1.0), Align2::LEFT_TOP, format!("{}", x as i64), font.clone(), tc); }
        x += minor;
    }
    let y0 = (v.w(left.min).y / minor).floor() * minor;
    let mut y = y0;
    while v.s(pos2(0.0, y)).y < left.max.y {
        let sy = v.s(pos2(0.0, y)).y;
        let is_major = (y / step).fract().abs() < 1e-3;
        let w = if is_major { 10.0 } else { 4.0 };
        if sy >= left.min.y { p.line_segment([pos2(left.max.x - w, sy), pos2(left.max.x, sy)], Stroke::new(1.0, tc)); }
        if is_major && sy >= left.min.y {
            let g = p.layout_no_wrap(format!("{}", y as i64), font.clone(), tc);
            p.add(egui::epaint::TextShape::new(pos2(left.min.x + 2.0, sy + 3.0 + g.size().x), g, tc).with_angle(-std::f32::consts::FRAC_PI_2));
        }
        y += minor;
    }
    if let Some(m) = app.mouse_world {
        let s = v.s(m);
        p.line_segment([pos2(s.x, top.min.y), pos2(s.x, top.max.y)], Stroke::new(1.0, pal.accent));
        p.line_segment([pos2(left.min.x, s.y), pos2(left.max.x, s.y)], Stroke::new(1.0, pal.accent));
    }
    p.line_segment([pos2(top.min.x, top.max.y - 0.5), pos2(top.max.x, top.max.y - 0.5)], Stroke::new(1.0, pal.border));
    p.line_segment([pos2(left.max.x - 0.5, left.min.y), pos2(left.max.x - 0.5, left.max.y)], Stroke::new(1.0, pal.border));
}

fn edit_overlay(app: &mut App, ui: &mut Ui, v: &View) {
    let Some(e) = app.edit.as_ref() else { return };
    let (id, conn) = (e.id, e.conn);
    let page = app.page();
    let (rect, fs, align) = if conn {
        let Some(c) = page.conn(id) else { app.edit = None; return };
        let mid = v.s(geom::poly_mid(&geom::route(page, c)));
        (Rect::from_center_size(mid, vec2(180.0, 30.0)), c.style.font_size.unwrap_or(12.0) * v.zoom, egui::Align::Center)
    } else {
        let Some(s) = page.shape(id) else { app.edit = None; return };
        let (mut r, _, rotated) = render::text_screen_box(v, s);
        if rotated || s.rot != 0.0 { let c = v.s(pos2(s.center().0, s.center().1)); r = Rect::from_center_size(c, vec2((s.w * v.zoom).max(140.0), 60.0)); }
        let r = Rect::from_center_size(r.center(), vec2(r.width().max(120.0), r.height().max(30.0)));
        let a = match s.style.align.unwrap_or(HAlign::Center) { HAlign::Left => egui::Align::LEFT, HAlign::Center => egui::Align::Center, HAlign::Right => egui::Align::RIGHT };
        (r, s.style.font_size.unwrap_or(13.0) * v.zoom, a)
    };
    let pal = app.pal;
    let mut commit = false;
    let mut cancel = false;
    egui::Area::new(EId::new("naqsha-editor")).fixed_pos(rect.min).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
        egui::Frame::new().fill(pal.panel).stroke(Stroke::new(1.5, pal.sel)).corner_radius(4).inner_margin(4).show(ui, |ui| {
            let e = app.edit.as_mut().unwrap();
            let te = egui::TextEdit::multiline(&mut e.buf)
                .desired_width(rect.width() - 8.0)
                .desired_rows(1)
                .frame(egui::Frame::NONE)
                .font(FontId::proportional(fs.clamp(9.0, 64.0)))
                .horizontal_align(align)
                .text_color(pal.text);
            let r = ui.add_sized(vec2(rect.width() - 8.0, (rect.height() - 8.0).max(fs * 1.4)), te);
            if e.focus { r.request_focus(); e.focus = false; }
            if ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command) { commit = true; }
            if ui.input(|i| i.key_pressed(Key::Escape)) { commit = true; }
            if r.lost_focus() { commit = true; }
            let _ = &mut cancel;
        });
    });
    if commit { commit_edit(app); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Settings;

    /// Drive a segment drag like the canvas would, at zoom 1 (screen == world).
    fn drag_seg(a: &mut App, id: Id, k: usize, by: Vec2) {
        let r = geom::route(a.page(), a.page().conn(id).unwrap());
        let m = r[k] + (r[k + 1] - r[k]) * 0.5;
        let before = a.doc.clone();
        begin_drag(a, Hit::Seg(id, k), m, false, false);
        assert!(matches!(a.drag, Drag::Seg { .. }));
        for i in 1..=4 { let p = m + by * (i as f32 / 4.0); update_drag(a, p, p, 1.0, false, false); }
        end_drag(a, 1.0);
        let _ = before;
    }

    fn setup(kind: &str) -> (App, Id) {
        let mut a = App::new(Settings::default(), None);
        a.set.snap = false;
        let s1 = a.add_shape("rrect", pos2(160.0, 120.0));
        let s2 = a.add_shape(kind, pos2(520.0, 420.0));
        let id = a.doc.new_id();
        a.page_mut().conns.push(Conn { id, from: End::Glued { shape: s1, port: -1 }, to: End::Glued { shape: s2, port: -1 }, routing: Routing::Orthogonal, ..Default::default() });
        a.sel = vec![id];
        (a, id)
    }

    #[test]
    fn segments_stay_where_dragged_for_any_shape() {
        let mut fails = vec![];
        for kind in ["rtriangle", "triangle", "rect", "ellipse", "diamond", "hexagon"] {
            for port in [-1, 0, 1, 2, 3] {
                let mk = || {
                    let (mut a, id) = setup(kind);
                    if let Some(c) = a.page_mut().conn_mut(id) { if let End::Glued { shape, .. } = c.to { c.to = End::Glued { shape, port }; } }
                    (a, id)
                };
                let (a0, id) = mk();
                let n = geom::route(a0.page(), a0.page().conn(id).unwrap()).len();
                for k in 0..n - 1 {
                    for sign in [1.0, -1.0] {
                        let (mut b, id) = mk();
                        let r = geom::route(b.page(), b.page().conn(id).unwrap());
                        if (r[k + 1] - r[k]).length() < 22.0 { continue; }
                        let horiz = (r[k].y - r[k + 1].y).abs() < 0.5;
                        let by = if horiz { vec2(0.0, 40.0 * sign) } else { vec2(40.0 * sign, 0.0) };
                        drag_seg(&mut b, id, k, by);
                        if !b.page().conn(id).unwrap().has_bends() { fails.push(format!("{kind} port {port} seg {k} by {by:?}: route {r:?}")); }
                    }
                }
            }
        }
        assert!(fails.is_empty(), "{} dropped edits:\n{}", fails.len(), fails.join("\n"));
    }

    /// Problems that make a route hard to fix by hand: a leg that doubles back over the previous
    /// one (draws as a spike), a slanted leg in an orthogonal connector, or a leg too short to grab.
    fn route_defects(r: &[Pos2]) -> Vec<String> {
        let mut out = vec![];
        for w in r.windows(2) {
            let d = w[1] - w[0];
            if d.x.abs() > 0.5 && d.y.abs() > 0.5 { out.push(format!("slanted leg {:?}->{:?}", w[0], w[1])); }
            if d.length() < 3.0 && r.len() > 2 { out.push(format!("tiny leg {:?}->{:?}", w[0], w[1])); }
        }
        for w in r.windows(3) {
            let (u, v) = (w[1] - w[0], w[2] - w[1]);
            if u.length() > 0.01 && v.length() > 0.01 && u.normalized().dot(v.normalized()) < -0.99 { out.push(format!("doubles back at {:?}", w[1])); }
        }
        out
    }

    #[test]
    fn segment_drags_never_leave_spikes_or_slants() {
        let mut fails = vec![];
        let layouts: [(&str, Pos2, &str, Pos2); 4] = [
            ("rrect", pos2(160.0, 100.0), "rect", pos2(460.0, 520.0)),
            ("rrect", pos2(160.0, 100.0), "rtriangle", pos2(520.0, 420.0)),
            ("rect", pos2(400.0, 100.0), "rect", pos2(180.0, 420.0)),
            ("rect", pos2(160.0, 300.0), "rect", pos2(520.0, 300.0)),
        ];
        for (ka, pa, kb, pb) in layouts {
            for port in [-1, 0, 1, 2, 3] {
                let mk = || {
                    let mut a = App::new(Settings::default(), None);
                    a.set.snap = false;
                    let s1 = a.add_shape(ka, pa);
                    let s2 = a.add_shape(kb, pb);
                    let id = a.doc.new_id();
                    a.page_mut().conns.push(Conn { id, from: End::Glued { shape: s1, port: -1 }, to: End::Glued { shape: s2, port }, routing: Routing::Orthogonal, ..Default::default() });
                    a.sel = vec![id];
                    (a, id)
                };
                let (a0, id) = mk();
                let n = geom::route(a0.page(), a0.page().conn(id).unwrap()).len();
                for k in 0..n - 1 {
                    for off in [-300.0, -150.0, -60.0, -25.0, -8.0, 8.0, 25.0, 60.0, 150.0, 300.0f32] {
                        for along in [false, true] {
                            // a second drag on top of the first, as people do when adjusting
                            let (mut b, id) = mk();
                            let r = geom::route(b.page(), b.page().conn(id).unwrap());
                            let horiz = (r[k].y - r[k + 1].y).abs() < 0.5;
                            let by = if horiz != along { vec2(0.0, off) } else { vec2(off, 0.0) };
                            drag_seg(&mut b, id, k, by);
                            let r1 = geom::route(b.page(), b.page().conn(id).unwrap());
                            let mut d = route_defects(&r1);
                            for k2 in 0..r1.len() - 1 {
                                if !d.is_empty() { break; }
                                let h2 = (r1[k2].y - r1[k2 + 1].y).abs() < 0.5;
                                let (mut c, _) = (std::mem::replace(&mut b, App::new(Settings::default(), None)), 0);
                                let snap = c.doc.clone();
                                drag_seg(&mut c, id, k2, if h2 { vec2(0.0, -off / 2.0) } else { vec2(-off / 2.0, 0.0) });
                                let r2 = geom::route(c.page(), c.page().conn(id).unwrap());
                                d = route_defects(&r2).into_iter().map(|x| format!("2nd drag seg {k2}: {x}; route {r2:?}")).collect();
                                c.doc = snap;
                                b = c;
                            }
                            if !d.is_empty() { fails.push(format!("{ka}->{kb} port {port} seg {k} by {by:?}: {d:?}\n   route {r1:?}")); }
                        }
                    }
                }
            }
        }
        assert!(fails.is_empty(), "{} bad routes:\n{}", fails.len(), fails[..fails.len().min(15)].join("\n"));
    }

    #[test]
    fn automatic_routes_have_no_spikes() {
        let mut fails = vec![];
        for avoid in [false, true] {
            for pa in 0..5 { for pb in 0..5 {
                for dx in [-300.0, -150.0, -138.0, -120.0, -40.0, -6.0, 0.0, 6.0, 40.0, 120.0, 138.0, 150.0, 300.0f32] { for dy in [-300.0, -120.0, -100.0, -84.0, -40.0, -5.0, 0.0, 5.0, 40.0, 84.0, 100.0, 120.0, 300.0f32] {
                    if dx.abs() < 130.0 && dy.abs() < 80.0 { continue; }
                    let mut a = App::new(Settings::default(), None);
                    a.page_mut().avoid_shapes = avoid;
                    let s1 = a.add_shape("rect", pos2(500.0, 500.0));
                    let s2 = a.add_shape("rect", pos2(500.0 + dx, 500.0 + dy));
                    let c = Conn { id: 999, from: End::Glued { shape: s1, port: pa - 1 }, to: End::Glued { shape: s2, port: pb - 1 }, routing: Routing::Orthogonal, ..Default::default() };
                    let r = geom::route(a.page(), &c);
                    let d = route_defects(&r);
                    if !d.is_empty() { fails.push(format!("avoid {avoid} ports {} {} at {dx},{dy}: {d:?} {r:?}", pa - 1, pb - 1)); }
                } }
            } }
        }
        assert!(fails.is_empty(), "{} bad:\n{}", fails.len(), fails[..fails.len().min(10)].join("\n"));
    }
}
