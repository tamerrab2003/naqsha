//! Geometry helpers: shape transforms, connection points, connector routing, hit-testing.
use crate::library;
use crate::model::*;
use egui::{pos2, vec2, Pos2, Rect, Vec2};

pub fn rot(v: Vec2, deg: f32) -> Vec2 {
    let (s, c) = deg.to_radians().sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Local (0..w, 0..h) -> world.
pub fn l2w(s: &Shape, p: Pos2) -> Pos2 {
    let fx = if s.flip_h { -1.0 } else { 1.0 };
    let fy = if s.flip_v { -1.0 } else { 1.0 };
    let v = vec2((p.x - s.w / 2.0) * fx, (p.y - s.h / 2.0) * fy);
    let (cx, cy) = s.center();
    pos2(cx, cy) + rot(v, s.rot)
}

/// World -> local offset from centre, in the shape's rotated (unflipped) frame.
pub fn w2c(s: &Shape, p: Pos2) -> Vec2 {
    let (cx, cy) = s.center();
    rot(p - pos2(cx, cy), -s.rot)
}

pub fn contains(s: &Shape, p: Pos2, pad: f32) -> bool {
    let v = w2c(s, p);
    v.x.abs() <= s.w / 2.0 + pad && v.y.abs() <= s.h / 2.0 + pad
}

/// The shape's closed outlines in local coordinates; empty for shapes that are just their box
/// (icons, pictures, text).
pub fn outline(s: &Shape) -> Vec<Vec<Pos2>> {
    if library::is_icon(&s.kind) || s.kind == "image" { return vec![]; }
    library::shape_parts(s)
        .into_iter()
        .filter(|pt| pt.closed && pt.pts.len() >= 3 && matches!(pt.kind, library::PK::Main | library::PK::Fill | library::PK::Hit))
        .map(|pt| pt.pts)
        .collect()
}

/// Is `p` inside the shape's real outline (not just its bounding box), at least `inset` from its edge?
pub fn inside_outline(s: &Shape, polys: &[Vec<Pos2>], p: Pos2, inset: f32) -> bool {
    let v = w2c(s, p);
    if v.x.abs() > s.w / 2.0 - inset || v.y.abs() > s.h / 2.0 - inset { return false; }
    if polys.is_empty() { return true; }
    let fx = if s.flip_h { -1.0 } else { 1.0 };
    let fy = if s.flip_v { -1.0 } else { 1.0 };
    let q = pos2(v.x * fx + s.w / 2.0, v.y * fy + s.h / 2.0);
    polys.iter().any(|poly| {
        let n = poly.len();
        let mut inside = false;
        let mut dmin = f32::MAX;
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) { inside = !inside; }
            dmin = dmin.min(dist_seg(q, a, b));
        }
        inside && dmin > inset
    })
}

pub fn bbox(s: &Shape) -> Rect {
    if s.rot == 0.0 {
        return Rect::from_min_size(pos2(s.x, s.y), vec2(s.w, s.h));
    }
    let pts = [pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)].map(|p| l2w(s, p));
    Rect::from_points(&pts)
}

#[derive(Clone, Copy, Debug)]
pub struct Port {
    pub pos: Pos2,
    pub dir: Vec2,
}

fn axis(v: Vec2) -> Vec2 {
    if v.x.abs() >= v.y.abs() { vec2(v.x.signum(), 0.0) } else { vec2(0.0, v.y.signum()) }
}

pub fn ports(s: &Shape) -> Vec<Port> {
    library::ports(&s.kind)
        .iter()
        .map(|&(nx, ny)| {
            // icon shapes carry their label underneath: attach bottom connectors below the label
            let ly = if ny >= 0.999 && (library::is_icon(&s.kind) || s.kind == "image") && !s.text.is_empty() { s.h + 24.0 } else { ny * s.h };
            let pos = l2w(s, pos2(nx * s.w, ly));
            let mut d = if nx <= 0.001 { vec2(-1.0, 0.0) } else if nx >= 0.999 { vec2(1.0, 0.0) } else if ny <= 0.001 { vec2(0.0, -1.0) } else if ny >= 0.999 { vec2(0.0, 1.0) } else { axis(vec2(nx - 0.5, ny - 0.5)) };
            if s.flip_h { d.x = -d.x; }
            if s.flip_v { d.y = -d.y; }
            Port { pos, dir: rot(d, s.rot) }
        })
        .collect()
}

fn end_hint(page: &Page, e: &End) -> Pos2 {
    match *e {
        End::Free { x, y } => pos2(x, y),
        End::Glued { shape, .. } => page.shape(shape).map(|s| { let (x, y) = s.center(); pos2(x, y) }).unwrap_or_default(),
    }
}

/// Resolve an end to a point + outgoing direction (None for free ends).
pub fn resolve_end(page: &Page, e: &End, other: &End) -> (Pos2, Option<Vec2>) {
    let hint = end_hint(page, other);
    let (p, d, _) = resolve_at(page, e, hint);
    (p, d)
}

/// Resolve an end towards `hint`; also returns the chosen port index (-1 if none).
pub fn resolve_at(page: &Page, e: &End, hint: Pos2) -> (Pos2, Option<Vec2>, i32) {
    match *e {
        End::Free { x, y } => (pos2(x, y), None, -1),
        End::Glued { shape, port } => {
            let Some(s) = page.shape(shape) else { return (end_hint(page, e), None, -1) };
            let ps = ports(s);
            if ps.is_empty() {
                let (x, y) = s.center();
                return (pos2(x, y), None, -1);
            }
            if port >= 0 && (port as usize) < ps.len() {
                let p = ps[port as usize];
                return (p.pos, Some(p.dir), port);
            }
            // Dynamic glue: choose the port whose direction faces the hint, then nearest.
            let (i, best) = ps
                .iter()
                .enumerate()
                .min_by(|a, b| score(a.1, hint).partial_cmp(&score(b.1, hint)).unwrap())
                .map(|(i, p)| (i, *p))
                .unwrap();
            (best.pos, Some(best.dir), i as i32)
        }
    }
}

fn score(p: &Port, hint: Pos2) -> f32 {
    let to = hint - p.pos;
    let d = to.length();
    let facing = if d > 0.0 { p.dir.dot(to / d) } else { 0.0 };
    d * (1.6 - facing * 0.6)
}

fn simplify(pts: Vec<Pos2>) -> Vec<Pos2> { crate::route::simplify(pts) }

/// Length of the straight stub out of a port: 20px, but never more than half the gap between two
/// ports that face each other (longer stubs would cross and force a detour).
pub fn stub_len(a: Pos2, ad: Vec2, b: Pos2, bd: Vec2) -> f32 {
    let (ad, bd) = (axis(ad), axis(bd));
    let gap = (b - a).dot(ad);
    if ad.dot(bd) < -0.99 && gap > 0.0 { (gap / 2.0).min(20.0) } else { 20.0 }
}

fn ortho(a: Pos2, ad: Vec2, b: Pos2, bd: Vec2) -> Vec<Pos2> {
    let g = stub_len(a, ad, b, bd);
    let ad = axis(ad);
    let bd = axis(bd);
    let a1 = a + ad * g;
    let b1 = b + bd * g;
    let mut pts = vec![a, a1];
    let ah = ad.x != 0.0;
    let bh = bd.x != 0.0;
    if ah && bh {
        let mx = if ad.x == bd.x {
            if ad.x > 0.0 { a1.x.max(b1.x) } else { a1.x.min(b1.x) }
        } else if (ad.x > 0.0 && b1.x < a1.x) || (ad.x < 0.0 && b1.x > a1.x) {
            // facing away: go around through the middle row
            let my = (a.y + b.y) / 2.0;
            pts.push(pos2(a1.x, my));
            pts.push(pos2(b1.x, my));
            pts.push(b1);
            pts.push(b);
            return simplify(pts);
        } else {
            (a1.x + b1.x) / 2.0
        };
        pts.push(pos2(mx, a1.y));
        pts.push(pos2(mx, b1.y));
    } else if !ah && !bh {
        let my = if ad.y == bd.y {
            if ad.y > 0.0 { a1.y.max(b1.y) } else { a1.y.min(b1.y) }
        } else if (ad.y > 0.0 && b1.y < a1.y) || (ad.y < 0.0 && b1.y > a1.y) {
            let mx = (a.x + b.x) / 2.0;
            pts.push(pos2(mx, a1.y));
            pts.push(pos2(mx, b1.y));
            pts.push(b1);
            pts.push(b);
            return simplify(pts);
        } else {
            (a1.y + b1.y) / 2.0
        };
        pts.push(pos2(a1.x, my));
        pts.push(pos2(b1.x, my));
    } else if ah {
        pts.push(pos2(b1.x, a1.y));
    } else {
        pts.push(pos2(a1.x, b1.y));
    }
    pts.push(b1);
    pts.push(b);
    simplify(pts)
}

/// Polyline (or flattened curve) for a connector in world coords.
/// Manual right-angle route through `bends`: elbows are inserted wherever two consecutive
/// points aren't aligned, leaving along `ad` and arriving along `bd`.
fn manual_ortho(a: Pos2, ad: Vec2, bends: &[Pos2], b: Pos2, bd: Vec2) -> Vec<Pos2> {
    let mut pts = vec![a];
    // whether the next leg should preferably be horizontal
    let mut h = axis(ad).x != 0.0;
    let targets: Vec<Pos2> = bends.iter().copied().chain(std::iter::once(b)).collect();
    let n = targets.len();
    for (k, q) in targets.into_iter().enumerate() {
        let p = *pts.last().unwrap();
        if (p.x - q.x).abs() < 0.5 || (p.y - q.y).abs() < 0.5 {
            let seg_h = (p.y - q.y).abs() < 0.5 && (p.x - q.x).abs() >= 0.5;
            pts.push(q);
            h = !seg_h;
            continue;
        }
        // two legs with an elbow; the leg into `b` runs along b's port axis
        let first_h = if k == n - 1 { axis(bd).x == 0.0 } else { h };
        pts.push(if first_h { pos2(q.x, p.y) } else { pos2(p.x, q.y) });
        pts.push(q);
        h = first_h; // second leg was !first_h, so the next one turns back
    }
    crate::route::simplify(pts)
}

/// The manual route of a connector, or None when it has no bends or the bends no longer make
/// sense (it would leave/enter a shape from the wrong side, or cut through an end shape).
pub fn manual_route(page: &Page, c: &Conn) -> Option<Vec<Pos2>> {
    if !c.has_bends() { return None; }
    let bends: Vec<Pos2> = c.bends.as_ref().unwrap().iter().map(|p| pos2(p[0], p[1])).collect();
    let (a, ad, _) = resolve_at(page, &c.from, bends[0]);
    let (b, bd, _) = resolve_at(page, &c.to, *bends.last().unwrap());
    let pts = match c.routing {
        Routing::Orthogonal => {
            let ad = ad.unwrap_or_else(|| axis(bends[0] - a + vec2(0.001, 0.0)));
            let bd = bd.unwrap_or_else(|| axis(*bends.last().unwrap() - b + vec2(0.001, 0.0)));
            manual_ortho(a, ad, &bends, b, bd)
        }
        _ => std::iter::once(a).chain(bends).chain(std::iter::once(b)).collect(),
    };
    if pts.len() < 2 { return None; }
    let n = pts.len();
    // must leave along the port's outward direction and arrive against the target port's
    // …and actually get clear of the shape: a first/last leg of a few pixels hugs its outline
    if let Some(d) = ad { if (pts[1] - pts[0]).dot(d) < 6.0f32.min((b - a).length() / 3.0) { return None; } }
    if let Some(d) = bd { if (pts[n - 1] - pts[n - 2]).dot(d) > -(6.0f32.min((b - a).length() / 3.0)) { return None; } }
    // no segment may pass through an end shape
    for e in [c.from, c.to] {
        if let End::Glued { shape, .. } = e {
            if let Some(s) = page.shape(shape) {
                // test against the real outline: a triangle's slanted-side port sits inside its box
                if s.w > 4.0 && s.h > 4.0 {
                    let r = bbox(s);
                    let polys = std::cell::OnceCell::new();
                    for w in pts.windows(2) {
                        if !r.intersects(Rect::from_two_pos(w[0], w[1]).expand(1.0)) { continue; }
                        if seg_hits_shape(w[0], w[1], s, polys.get_or_init(|| outline(s))) { return None; }
                    }
                }
            }
        }
    }
    Some(pts)
}

/// Does the segment p-q pass through the interior of the shape?
fn seg_hits_shape(p: Pos2, q: Pos2, s: &Shape, polys: &[Vec<Pos2>]) -> bool {
    let steps = (((q - p).length() / 3.0).ceil() as usize).clamp(1, 600);
    (0..=steps).any(|i| inside_outline(s, polys, p + (q - p) * (i as f32 / steps as f32), 2.0))
}

/// When two facing ports are only a few pixels out of line, slide a dynamically glued end along
/// its shape's side so the connector is perfectly straight instead of showing a tiny jog.
fn straighten(page: &Page, c: &Conn, a: &mut Pos2, ad: Vec2, b: &mut Pos2, bd: Vec2) {
    if ad.dot(bd) > -0.99 { return; }
    let vertical = ad.x.abs() < 0.01;
    let (pa, pb) = if vertical { (a.x, b.x) } else { (a.y, b.y) };
    let off = pb - pa;
    if off.abs() < 0.01 || off.abs() > 16.0 || (*b - *a).dot(ad) <= 0.0 { return; }
    let span = |e: &End| -> Option<(f32, f32)> {
        match *e {
            End::Glued { shape, port: -1 } => page.shape(shape).filter(|s| s.rot == 0.0).map(|s| { let r = bbox(s); if vertical { (r.min.x + 4.0, r.max.x - 4.0) } else { (r.min.y + 4.0, r.max.y - 4.0) } }),
            End::Free { .. } => Some((f32::MIN, f32::MAX)),
            _ => None,
        }
    };
    if let Some((lo, hi)) = span(&c.to) { if pa >= lo && pa <= hi { if vertical { b.x = pa } else { b.y = pa } return; } }
    if let Some((lo, hi)) = span(&c.from) { if pb >= lo && pb <= hi { if vertical { a.x = pb } else { a.y = pb } } }
}

pub fn route(page: &Page, c: &Conn) -> Vec<Pos2> {
    if let Some(pts) = manual_route(page, c) { return pts; }
    let (mut a, ad) = resolve_end(page, &c.from, &c.to);
    let (mut b, bd) = resolve_end(page, &c.to, &c.from);
    if c.routing == Routing::Orthogonal { if let (Some(x), Some(y)) = (ad, bd) { straighten(page, c, &mut a, x, &mut b, y); } }
    let d = b - a;
    let ad = ad.unwrap_or_else(|| axis(if d.length() > 0.0 { d } else { vec2(1.0, 0.0) }));
    let bd = bd.unwrap_or_else(|| axis(if d.length() > 0.0 { -d } else { vec2(-1.0, 0.0) }));
    match c.routing {
        Routing::Straight => vec![a, b],
        Routing::Orthogonal => {
            if page.avoid_shapes {
                let obstacles: Vec<Rect> = page
                    .shapes
                    .iter()
                    .filter(|s| page.shape_shown(s) && !library::is_container(&s.kind) && !library::is_textlike(&s.kind) && s.kind != "phase" && s.kind != "uml_lifeline")
                    .map(|s| {
                        let mut b = bbox(s);
                        if (library::is_icon(&s.kind) || s.kind == "image") && !s.text.is_empty() { b.max.y += 22.0; }
                        b
                    })
                    .collect();
                if let Some(r) = crate::route::avoid(a, ad, b, bd, &obstacles) { return r; }
            }
            ortho(a, ad, b, bd)
        }
        Routing::Curved => {
            let k = (d.length() / 2.5).max(40.0);
            let (p1, p2) = (a + ad * k, b + bd * k);
            (0..=32)
                .map(|i| {
                    let t = i as f32 / 32.0;
                    let u = 1.0 - t;
                    (a.to_vec2() * (u * u * u) + p1.to_vec2() * (3.0 * u * u * t) + p2.to_vec2() * (3.0 * u * t * t) + b.to_vec2() * (t * t * t)).to_pos2()
                })
                .collect()
        }
    }
}

pub fn poly_len(pts: &[Pos2]) -> f32 { pts.windows(2).map(|w| (w[1] - w[0]).length()).sum() }

pub fn poly_mid(pts: &[Pos2]) -> Pos2 {
    let total = poly_len(pts);
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = (w[1] - w[0]).length();
        if acc + l >= total / 2.0 && l > 0.0 {
            return w[0] + (w[1] - w[0]) * ((total / 2.0 - acc) / l);
        }
        acc += l;
    }
    pts.first().copied().unwrap_or_default()
}

pub fn dist_seg(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_sq();
    let t = if l2 > 0.0 { ((p - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p - (a + ab * t)).length()
}

pub fn dist_poly(p: Pos2, pts: &[Pos2]) -> f32 { pts.windows(2).map(|w| dist_seg(p, w[0], w[1])).fold(f32::MAX, f32::min) }

pub fn snap(v: f32, g: f32) -> f32 { (v / g).round() * g }

pub fn rect_union(a: Option<Rect>, b: Rect) -> Option<Rect> { Some(a.map_or(b, |a| a.union(b))) }

/// Bounding box of everything drawn on a page (shapes and connector routes).
pub fn content_bounds(page: &Page) -> Option<Rect> {
    let mut b: Option<Rect> = None;
    for s in page.shapes.iter().filter(|s| page.shape_shown(s)) { b = rect_union(b, bbox(s)); }
    for c in page.conns.iter().filter(|c| page.conn_shown(c)) {
        for p in route(page, c) { b = rect_union(b, Rect::from_center_size(p, vec2(2.0, 2.0))); }
    }
    b
}

/// How a linked page's preview sits inside shape `s`: (content rect on the sub-page, scale,
/// world position where the content's top-left lands). None for an empty page.
pub fn preview_fit(s: &Shape, sub: &Page) -> Option<(Rect, f32, Pos2)> {
    let content = content_bounds(sub)?.expand(8.0);
    let pb = library::preview_box(s).translate(vec2(s.x, s.y));
    if pb.width() < 4.0 || pb.height() < 4.0 { return None; }
    let k = (pb.width() / content.width()).min(pb.height() / content.height()).min(1.0);
    let size = content.size() * k;
    Some((content, k, pb.center() - size / 2.0))
}
