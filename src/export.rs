//! SVG / PNG / PDF export.
use crate::geom::{self, l2w};
use crate::library::{self, PK};
use crate::model::*;
use crate::render::{self, Ctx, View};
use crate::theme::Pal;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape as EShape};
use std::collections::HashMap;
use std::fmt::Write;

fn col(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0) }
}
fn path_d(pts: &[Pos2], closed: bool) -> String {
    let mut s = String::new();
    for (i, p) in pts.iter().enumerate() { let _ = write!(s, "{}{:.2},{:.2} ", if i == 0 { "M" } else { "L" }, p.x, p.y); }
    if closed { s.push('Z'); }
    s
}
fn esc(t: &str) -> String { t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn dash_attr(d: Dash, w: f32) -> String {
    match d { Dash::Solid => String::new(), Dash::Dashed => format!(" stroke-dasharray=\"{:.1} {:.1}\"", w * 4.0 + 3.0, w * 2.5 + 2.0), Dash::Dotted => format!(" stroke-dasharray=\"{:.1} {:.1}\" stroke-linecap=\"round\"", w.max(1.0), w * 2.0 + 1.5) }
}

/// Convert egui shapes we emit for text / labels / heads into SVG.
fn eshape_svg(out: &mut String, sh: &EShape) {
    match sh {
        EShape::Text(t) => {
            let ang = t.angle.to_degrees();
            let _ = write!(out, "<g transform=\"translate({:.2} {:.2}) rotate({:.2})\">", t.pos.x, t.pos.y, ang);
            for row in &t.galley.rows {
                let Some(g0) = row.glyphs.first() else { continue };
                let text: String = row.glyphs.iter().map(|g| g.chr).collect();
                let sec = &t.galley.job.sections[0];
                let f = &sec.format;
                let c = t.override_text_color.unwrap_or(f.color);
                let bold = matches!(&f.font_id.family, egui::FontFamily::Name(n) if n.as_ref() == "bold");
                let family = if f.font_id.family == egui::FontFamily::Monospace { "Menlo, Consolas, 'DejaVu Sans Mono', monospace" } else { "-apple-system, Helvetica, Arial, sans-serif" };
                let _ = write!(out, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"{family}\" font-size=\"{:.2}\" fill=\"{}\"{}{} xml:space=\"preserve\">{}</text>",
                    row.pos.x + g0.pos.x, row.pos.y + g0.pos.y, f.font_id.size, col(c),
                    if bold { " font-weight=\"bold\"" } else { "" }, if f.italics { " font-style=\"italic\"" } else { "" }, esc(&text));
            }
            out.push_str("</g>");
        }
        EShape::Rect(r) => {
            let rr = r.rect;
            let _ = write!(out, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.1}\" fill=\"{}\"/>", rr.min.x, rr.min.y, rr.width(), rr.height(), r.corner_radius.nw as f32, col(r.fill));
        }
        EShape::Path(p) => {
            let fill = if p.fill.a() > 0 { col(p.fill) } else { "none".into() };
            let (w, c) = (p.stroke.width, match &p.stroke.color { egui::epaint::ColorMode::Solid(c) => *c, _ => Color32::BLACK });
            let _ = write!(out, "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>", path_d(&p.points, p.closed), fill, if w > 0.0 { col(c) } else { "none".into() }, w);
        }
        EShape::LineSegment { points, stroke } => {
            let _ = write!(out, "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>", points[0].x, points[0].y, points[1].x, points[1].y, col(stroke.color), stroke.width);
        }
        EShape::Circle(c) => {
            let _ = write!(out, "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>", c.center.x, c.center.y, c.radius, if c.fill.a() > 0 { col(c.fill) } else { "none".into() }, col(c.stroke.color), c.stroke.width);
        }
        EShape::Vec(v) => for s in v { eshape_svg(out, s); },
        _ => {}
    }
}

/// Shapes and connectors of a page as SVG elements (world coordinates).
fn write_items(o: &mut String, cx: &Ctx, page: &Page, page_bg: Color32) {
    let pal = cx.pal;
    for s in page.shapes.iter().filter(|s| page.shape_shown(s)) {
        let fill = render::fill_of(s, pal);
        let stroke = render::stroke_of(&s.style, pal.shape_stroke);
        let sw = s.style.width.unwrap_or(1.5);
        let filt = if s.style.shadow { " filter=\"url(#sh)\"" } else { "" };
        let _ = write!(o, "<g{filt}>");
        if let (Some(data), "image") = (&s.image, s.kind.as_str()) {
            let (cx0, cy0) = s.center();
            let _ = write!(o, "<image href=\"data:image/png;base64,{data}\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" preserveAspectRatio=\"none\" transform=\"rotate({:.2} {:.2} {:.2})\"/>", s.x, s.y, s.w, s.h, s.rot, cx0, cy0);
        }
        for part in library::shape_parts(s) {
            let pts: Vec<Pos2> = part.pts.iter().map(|p| l2w(s, *p)).collect();
            let d = path_d(&pts, part.closed);
            let da = dash_attr(if part.dashed { Dash::Dashed } else { s.style.dash }, sw);
            match part.kind {
                PK::Main => { let _ = write!(o, "<path d=\"{d}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{sw}\" stroke-linejoin=\"round\"{da}/>", if part.closed { col(fill) } else { "none".into() }, col(stroke)); }
                PK::Line => { let _ = write!(o, "<path d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{sw}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"{da}/>", col(stroke)); }
                PK::Glyph => { let _ = write!(o, "<path d=\"{d}\" fill=\"{}\"/>", col(stroke)); }
                PK::Tint => { let _ = write!(o, "<path d=\"{d}\" fill=\"{}\"/>", col(render::with_alpha(stroke, 0.16))); }
                PK::Fill => { let _ = write!(o, "<path d=\"{d}\" fill=\"{}\"/>", col(fill)); }
                PK::Hit => {}
            }
        }
        if library::has_compartments(&s.kind) {
            let rows = render::compartment_rows(s);
            let (a, bb) = rows[0];
            let hp: Vec<Pos2> = [pos2(0.0, a), pos2(s.w, a), pos2(s.w, bb), pos2(0.0, bb)].iter().map(|p| l2w(s, *p)).collect();
            let _ = write!(o, "<path d=\"{}\" fill=\"{}\"/>", path_d(&hp, true), col(render::with_alpha(stroke, 0.14)));
            for (a, _) in rows.iter().skip(1) {
                let (p1, p2) = (l2w(s, pos2(0.0, *a)), l2w(s, pos2(s.w, *a)));
                let _ = write!(o, "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{sw}\"/>", p1.x, p1.y, p2.x, p2.y, col(stroke));
            }
        }
        if cx.depth < 2 && library::shows_preview(s) {
            if let Some(sub) = cx.doc.and_then(|d| d.linked_page(s)) {
                if let Some((content, k, at)) = geom::preview_fit(s, sub) {
                    let _ = write!(o, "<g transform=\"translate({:.2} {:.2}) scale({:.5}) translate({:.2} {:.2})\">", at.x, at.y, k, -content.min.x, -content.min.y);
                    let sub_cx = Ctx { painter: cx.painter, view: cx.view, pal: cx.pal, images: cx.images, doc: cx.doc, depth: cx.depth + 1 };
                    write_items(o, &sub_cx, sub, fill);
                    o.push_str("</g>");
                }
            }
        }
        for t in render::text_shapes(cx, s) { eshape_svg(o, &t); }
        o.push_str("</g>");
    }
    for (ci, route) in render::page_routes(page) {
        let c = &page.conns[ci];
        let mut out = vec![];
        render::draw_conn(cx, &mut out, c, route.clone(), page_bg, true);
        let sw = c.style.width.unwrap_or(1.5);
        let da = dash_attr(c.style.dash, sw);
        if c.style.dash != Dash::Solid {
            // dashed lines come back as many segments; emit the main path once (under the label) with dash attributes instead
            let pts = route;
            let _ = write!(o, "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{sw}\"{da}/>", path_d(&pts, false), col(render::stroke_of(&c.style, pal.conn)));
        }
        for sh in &out {
            if c.style.dash != Dash::Solid { if let EShape::LineSegment { .. } = sh { continue; } }
            eshape_svg(o, sh);
        }
    }
}

/// Build a standalone SVG of the page's content.
pub fn page_svg(doc: Option<&Document>, page: &Page, pal: &Pal, painter: &Painter, padding: f32) -> (String, f32, f32) {
    let mut bounds: Option<Rect> = None;
    for s in &page.shapes {
        let mut b = geom::bbox(s);
        if !s.text.is_empty() {
            let tb = library::shape_text_box(s).rect;
            b = b.union(Rect::from_min_size(pos2(s.x + tb.min.x, s.y + tb.min.y), tb.size()));
        }
        bounds = geom::rect_union(bounds, b);
    }
    for c in &page.conns { for p in geom::route(page, c) { bounds = geom::rect_union(bounds, Rect::from_center_size(p, vec2(16.0, 16.0))); } }
    let b = bounds.unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(page.width, page.height))).expand(padding);
    let (w, h) = (b.width().ceil(), b.height().ceil());
    let page_bg = page.bg.map(crate::theme::to_c32).unwrap_or(pal.page);
    let mut o = String::new();
    let _ = write!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"{:.2} {:.2} {w} {h}\">", b.min.x, b.min.y);
    let _ = write!(o, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", b.min.x, b.min.y, col(page_bg));
    o.push_str("<defs><filter id=\"sh\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"150%\"><feDropShadow dx=\"0\" dy=\"4\" stdDeviation=\"3.5\" flood-color=\"#000\" flood-opacity=\"0.22\"/></filter></defs>");

    let empty = HashMap::new();
    let cx = Ctx { painter, view: View { origin: Pos2::ZERO, pan: egui::Vec2::ZERO, zoom: 1.0 }, pal, images: &empty, doc, depth: 0 };
    write_items(&mut o, &cx, page, page_bg);
    o.push_str("</svg>");
    (o, w, h)
}

fn tree(svg: &str) -> Result<resvg::usvg::Tree, String> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())
}

pub fn svg_to_png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    let t = tree(svg)?;
    let size = t.size().to_int_size().scale_by(scale).ok_or("bad size")?;
    let mut pm = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("image too large")?;
    resvg::render(&t, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    pm.encode_png().map_err(|e| e.to_string())
}

pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let t = svg2pdf::usvg::Tree::from_str(svg, &{ let mut o = svg2pdf::usvg::Options::default(); o.fontdb_mut().load_system_fonts(); o }).map_err(|e| e.to_string())?;
    svg2pdf::to_pdf(&t, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default()).map_err(|e| format!("{e:?}"))
}
