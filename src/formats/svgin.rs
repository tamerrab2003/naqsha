//! SVG import: every path becomes an editable custom shape, text becomes text shapes,
//! embedded bitmaps become image shapes. Groups are kept as Naqsha groups.
use super::Imported;
use crate::model::*;
use resvg::tiny_skia::{self, PathSegment};
use resvg::usvg;

fn paint_col(p: &usvg::Paint, opacity: f32) -> Option<Col> {
    let c = match p {
        usvg::Paint::Color(c) => *c,
        usvg::Paint::LinearGradient(g) => g.stops().first()?.color(),
        usvg::Paint::RadialGradient(g) => g.stops().first()?.color(),
        usvg::Paint::Pattern(_) => return Some(Col([200, 200, 200, (opacity * 255.0) as u8])),
    };
    Some(Col([c.red, c.green, c.blue, (opacity * 255.0).round() as u8]))
}

struct Ctx {
    doc: Document,
    shapes: Vec<Shape>,
    page_w: f32,
    page_h: f32,
    bg: Option<Col>,
    first_path: bool,
    skipped_gradients: usize,
}

fn flatten(path: &tiny_skia::Path) -> Vec<(Vec<(f32, f32)>, bool)> {
    let mut out = vec![];
    let mut cur: Vec<(f32, f32)> = vec![];
    let mut last = (0.0, 0.0);
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => { if cur.len() > 1 { out.push((std::mem::take(&mut cur), false)); } cur.clear(); last = (p.x, p.y); cur.push(last); }
            PathSegment::LineTo(p) => { last = (p.x, p.y); cur.push(last); }
            PathSegment::QuadTo(c, p) => {
                for i in 1..=10 { let t = i as f32 / 10.0; let u = 1.0 - t; cur.push((u * u * last.0 + 2.0 * u * t * c.x + t * t * p.x, u * u * last.1 + 2.0 * u * t * c.y + t * t * p.y)); }
                last = (p.x, p.y);
            }
            PathSegment::CubicTo(c1, c2, p) => {
                for i in 1..=14 {
                    let t = i as f32 / 14.0;
                    let u = 1.0 - t;
                    cur.push((u * u * u * last.0 + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * p.x, u * u * u * last.1 + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * p.y));
                }
                last = (p.x, p.y);
            }
            PathSegment::Close => { if cur.len() > 1 { if let Some(f) = cur.first().copied() { last = f; } out.push((std::mem::take(&mut cur), true)); } }
        }
    }
    if cur.len() > 1 { out.push((cur, false)); }
    out
}

fn walk(ctx: &mut Ctx, g: &usvg::Group, group: Option<Id>, top: bool) {
    for node in g.children() {
        match node {
            usvg::Node::Group(sub) => {
                let gid = if top && sub.children().len() > 1 { Some(ctx.doc.new_id()) } else { group };
                walk(ctx, sub, gid, false);
            }
            usvg::Node::Path(p) => {
                if !p.is_visible() { continue; }
                let Some(data) = p.data().clone().transform(p.abs_transform()) else { continue };
                let b = data.bounds();
                let fill = p.fill().and_then(|f| { if !matches!(f.paint(), usvg::Paint::Color(_)) { ctx.skipped_gradients += 1; } paint_col(f.paint(), f.opacity().get()) });
                let stroke = p.stroke().and_then(|s| paint_col(s.paint(), s.opacity().get()).map(|c| (c, s.width().get() * p.abs_transform().get_scale().0.abs().max(0.01))));
                // a full-canvas filled rectangle first in the file is the background
                if ctx.first_path {
                    ctx.first_path = false;
                    if stroke.is_none() && b.width() >= ctx.page_w * 0.97 && b.height() >= ctx.page_h * 0.97 && data.len() <= 6 { ctx.bg = fill; continue; }
                }
                let subs = flatten(&data);
                if subs.is_empty() { continue; }
                let (w, h) = (b.width().max(0.5), b.height().max(0.5));
                let paths = subs
                    .into_iter()
                    .map(|(pts, closed)| CPath { pts: pts.iter().map(|(x, y)| [(x - b.left()) / w, (y - b.top()) / h]).collect(), closed: closed || fill.is_some(), fill: fill.is_some(), stroke: stroke.is_some() })
                    .collect();
                let mut st = Style::default();
                st.fill = Some(fill.unwrap_or(Col::NONE));
                st.stroke = Some(stroke.map(|s| s.0).unwrap_or(Col::NONE));
                st.width = stroke.map(|s| s.1);
                let id = ctx.doc.new_id();
                ctx.shapes.push(Shape { id, kind: "custom".into(), x: b.left(), y: b.top(), w, h, style: st, group, path: Some(paths), ..Default::default() });
            }
            usvg::Node::Text(t) => {
                let r = t.abs_bounding_box();
                let text: String = t.chunks().iter().map(|c| c.text().to_string()).collect::<Vec<_>>().join("\n");
                if text.trim().is_empty() { continue; }
                let span = t.chunks().first().and_then(|c| c.spans().first());
                let mut st = Style { fill: Some(Col::NONE), stroke: Some(Col::NONE), ..Default::default() };
                if let Some(sp) = span {
                    st.font_size = Some(sp.font_size().get() * t.abs_transform().get_scale().1.abs().max(0.01));
                    st.text = sp.fill().and_then(|f| paint_col(f.paint(), f.opacity().get()));
                    st.bold = sp.font().weight() >= 600;
                    st.italic = !matches!(sp.font().style(), usvg::FontStyle::Normal);
                }
                st.align = Some(HAlign::Left);
                let id = ctx.doc.new_id();
                ctx.shapes.push(Shape { id, kind: "text".into(), x: r.left() - 4.0, y: r.top() - 3.0, w: r.width() * 1.4 + 24.0, h: r.height() + 6.0, text, style: st, group, ..Default::default() });
            }
            usvg::Node::Image(img) => {
                let bytes = match img.kind() { usvg::ImageKind::PNG(b) | usvg::ImageKind::JPEG(b) | usvg::ImageKind::GIF(b) | usvg::ImageKind::WEBP(b) => b.clone(), _ => continue };
                use base64::Engine;
                let Ok(dec) = image::load_from_memory(&bytes) else { continue };
                let mut png = vec![];
                if dec.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_err() { continue; }
                let r = img.abs_bounding_box();
                let id = ctx.doc.new_id();
                ctx.shapes.push(Shape { id, kind: "image".into(), x: r.left(), y: r.top(), w: r.width(), h: r.height(), image: Some(base64::engine::general_purpose::STANDARD.encode(png)), style: crate::library::default_style("image"), group, ..Default::default() });
            }
        }
    }
}

pub fn import(bytes: &[u8], name: &str) -> Result<Imported, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_data(bytes, &opt).map_err(|e| format!("couldn't read SVG: {e}"))?;
    let size = tree.size();
    let mut doc = Document { pages: vec![], ..Document::default() };
    let pid = doc.new_id();
    let mut ctx = Ctx { doc, shapes: vec![], page_w: size.width(), page_h: size.height(), bg: None, first_path: true, skipped_gradients: 0 };
    walk(&mut ctx, tree.root(), None, true);
    let mut doc = ctx.doc;
    let n = ctx.shapes.len();
    doc.pages.push(Page { id: pid, name: name.to_string(), width: size.width().ceil().max(200.0), height: size.height().ceil().max(200.0), bg: ctx.bg, shapes: ctx.shapes, avoid_shapes: false, ..Default::default() });
    doc.fix_ids();
    let mut notes = vec![format!("{n} shapes imported from SVG")];
    if ctx.skipped_gradients > 0 { notes.push(format!("{} gradient fill(s) flattened to a solid colour", ctx.skipped_gradients)); }
    Ok(Imported { doc, notes })
}
