//! SVG / PNG / PDF export.
use crate::geom::{self, l2w};
use crate::anim;
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

/// CSS for one animated shape's group: (class, inline style, keyframes rule).
fn shape_css(s: &Shape, i: usize, pal: &crate::theme::Pal) -> (String, String) {
    let dur = anim::period(&s.style);
    let name = format!("nq{i}");
    let base = format!("animation:{name} {dur:.2}s {} infinite;transform-box:fill-box;transform-origin:center", if s.style.anim == Anim::Spin { "linear" } else { "ease-in-out" });
    let kf = match s.style.anim {
        Anim::Pulse => "0%,100%{transform:scale(1)}50%{transform:scale(1.08)}".to_string(),
        Anim::Spin => "from{transform:rotate(0deg)}to{transform:rotate(360deg)}".to_string(),
        Anim::Shake => "0%,100%{transform:translateX(0)}".to_string() + &(1..12).map(|k| format!("{:.2}%{{transform:translateX({}px)}}", k as f32 * 100.0 / 12.0, if k % 2 == 1 { 3 } else { -3 })).collect::<String>(),
        Anim::Blink => "0%,49.9%{opacity:1}50%,100%{opacity:.18}".to_string(),
        Anim::Glow => {
            let c = col(render::glow_color(&s.style, render::stroke_of(&s.style, pal.shape_stroke)));
            format!("0%,100%{{filter:drop-shadow(0 0 2px {c})}}50%{{filter:drop-shadow(0 0 9px {c}) drop-shadow(0 0 3px {c})}}")
        }
        _ => return (String::new(), String::new()),
    };
    (base, format!("@keyframes {name}{{{kf}}}"))
}

/// Shapes and connectors of a page as SVG elements (world coordinates). With `css`, animated
/// items get CSS animations (their keyframes are added to `css`); otherwise each is drawn at
/// `cx.time`.
fn write_items(o: &mut String, cx: &Ctx, page: &Page, page_bg: Color32, mut css: Option<&mut String>) {
    let pal = cx.pal;
    for (si, s0) in page.shapes.iter().enumerate().filter(|(_, s)| page.shape_shown(s)) {
        // the shape as it stands at this moment (text is drawn from the unmoved shape)
        let moved = cx.time.and_then(|t| anim::shape_at(s0, t));
        let s = moved.as_ref().unwrap_or(s0);
        let fill = render::fill_of(s, pal);
        let stroke = render::stroke_of(&s.style, pal.shape_stroke);
        let sw = s.style.width.unwrap_or(1.5);
        let filt = if s.style.shadow { " filter=\"url(#sh)\"" } else { "" };
        let _ = write!(o, "<g{filt}>");
        if let (Some(t), Anim::Glow) = (cx.time, s.style.anim) {
            let c = render::glow_color(&s.style, stroke);
            for part in library::shape_parts(s).iter().filter(|p| p.kind != PK::Hit) {
                let pts: Vec<Pos2> = part.pts.iter().map(|p| l2w(s, *p)).collect();
                for (w, a) in render::glow_layers(sw, anim::glow_level(&s.style, t)) {
                    let _ = write!(o, "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{w:.2}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>", path_d(&pts, part.closed), col(render::with_alpha(c, a)));
                }
            }
        }
        let animated = match css.as_deref_mut() {
            Some(rules) if s.style.anim != Anim::None => {
                let (style, kf) = shape_css(s, si, pal);
                rules.push_str(&kf);
                let _ = write!(o, "<g style=\"{style}\">");
                true
            }
            _ => false,
        };
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
                    let sub_cx = Ctx { depth: cx.depth + 1, time: cx.time.or(Some(0.0)), ..*cx };
                    write_items(o, &sub_cx, sub, fill, None);
                    o.push_str("</g>");
                }
            }
        }
        if animated { o.push_str("</g>"); }
        if !render::label_drawn_last(s0) { for t in render::text_shapes(cx, s0) { eshape_svg(o, &t); } }
        o.push_str("</g>");
    }
    for (ci, route) in render::page_routes(page) {
        let c = &page.conns[ci];
        let mut out = vec![];
        let a = c.style.anim;
        let blink = css.is_some() && a == Anim::Blink;
        if blink {
            let name = format!("nqc{ci}");
            if let Some(r) = css.as_deref_mut() { let _ = write!(r, "@keyframes {name}{{0%,49.9%{{opacity:1}}50%,100%{{opacity:.18}}}}"); }
            let _ = write!(o, "<g style=\"animation:{name} {:.2}s infinite\">", anim::period(&c.style));
        }
        let sw = c.style.width.unwrap_or(1.5);
        let line = render::stroke_of(&c.style, pal.conn);
        let moving = render::glow_color(&c.style, line);
        if let (Some(rules), Anim::Glow) = (css.as_deref_mut(), a) {
            // a wide soft line under the connector that brightens and fades
            let name = format!("nqc{ci}");
            let _ = write!(rules, "@keyframes {name}{{0%,100%{{opacity:.2}}50%{{opacity:.9}}}}");
            for (w, al) in render::glow_layers(sw, 1.0) {
                let _ = write!(o, "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{w:.2}\" stroke-linejoin=\"round\" stroke-linecap=\"round\" style=\"animation:{name} {:.2}s ease-in-out infinite\"/>", path_d(&route, false), col(render::with_alpha(moving, al)), anim::period(&c.style));
            }
        }
        // the label goes on last, over any moving dashes
        render::draw_conn(cx, &mut out, c, route.clone(), page_bg, false);
        let da = dash_attr(c.style.dash, sw);
        if c.style.dash != Dash::Solid && a != Anim::Flow {
            // dashed lines come back as many segments; emit the main path once (under the label) with dash attributes instead
            let pts = &route;
            let _ = write!(o, "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{sw}\"{da}/>", path_d(pts, false), col(render::stroke_of(&c.style, pal.conn)));
        }
        for sh in &out {
            if c.style.dash != Dash::Solid && a != Anim::Flow { if let EShape::LineSegment { .. } = sh { continue; } }
            eshape_svg(o, sh);
        }
        if let (Some(rules), Anim::Flow | Anim::Dots) = (css.as_deref_mut(), a) {
            // dashes (or round dots: zero-length dashes with round ends) whose offset runs down
            let name = format!("nqc{ci}");
            let shift = anim::travel(a, sw, 1.0);
            let _ = write!(rules, "@keyframes {name}{{from{{stroke-dashoffset:{shift:.2}}}to{{stroke-dashoffset:0}}}}");
            let (pattern, width, cap) = if a == Anim::Flow { let (d, g) = anim::flow_dash(sw); (format!("{d:.2} {g:.2}"), sw, "butt") } else { let (gap, r) = anim::dot_spacing(sw); (format!("0 {gap:.2}"), r * 2.0, "round") };
            let _ = write!(o, "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width:.2}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"round\" stroke-dasharray=\"{pattern}\" style=\"animation:{name} {:.2}s linear infinite\"/>", path_d(&route, false), col(moving), anim::period(&c.style));
        }
        for sh in render::conn_label(cx, c, &route, page_bg) { eshape_svg(o, &sh); }
        if blink { o.push_str("</g>"); }
    }
    for s in page.shapes.iter().filter(|s| page.shape_shown(s) && render::label_drawn_last(s)) {
        for sh in render::masked_label(cx, s, page_bg) { eshape_svg(o, &sh); }
    }
}

/// How animations appear in an export.
#[derive(Clone, Copy, PartialEq)]
pub enum Motion {
    /// A still picture (animations at their first frame).
    Still,
    /// The picture at this many seconds.
    At(f64),
    /// Moving, as CSS animations (for SVG files).
    Css,
}

/// The page's content box, with padding.
pub fn content_bounds(page: &Page, padding: f32) -> Rect {
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
    bounds.unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(page.width, page.height))).expand(padding)
}

/// Build a standalone SVG of the page's content.
pub fn page_svg(doc: Option<&Document>, page: &Page, pal: &Pal, painter: &Painter, padding: f32) -> (String, f32, f32) {
    page_svg_with(doc, page, pal, painter, content_bounds(page, padding), Motion::Still)
}

/// SVG of a fixed area of the page, e.g. the whole sheet for printing.
#[cfg_attr(not(feature = "pro"), allow(dead_code))]
pub fn page_svg_area(doc: Option<&Document>, page: &Page, pal: &Pal, painter: &Painter, b: Rect) -> (String, f32, f32) {
    page_svg_with(doc, page, pal, painter, b, Motion::Still)
}

pub fn page_svg_with(doc: Option<&Document>, page: &Page, pal: &Pal, painter: &Painter, b: Rect, motion: Motion) -> (String, f32, f32) {
    let (w, h) = (b.width().ceil(), b.height().ceil());
    let page_bg = page.bg.map(crate::theme::to_c32).unwrap_or(pal.page);
    let mut o = String::new();
    let _ = write!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"{:.2} {:.2} {w} {h}\">", b.min.x, b.min.y);
    let _ = write!(o, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", b.min.x, b.min.y, col(page_bg));
    o.push_str("<defs><filter id=\"sh\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"150%\"><feDropShadow dx=\"0\" dy=\"4\" stdDeviation=\"3.5\" flood-color=\"#000\" flood-opacity=\"0.22\"/></filter></defs>");

    let empty = HashMap::new();
    let time = match motion { Motion::Still => Some(0.0), Motion::At(t) => Some(t), Motion::Css => None };
    let cx = Ctx { painter, view: View { origin: Pos2::ZERO, pan: egui::Vec2::ZERO, zoom: 1.0 }, pal, images: &empty, doc, depth: 0, time };
    let mut rules = String::new();
    let mut items = String::new();
    write_items(&mut items, &cx, page, page_bg, if motion == Motion::Css { Some(&mut rules) } else { None });
    if !rules.is_empty() {
        // viewers that don't animate (or people who asked for less motion) see the still picture
        let _ = write!(o, "<style>@media (prefers-reduced-motion: reduce){{*{{animation:none!important}}}}{rules}</style>");
    }
    o.push_str(&items);
    o.push_str("</svg>");
    (o, w, h)
}

/// An animated GIF of the page's content: one loop of its animations at 12.5 frames a second.
/// Each frame after the first only carries the part of the picture that changed.
pub fn page_gif(doc: Option<&Document>, page: &Page, pal: &Pal, painter: &Painter, padding: f32, scale: f32) -> Result<Vec<u8>, String> {
    const DELAY_CS: u16 = 8;
    let b = content_bounds(page, padding);
    let secs = anim::loop_secs(page);
    let n = ((secs * 100.0 / DELAY_CS as f32).round() as usize).max(1);
    let mut frames: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(n);
    for i in 0..n {
        let (svg, _, _) = page_svg_with(doc, page, pal, painter, b, Motion::At(i as f64 * secs as f64 / n as f64));
        let t = tree(&svg)?;
        let size = t.size().to_int_size().scale_by(scale).ok_or("bad size")?;
        let mut pm = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("image too large")?;
        resvg::render(&t, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
        frames.push((size.width(), size.height(), pm.take()));
    }
    let (w, h) = (frames[0].0, frames[0].1);
    if w > u16::MAX as u32 || h > u16::MAX as u32 { return Err("image too large".into()); }
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, w as u16, h as u16, &[]).map_err(|e| e.to_string())?;
        enc.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;
        let mut prev: Option<&Vec<u8>> = None;
        for (_, _, px) in &frames {
            // the box of pixels that differ from the previous frame
            let (mut x0, mut y0, mut x1, mut y1) = (0, 0, w, h);
            if let Some(p) = prev {
                let (mut a, mut bb, mut c, mut d) = (w, h, 0, 0);
                for y in 0..h { for x in 0..w {
                    let k = ((y * w + x) * 4) as usize;
                    if px[k..k + 4] != p[k..k + 4] { a = a.min(x); bb = bb.min(y); c = c.max(x + 1); d = d.max(y + 1); }
                } }
                if c == 0 { (x0, y0, x1, y1) = (0, 0, 1, 1); } else { (x0, y0, x1, y1) = (a, bb, c, d); }
            }
            let (fw, fh) = (x1 - x0, y1 - y0);
            let mut rgba = Vec::with_capacity((fw * fh * 4) as usize);
            for y in y0..y1 { let k = ((y * w + x0) * 4) as usize; rgba.extend_from_slice(&px[k..k + (fw * 4) as usize]); }
            // the page background is opaque, so premultiplied pixels are already plain colours
            let mut f = gif::Frame::from_rgba_speed(fw as u16, fh as u16, &mut rgba, 20);
            f.left = x0 as u16; f.top = y0 as u16; f.delay = DELAY_CS;
            f.dispose = gif::DisposalMethod::Keep;
            enc.write_frame(&f).map_err(|e| e.to_string())?;
            prev = Some(px);
        }
    }
    Ok(out)
}

/// Parse an SVG made by this module (with the fonts used for export).
#[cfg_attr(not(feature = "pro"), allow(dead_code))]
pub fn svg_tree(svg: &str) -> Result<resvg::usvg::Tree, String> { tree(svg) }

fn tree(svg: &str) -> Result<resvg::usvg::Tree, String> {
    let mut opt = resvg::usvg::Options::default();
    load_fonts(opt.fontdb_mut());
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
    let t = svg2pdf::usvg::Tree::from_str(svg, &{ let mut o = svg2pdf::usvg::Options::default(); load_fonts(o.fontdb_mut()); o }).map_err(|e| e.to_string())?;
    svg2pdf::to_pdf(&t, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default()).map_err(|e| format!("{e:?}"))
}

/// Fonts for PNG/PDF export. The desktop uses the system's; the browser has none, so it uses the
/// fonts built into the app (DejaVu Sans, as on screen, and egui's monospace font).
fn load_fonts(db: &mut svg2pdf::usvg::fontdb::Database) {
    #[cfg(not(target_arch = "wasm32"))]
    db.load_system_fonts();
    #[cfg(target_arch = "wasm32")]
    {
        db.load_font_data(include_bytes!("../web/fonts/DejaVuSans.ttf").to_vec());
        db.load_font_data(include_bytes!("../web/fonts/DejaVuSans-Bold.ttf").to_vec());
        let defs = egui::FontDefinitions::default();
        let mut mono = None;
        for (name, data) in &defs.font_data {
            if !name.contains("Hack") { continue; }
            let before = db.len();
            db.load_font_data(data.font.to_vec());
            mono = db.faces().nth(before).and_then(|f| f.families.first().map(|(n, _)| n.clone()));
        }
        db.set_sans_serif_family("DejaVu Sans");
        db.set_serif_family("DejaVu Sans");
        if let Some(f) = mono { db.set_monospace_family(f); }
    }
}
