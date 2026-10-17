//! Painting shapes and connectors into egui.
use crate::geom::{self, l2w};
use crate::library::{self, PK};
use crate::model::*;
use crate::theme::{to_c32, Pal};
use egui::epaint::{PathShape, PathStroke, TextShape, Vertex};
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align, Color32, FontFamily, FontId, Galley, Mesh, Painter, Pos2, Rect, Shape as EShape, Stroke, TextureId, Vec2};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub origin: Pos2,
    pub pan: Vec2,
    pub zoom: f32,
}

impl View {
    pub fn s(&self, p: Pos2) -> Pos2 { self.origin + self.pan + p.to_vec2() * self.zoom }
    pub fn w(&self, p: Pos2) -> Pos2 { ((p - self.origin - self.pan) / self.zoom).to_pos2() }
    pub fn rect(&self, r: Rect) -> Rect { Rect::from_min_max(self.s(r.min), self.s(r.max)) }
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, aa] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, ((aa as f32) * a).round() as u8)
}

pub fn fill_of(s: &Shape, pal: &Pal) -> Color32 {
    let c = match s.style.fill { Some(c) => to_c32(c), None => if library::is_container(&s.kind) { pal.cont_fill } else { pal.shape_fill } };
    with_alpha(c, s.style.opacity.unwrap_or(1.0))
}
pub fn stroke_of(st: &Style, default: Color32) -> Color32 {
    let c = st.stroke.map(to_c32).unwrap_or(default);
    with_alpha(c, st.opacity.unwrap_or(1.0))
}

fn luminance(c: Color32) -> f32 {
    let f = |v: u8| { let x = v as f32 / 255.0; if x <= 0.03928 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) } };
    0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
}
/// WCAG contrast ratio (1 = invisible, 21 = black on white).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}
fn over(top: Color32, under: Color32) -> Color32 {
    let [r, g, b, a] = top.to_srgba_unmultiplied();
    let t = a as f32 / 255.0;
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    Color32::from_rgb(m(r, under.r()), m(g, under.g()), m(b, under.b()))
}

/// What a shape's text is drawn on: its own fill when the text sits inside a filled shape,
/// otherwise the page.
fn text_backdrop(s: &Shape, pal: &Pal) -> Color32 {
    let filled = !library::is_icon(&s.kind) && s.kind != "image" && !library::is_textlike(&s.kind)
        && library::shape_parts(s).iter().any(|p| p.closed && matches!(p.kind, PK::Main | PK::Fill));
    let inside = match s.tbox {
        Some([x, y, w, h]) => { let (cx, cy) = (x + w / 2.0, y + h / 2.0); cx >= 0.0 && cx <= s.w && cy >= 0.0 && cy <= s.h }
        None => true,
    };
    if filled && inside { over(fill_of(s, pal), pal.page) } else { pal.page }
}

/// Text colour of a shape, kept readable: text with no colour of its own picks dark or light
/// to suit what it sits on (imported diagrams often have light fills and no font colour, which
/// would otherwise come out light-on-light in dark mode), and an explicit colour that would be
/// practically invisible on its backdrop is swapped for one that can be read.
pub fn shape_text_color(s: &Shape, pal: &Pal) -> Color32 {
    let bg = text_backdrop(s, pal);
    let dark_ink = Color32::from_rgb(0x1A, 0x1A, 0x2E);
    let light_ink = Color32::from_rgb(0xF2, 0xF2, 0xF8);
    let best = if contrast(dark_ink, bg) >= contrast(light_ink, bg) { dark_ink } else { light_ink };
    match s.style.text {
        Some(c) => {
            let c = with_alpha(to_c32(c), s.style.opacity.unwrap_or(1.0));
            if contrast(over(c, bg), bg) < 1.6 { best } else { c }
        }
        None => if contrast(pal.shape_text, bg) >= 3.0 { pal.shape_text } else { best },
    }
}

pub fn fill_poly(mesh: &mut Mesh, pts: &[Pos2], color: Color32) {
    if pts.len() < 3 || color.a() == 0 { return; }
    let flat: Vec<f64> = pts.iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
    let Ok(idx) = earcutr::earcut(&flat, &[], 2) else { return };
    let base = mesh.vertices.len() as u32;
    for p in pts { mesh.colored_vertex(*p, color); }
    for t in idx.chunks(3) {
        if t.len() == 3 { mesh.add_triangle(base + t[0] as u32, base + t[1] as u32, base + t[2] as u32); }
    }
}

pub fn stroke_path(out: &mut Vec<EShape>, pts: &[Pos2], closed: bool, stroke: Stroke, dash: Dash) {
    if stroke.width <= 0.0 || stroke.color.a() == 0 || pts.len() < 2 { return; }
    match dash {
        Dash::Solid => {
            if closed { out.push(EShape::Path(PathShape::closed_line(pts.to_vec(), PathStroke::new(stroke.width, stroke.color)))); }
            else { out.push(EShape::Path(PathShape::line(pts.to_vec(), PathStroke::new(stroke.width, stroke.color)))); }
        }
        Dash::Dashed | Dash::Dotted => {
            let mut v = pts.to_vec();
            if closed { v.push(pts[0]); }
            let (d, g) = if dash == Dash::Dashed { (stroke.width * 4.0 + 3.0, stroke.width * 2.5 + 2.0) } else { (stroke.width.max(1.0), stroke.width * 2.0 + 1.5) };
            out.extend(EShape::dashed_line(&v, stroke, d, g));
        }
    }
}

pub fn font(size: f32, bold: bool) -> FontId { FontId::new(size.max(1.0), if bold { FontFamily::Name("bold".into()) } else { FontFamily::Proportional }) }

/// Code text: monospace, no wrapping; lines that don't fit are cut with "…" and extra lines dropped.
fn layout_code(p: &Painter, text: &str, size: f32, color: Color32, bx: Rect) -> Arc<Galley> {
    let cw = size * 0.602;
    let max_chars = ((bx.width() / cw).floor() as usize).max(4);
    let max_lines = ((bx.height() / (size * 1.25)).floor() as usize).max(1);
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i == max_lines { break; }
        if i > 0 { out.push('\n'); }
        let last = i + 1 == max_lines && lines.len() > max_lines;
        let l = l.replace('\t', "  ");
        if last { out.push_str("…"); continue; }
        if l.chars().count() > max_chars { out.extend(l.chars().take(max_chars - 1)); out.push('…'); } else { out.push_str(&l); }
    }
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    job.append(&out, 0.0, TextFormat { font_id: FontId::new(size.max(1.0), FontFamily::Monospace), color, line_height: Some(size * 1.25), ..Default::default() });
    p.layout_job(job)
}

pub fn layout(p: &Painter, text: &str, size: f32, bold: bool, italic: bool, color: Color32, wrap: f32, halign: Align) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    // never split a single word: let it overflow the box instead (Visio / draw.io behaviour)
    let longest = text.split_whitespace().max_by_key(|w| w.chars().count()).unwrap_or("");
    let lw = if longest.chars().count() > 3 { p.layout_no_wrap(longest.to_string(), font(size, bold), color).size().x + 1.0 } else { 0.0 };
    job.wrap.max_width = wrap.max(lw).max(4.0);
    job.halign = halign;
    job.append(text, 0.0, TextFormat { font_id: font(size, bold), color, italics: italic, line_height: Some(size * 1.25), ..Default::default() });
    p.layout_job(job)
}

fn halign(a: HAlign) -> Align { match a { HAlign::Left => Align::LEFT, HAlign::Center => Align::Center, HAlign::Right => Align::RIGHT } }

/// Place a galley in an (unrotated) screen box, then rotate around `pivot`.
pub fn place_text(out: &mut Vec<EShape>, g: Arc<Galley>, bx: Rect, ha: HAlign, va: VAlign, pivot: Pos2, angle_deg: f32, color: Color32) {
    let gr = g.rect;
    let x = match ha { HAlign::Left => bx.min.x - gr.min.x, HAlign::Center => bx.center().x - gr.center().x, HAlign::Right => bx.max.x - gr.max.x };
    let y = match va { VAlign::Top => bx.min.y - gr.min.y, VAlign::Middle => bx.center().y - gr.center().y, VAlign::Bottom => bx.max.y - gr.max.y };
    let mut pos = pos2(x, y);
    if angle_deg != 0.0 { pos = pivot + geom::rot(pos - pivot, angle_deg); }
    out.push(EShape::Text(TextShape::new(pos, g, color).with_angle(angle_deg.to_radians())));
}

pub struct Ctx<'a> {
    pub painter: &'a Painter,
    pub view: View,
    pub pal: &'a Pal,
    pub images: &'a HashMap<Id, TextureId>,
    /// The whole document, for drawing linked sub-diagram previews (None: no previews).
    pub doc: Option<&'a Document>,
    /// Preview nesting level (previews inside previews stop at 2).
    pub depth: u8,
}

/// Draw the linked page's content, scaled to fit inside the shape.
fn draw_preview(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape, sub: &Page, fill: Color32, stroke_c: Color32) {
    let v = cx.view;
    let pb = library::preview_box(s).translate(vec2(s.x, s.y));
    let spb = v.rect(pb);
    if spb.width() < 14.0 || spb.height() < 10.0 { return; }
    let Some((content, k, at)) = geom::preview_fit(s, sub) else {
        // empty sub-diagram: a faint dashed frame says "there is a page behind this"
        let r = spb.shrink(2.0);
        let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
        stroke_path(out, &pts, true, Stroke::new(1.0, with_alpha(stroke_c, 0.35)), Dash::Dashed);
        return;
    };
    let zoom = v.zoom * k;
    let sv = View { origin: v.s(at) - content.min.to_vec2() * zoom, pan: Vec2::ZERO, zoom };
    let sub_cx = Ctx { painter: cx.painter, view: sv, pal: cx.pal, images: cx.images, doc: cx.doc, depth: cx.depth + 1 };
    for t in &sub.shapes {
        if !sub.shape_shown(t) { continue; }
        let legible = t.style.font_size.unwrap_or(13.0) * zoom >= 3.5;
        draw_shape(&sub_cx, out, t, legible);
    }
    for (ci, route) in page_routes(sub) {
        let c = &sub.conns[ci];
        draw_conn(&sub_cx, out, c, route, fill, c.style.font_size.unwrap_or(12.0) * zoom >= 3.5);
    }
}

/// Draw one shape. `skip_text` hides text (e.g. while it is being edited).
pub fn draw_shape(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape, with_text: bool) {
    let v = cx.view;
    let z = v.zoom;
    let fill = fill_of(s, cx.pal);
    let stroke_c = stroke_of(&s.style, cx.pal.shape_stroke);
    let sw = s.style.width.unwrap_or(1.5) * z;
    let parts = library::shape_parts(s);
    let screen: Vec<Vec<Pos2>> = parts.iter().map(|p| p.pts.iter().map(|q| v.s(l2w(s, *q))).collect()).collect();

    if s.style.shadow {
        let mut m = Mesh::default();
        let off = vec2(0.0, 4.0 * z);
        for (p, pts) in parts.iter().zip(&screen) {
            if p.closed && p.kind != PK::Hit {
                let shifted: Vec<Pos2> = pts.iter().map(|q| *q + off).collect();
                fill_poly(&mut m, &shifted, Color32::from_black_alpha(if cx.pal.dark { 110 } else { 38 }));
            }
        }
        out.push(EShape::mesh(m));
    }

    if s.kind == "image" {
        if let Some(tex) = cx.images.get(&s.id) {
            let c = [pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)].map(|p| v.s(l2w(s, p)));
            let uv = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)];
            let mut m = Mesh::with_texture(*tex);
            let a = (s.style.opacity.unwrap_or(1.0) * 255.0) as u8;
            for i in 0..4 { m.vertices.push(Vertex { pos: c[i], uv: uv[i], color: Color32::from_white_alpha(a) }); }
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            out.push(EShape::mesh(m));
        }
    }

    for (p, pts) in parts.iter().zip(&screen) {
        match p.kind {
            PK::Main => {
                if p.closed {
                    let mut m = Mesh::default();
                    fill_poly(&mut m, pts, fill);
                    out.push(EShape::mesh(m));
                }
                stroke_path(out, pts, p.closed, Stroke::new(sw, stroke_c), s.style.dash);
            }
            PK::Line => stroke_path(out, pts, p.closed, Stroke::new(sw, stroke_c), if p.dashed { Dash::Dashed } else { s.style.dash }),
            PK::Glyph => { let mut m = Mesh::default(); fill_poly(&mut m, pts, stroke_c); out.push(EShape::mesh(m)); }
            PK::Tint => { let mut m = Mesh::default(); fill_poly(&mut m, pts, with_alpha(stroke_c, 0.16)); out.push(EShape::mesh(m)); }
            PK::Fill => { let mut m = Mesh::default(); fill_poly(&mut m, pts, fill); out.push(EShape::mesh(m)); }
            PK::Hit => {}
        }
    }

    if library::has_compartments(&s.kind) {
        draw_compartments(cx, out, s, stroke_c, sw);
    }
    if cx.depth < 2 && library::shows_preview(s) {
        if let Some(sub) = cx.doc.and_then(|d| d.linked_page(s)) { draw_preview(cx, out, s, sub, fill, stroke_c); }
    }
    if with_text {
        out.extend(text_shapes(cx, s));
    }
}

/// Engineering symbols keep their tag text out of the shape pass: it is drawn after the
/// connectors, on a small backing in the page colour, so pipes never run through a tag.
pub fn label_drawn_last(s: &Shape) -> bool { library::is_symbol(&s.kind) && s.kind != "x_e_panel" }

/// The tag of a symbol with its backing (see `label_drawn_last`).
pub fn masked_label(cx: &Ctx, s: &Shape, bg: Color32) -> Vec<EShape> {
    let text = text_shapes(cx, s);
    let mut out = vec![];
    for t in &text {
        if let EShape::Text(ts) = t {
            let r = Rect::from_min_size(ts.pos, ts.galley.size()).expand(2.0 * cx.view.zoom);
            out.push(EShape::rect_filled(r, 2.0 * cx.view.zoom, bg));
        }
    }
    out.extend(text);
    out
}

/// Text of a shape as egui text shapes (also used by the SVG exporter).
pub fn text_shapes(cx: &Ctx, s: &Shape) -> Vec<EShape> {
    let mut out = vec![];
    if s.kind == "p_script" { script_header(cx, &mut out, s); }
    if s.text.is_empty() { return out; }
    if library::has_compartments(&s.kind) { compartment_text(cx, &mut out, s); } else { draw_shape_text(cx, &mut out, s); }
    out
}

fn sections(text: &str) -> Vec<String> {
    let mut v = vec![String::new()];
    for line in text.lines() {
        if line.trim() == "--" { v.push(String::new()); continue; }
        let cur = v.last_mut().unwrap();
        if !cur.is_empty() { cur.push('\n'); }
        cur.push_str(line);
    }
    v
}

/// Section boundaries (local y) for UML/ER compartments.
pub fn compartment_rows(s: &Shape) -> Vec<(f32, f32)> {
    let secs = sections(&s.text);
    let fs = s.style.font_size.unwrap_or(13.0);
    let lh = fs * 1.25;
    let n0 = secs[0].lines().count().max(1) as f32;
    let h0 = (n0 * lh + 12.0).min(s.h);
    let mut rows = vec![(0.0, h0)];
    let rest: Vec<f32> = secs[1..].iter().map(|t| t.lines().count().max(1) as f32).collect();
    let total: f32 = rest.iter().sum();
    let mut y = h0;
    for n in rest {
        let hh = if total > 0.0 { (s.h - h0) * n / total } else { 0.0 };
        rows.push((y, y + hh));
        y += hh;
    }
    rows
}

fn draw_compartments(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape, stroke_c: Color32, sw: f32) {
    let v = cx.view;
    let rows = compartment_rows(s);
    // header tint
    let (h0a, h0b) = rows[0];
    let hp: Vec<Pos2> = [pos2(0.0, h0a), pos2(s.w, h0a), pos2(s.w, h0b), pos2(0.0, h0b)].iter().map(|p| v.s(l2w(s, *p))).collect();
    let mut m = Mesh::default();
    fill_poly(&mut m, &hp, with_alpha(stroke_c, 0.14));
    out.push(EShape::mesh(m));
    for (i, (a, _)) in rows.iter().enumerate().skip(1) {
        let _ = i;
        out.push(EShape::line_segment([v.s(l2w(s, pos2(0.0, *a))), v.s(l2w(s, pos2(s.w, *a)))], Stroke::new(sw, stroke_c)));
    }
}

fn compartment_text(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape) {
    let v = cx.view;
    let z = v.zoom;
    let secs = sections(&s.text);
    let rows = compartment_rows(s);
    let fs = s.style.font_size.unwrap_or(13.0) * z;
    let col = shape_text_color(s, cx.pal);
    let (cxw, cyw) = s.center();
    let pivot = v.s(pos2(cxw, cyw));
    for (i, ((a, b), t)) in rows.iter().zip(secs.iter()).enumerate() {
        if t.is_empty() { continue; }
        let bx_local = Rect::from_min_max(pos2(8.0, *a + 4.0), pos2(s.w - 8.0, *b - 4.0));
        let bx = Rect::from_min_size(pivot + (bx_local.min.to_vec2() - vec2(s.w / 2.0, s.h / 2.0)) * z, bx_local.size() * z);
        let header = i == 0;
        let ha = if header { HAlign::Center } else { s.style.align.unwrap_or(HAlign::Left) };
        let g = layout(cx.painter, t, fs, header || s.style.bold, s.style.italic, col, bx.width(), halign(ha));
        place_text(out, g, bx, ha, if header { VAlign::Middle } else { VAlign::Top }, pivot, s.rot, col);
    }
}

pub fn text_screen_box(v: &View, s: &Shape) -> (Rect, Pos2, bool) {
    let tb = library::shape_text_box(s);
    let (cxw, cyw) = s.center();
    let pivot = v.s(pos2(cxw, cyw));
    let r = Rect::from_min_size(pivot + (tb.rect.min.to_vec2() - vec2(s.w / 2.0, s.h / 2.0)) * v.zoom, tb.rect.size() * v.zoom);
    (r, pivot, tb.rotated)
}

/// File name in the title bar of a script block.
fn script_header(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape) {
    let v = cx.view;
    let name = s.data.iter().find(|(k, _)| k == "file").map(|(_, v)| v.clone()).unwrap_or_else(|| "script".into());
    let (cxw, cyw) = s.center();
    let pivot = v.s(pos2(cxw, cyw));
    let bx = Rect::from_min_size(pivot + (vec2(46.0, 2.0) - vec2(s.w / 2.0, s.h / 2.0)) * v.zoom, vec2((s.w - 54.0).max(10.0), 20.0) * v.zoom);
    let col = with_alpha(shape_text_color(s, cx.pal), 0.8);
    let g = layout_code(cx.painter, &name, 11.0 * v.zoom, col, bx);
    place_text(out, g, bx, HAlign::Left, VAlign::Middle, pivot, s.rot, col);
}

fn draw_shape_text(cx: &Ctx, out: &mut Vec<EShape>, s: &Shape) {
    let v = cx.view;
    let (bx, pivot, rotated) = text_screen_box(&v, s);
    let fs = s.style.font_size.unwrap_or(if library::is_icon(&s.kind) { 12.0 } else { 13.0 }) * v.zoom;
    let col = shape_text_color(s, cx.pal);
    let ha = s.style.align.unwrap_or(HAlign::Center);
    let va = s.style.valign.unwrap_or(if library::is_icon(&s.kind) || s.kind == "image" { VAlign::Top } else { VAlign::Middle });
    if rotated {
        // vertical header text (swimlane): wrap along the box height, rotate -90 about box centre
        let g = layout(cx.painter, &s.text, fs, s.style.bold, s.style.italic, col, bx.height(), Align::Center);
        let c = pivot + geom::rot(bx.center() - pivot, s.rot);
        let fake = Rect::from_center_size(c, vec2(bx.height(), bx.width()));
        place_text(out, g, fake, HAlign::Center, VAlign::Middle, c, s.rot - 90.0, col);
        return;
    }
    if s.style.mono {
        let g = layout_code(cx.painter, &s.text, fs, col, bx);
        place_text(out, g, bx, ha, va, pivot, s.rot, col);
        return;
    }
    let g = layout(cx.painter, &s.text, fs, s.style.bold, s.style.italic, col, bx.width(), halign(ha));
    place_text(out, g, bx, ha, va, pivot, s.rot, col);
}

/// Draw an arrow head at `tip` pointing along `dir` (unit). Returns how far to shorten the line.
fn head(out: &mut Vec<EShape>, kind: Arrow, tip: Pos2, dir: Vec2, size: f32, color: Color32, bg: Color32, sw: f32) {
    let n = vec2(-dir.y, dir.x);
    match kind {
        Arrow::None => {}
        Arrow::Filled => out.push(EShape::convex_polygon(vec![tip, tip - dir * size + n * size * 0.42, tip - dir * size - n * size * 0.42], color, Stroke::NONE)),
        Arrow::Open => out.push(EShape::line(vec![tip - dir * size + n * size * 0.45, tip, tip - dir * size - n * size * 0.45], PathStroke::new(sw, color))),
        Arrow::Diamond => out.push(EShape::convex_polygon(vec![tip, tip - dir * size * 0.6 + n * size * 0.38, tip - dir * size * 1.2, tip - dir * size * 0.6 - n * size * 0.38], bg, Stroke::new(sw, color))),
        Arrow::Circle => out.push(EShape::circle_filled(tip - dir * size * 0.42, size * 0.42, bg)),
    }
    if kind == Arrow::Circle { out.push(EShape::circle_stroke(tip - dir * size * 0.42, size * 0.42, Stroke::new(sw, color))); }
}

fn shorten(kind: Arrow, size: f32) -> f32 { match kind { Arrow::Filled => size * 0.85, Arrow::Diamond => size * 1.2, Arrow::Circle => size * 0.84, _ => 0.0 } }

/// Draw a connector along `pts_w` (its route in world coords, possibly with line jumps).
pub fn draw_conn(cx: &Ctx, out: &mut Vec<EShape>, c: &Conn, pts_w: Vec<Pos2>, page_bg: Color32, with_text: bool) -> Vec<Pos2> {
    let v = cx.view;
    let z = v.zoom;
    let mut pts: Vec<Pos2> = pts_w.iter().map(|p| v.s(*p)).collect();
    if pts.len() < 2 { return pts_w; }
    let width = c.style.width.unwrap_or(1.5);
    let sw = width * z;
    let col = stroke_of(&c.style, cx.pal.conn);
    let size = (8.0 + width * 2.2) * z;
    let n = pts.len();
    let dir_end = (pts[n - 1] - pts[n - 2]).normalized();
    let dir_start = (pts[0] - pts[1]).normalized();
    let tip_end = pts[n - 1];
    let tip_start = pts[0];
    let se = shorten(c.arrow_end, size).min((pts[n - 1] - pts[n - 2]).length() * 0.9);
    let ss = shorten(c.arrow_start, size).min((pts[0] - pts[1]).length() * 0.9);
    pts[n - 1] = pts[n - 1] - dir_end * se;
    pts[0] = pts[0] - dir_start * ss;
    stroke_path(out, &pts, false, Stroke::new(sw, col), c.style.dash);
    head(out, c.arrow_end, tip_end, dir_end, size, col, page_bg, sw);
    head(out, c.arrow_start, tip_start, dir_start, size, col, page_bg, sw);
    if with_text { out.extend(conn_label(cx, c, &pts_w, page_bg)); }
    pts_w
}

pub fn conn_label(cx: &Ctx, c: &Conn, pts_w: &[Pos2], page_bg: Color32) -> Vec<EShape> {
    let mut out = vec![];
    if c.text.is_empty() || pts_w.len() < 2 { return out; }
    let v = cx.view;
    let z = v.zoom;
    let fs = c.style.font_size.unwrap_or(12.0) * z;
    let tc = c.style.text.map(to_c32).unwrap_or(cx.pal.shape_text);
    let g = layout(cx.painter, &c.text, fs, c.style.bold, c.style.italic, tc, 220.0 * z, Align::Center);
    let mid = v.s(geom::poly_mid(pts_w));
    let r = Rect::from_center_size(mid, g.rect.size() + vec2(8.0, 4.0) * z);
    out.push(EShape::rect_filled(r, 3.0 * z, page_bg));
    place_text(&mut out, g, r, HAlign::Center, VAlign::Middle, mid, 0.0, tc);
    out
}

/// Routes for every visible connector on the page, with line jumps applied when enabled.
pub fn page_routes(page: &Page) -> Vec<(usize, Vec<Pos2>)> {
    let idx: Vec<usize> = (0..page.conns.len()).filter(|i| page.conn_shown(&page.conns[*i])).collect();
    let routes: Vec<Vec<Pos2>> = idx.iter().map(|i| geom::route(page, &page.conns[*i])).collect();
    if !page.line_jumps || routes.len() < 2 { return idx.into_iter().zip(routes).collect(); }
    idx.iter().enumerate().map(|(k, i)| (*i, crate::route::with_jumps(&routes, k, 5.0))).collect()
}
