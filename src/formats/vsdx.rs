//! Microsoft Visio .vsdx (Open Packaging XML) import and export.
//! Visio units are inches with Y pointing up; Naqsha uses 96 px per inch with Y pointing down.
use super::{decode_entities, hex_col, Imported};
use crate::geom;
use crate::library;
use crate::model::*;
use crate::theme;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::io::{Read, Write};

const PX: f32 = 96.0;

// =====================================================================================  IMPORT

type Cells = HashMap<String, (String, String)>; // N -> (V, F)

#[derive(Clone, Default, Debug)]
struct VRow {
    t: String,
    ix: i32,
    del: bool,
    cells: Cells,
}

#[derive(Clone, Default, Debug)]
struct VSection {
    n: String,
    ix: i32,
    del: bool,
    cells: Cells,
    rows: Vec<VRow>,
}

#[derive(Clone, Default, Debug)]
struct VShape {
    id: String,
    typ: String,
    master: Option<String>,
    master_shape: Option<String>,
    line_style: Option<String>,
    fill_style: Option<String>,
    text_style: Option<String>,
    cells: Cells,
    sections: Vec<VSection>,
    text: Option<String>,
    foreign_rel: Option<String>,
    children: Vec<VShape>,
}

fn attr(n: &roxmltree::Node, a: &str) -> Option<String> { n.attributes().find(|x| x.name() == a).map(|x| x.value().to_string()) }
fn kids<'a, 'i>(n: roxmltree::Node<'a, 'i>, tag: &'static str) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> { n.children().filter(move |c| c.is_element() && c.tag_name().name() == tag) }

fn read_cells(n: roxmltree::Node) -> Cells {
    kids(n, "Cell").filter_map(|c| Some((attr(&c, "N")?, (attr(&c, "V").unwrap_or_default(), attr(&c, "F").unwrap_or_default())))).collect()
}

fn text_of(n: roxmltree::Node) -> String {
    let mut s = String::new();
    for d in n.descendants() {
        if d.is_text() {
            // field elements hold their display value as text too
            s.push_str(d.text().unwrap_or(""));
        }
    }
    s.trim_end_matches(['\n', '\r']).replace("\r\n", "\n").replace('\r', "\n").replace('\u{2028}', "\n")
}

fn parse_shape(n: roxmltree::Node) -> VShape {
    let mut s = VShape {
        id: attr(&n, "ID").unwrap_or_default(),
        typ: attr(&n, "Type").unwrap_or_else(|| "Shape".into()),
        master: attr(&n, "Master"),
        master_shape: attr(&n, "MasterShape"),
        line_style: attr(&n, "LineStyle"),
        fill_style: attr(&n, "FillStyle"),
        text_style: attr(&n, "TextStyle"),
        cells: read_cells(n),
        ..Default::default()
    };
    for sec in kids(n, "Section") {
        let mut vs = VSection { n: attr(&sec, "N").unwrap_or_default(), ix: attr(&sec, "IX").and_then(|v| v.parse().ok()).unwrap_or(0), del: attr(&sec, "Del").as_deref() == Some("1"), cells: read_cells(sec), rows: vec![] };
        for (k, r) in kids(sec, "Row").enumerate() {
            vs.rows.push(VRow { t: attr(&r, "T").unwrap_or_default(), ix: attr(&r, "IX").and_then(|v| v.parse().ok()).unwrap_or(k as i32), del: attr(&r, "Del").as_deref() == Some("1"), cells: read_cells(r) });
        }
        s.sections.push(vs);
    }
    if let Some(t) = kids(n, "Text").next() { s.text = Some(text_of(t)); }
    if let Some(fd) = kids(n, "ForeignData").next() {
        s.foreign_rel = kids(fd, "Rel").next().and_then(|r| r.attributes().find(|a| a.name() == "id").map(|a| a.value().to_string()));
    }
    if let Some(sh) = kids(n, "Shapes").next() {
        s.children = kids(sh, "Shape").map(parse_shape).collect();
    }
    s
}

fn find_sub<'a>(list: &'a [VShape], id: &str) -> Option<&'a VShape> {
    for s in list {
        if s.id == id { return Some(s); }
        if let Some(f) = find_sub(&s.children, id) { return Some(f); }
    }
    None
}

/// Shape + its master shape + style chain, for inherited cell lookup.
struct Resolved<'a> {
    me: &'a VShape,
    master: Option<&'a VShape>,
    styles: &'a HashMap<String, VShape>,
}

impl<'a> Resolved<'a> {
    fn style_chain(&self, which: &str) -> Vec<&'a VShape> {
        let pick = |s: &VShape| match which { "line" => s.line_style.clone(), "fill" => s.fill_style.clone(), _ => s.text_style.clone() };
        let mut id = pick(self.me).or_else(|| self.master.and_then(|m| pick(m)));
        let mut out = vec![];
        for _ in 0..8 {
            let Some(i) = id else { break };
            let Some(st) = self.styles.get(&i) else { break };
            out.push(st);
            let next = pick(st);
            if next.as_deref() == Some(i.as_str()) { break; }
            id = next;
        }
        out
    }
    fn cell(&self, name: &str) -> Option<&'a str> {
        let get = |s: &'a VShape| s.cells.get(name).map(|c| c.0.as_str()).filter(|v| !v.is_empty());
        if let Some(v) = get(self.me) { return Some(v); }
        if let Some(m) = self.master { if let Some(v) = get(m) { return Some(v); } }
        let which = if name.starts_with("Line") || name.contains("Arrow") { "line" } else if name.starts_with("Fill") || name.starts_with("Shdw") { "fill" } else { "text" };
        self.style_chain(which).into_iter().find_map(get)
    }
    fn num(&self, name: &str) -> Option<f32> { self.cell(name).and_then(|v| v.parse::<f32>().ok()) }
    fn has(&self, name: &str) -> bool { self.me.cells.contains_key(name) || self.master.map_or(false, |m| m.cells.contains_key(name)) }
    /// First row of a named section (Character / Paragraph), with inheritance.
    fn row_cell(&self, sec: &str, name: &str) -> Option<&'a str> {
        let get = |s: &'a VShape| s.sections.iter().find(|x| x.n == sec).and_then(|x| x.rows.first()).and_then(|r| r.cells.get(name)).map(|c| c.0.as_str()).filter(|v| !v.is_empty());
        get(self.me).or_else(|| self.master.and_then(get)).or_else(|| self.style_chain("text").into_iter().find_map(get))
    }
    fn text(&self) -> String { self.me.text.clone().or_else(|| self.master.and_then(|m| m.text.clone())).unwrap_or_default() }
    /// Geometry sections, merging the instance's overrides into the master's sections.
    fn geometry(&self) -> Vec<VSection> {
        let mut secs: Vec<VSection> = self.master.map(|m| m.sections.iter().filter(|s| s.n == "Geometry").cloned().collect()).unwrap_or_default();
        for own in self.me.sections.iter().filter(|s| s.n == "Geometry") {
            if let Some(base) = secs.iter_mut().find(|s| s.ix == own.ix) {
                if own.del { base.del = true; continue; }
                for (k, v) in &own.cells { base.cells.insert(k.clone(), v.clone()); }
                for r in &own.rows {
                    if let Some(br) = base.rows.iter_mut().find(|b| b.ix == r.ix) {
                        if r.del { br.del = true; continue; }
                        if !r.t.is_empty() { br.t = r.t.clone(); }
                        for (k, v) in &r.cells { br.cells.insert(k.clone(), v.clone()); }
                    } else {
                        base.rows.push(r.clone());
                    }
                }
                base.rows.sort_by_key(|r| r.ix);
            } else {
                secs.push(own.clone());
            }
        }
        secs.retain(|s| !s.del);
        secs.sort_by_key(|s| s.ix);
        secs
    }
}

/// 2D affine (y-up inches): x' = a x + c y + e, y' = b x + d y + f
#[derive(Clone, Copy, Debug)]
struct Aff { a: f32, b: f32, c: f32, d: f32, e: f32, f: f32 }
impl Aff {
    const ID: Aff = Aff { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };
    fn mul(&self, o: &Aff) -> Aff {
        Aff { a: self.a * o.a + self.c * o.b, b: self.b * o.a + self.d * o.b, c: self.a * o.c + self.c * o.d, d: self.b * o.c + self.d * o.d, e: self.a * o.e + self.c * o.f + self.e, f: self.b * o.e + self.d * o.f + self.f }
    }
    fn ap(&self, x: f32, y: f32) -> (f32, f32) { (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f) }
    fn shape(pinx: f32, piny: f32, ang: f32, fx: bool, fy: bool, lx: f32, ly: f32) -> Aff {
        let (s, c) = ang.sin_cos();
        let sx = if fx { -1.0 } else { 1.0 };
        let sy = if fy { -1.0 } else { 1.0 };
        let t = Aff { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: pinx, f: piny };
        let r = Aff { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 };
        let f = Aff { a: sx, b: 0.0, c: 0.0, d: sy, e: 0.0, f: 0.0 };
        let l = Aff { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: -lx, f: -ly };
        t.mul(&r).mul(&f).mul(&l)
    }
}

fn vcolor(v: Option<&str>) -> Option<Col> {
    let v = v?.trim();
    if v.starts_with('#') { return hex_col(v); }
    // legacy colour-table indices
    let idx: usize = v.parse().ok()?;
    const T: [&str; 24] = ["#000000", "#FFFFFF", "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#FF00FF", "#00FFFF", "#800000", "#008000", "#000080", "#808000", "#800080", "#008080", "#C0C0C0", "#E6E6E6", "#CDCDCD", "#B3B3B3", "#9A9A9A", "#808080", "#666666", "#4D4D4D", "#333333", "#1A1A1A"];
    T.get(idx).and_then(|h| hex_col(h))
}

fn row_num(r: &VRow, n: &str) -> f32 { r.cells.get(n).and_then(|c| c.0.parse().ok()).unwrap_or(0.0) }

/// Circle arc through three points, sampled.
fn arc3(p0: (f32, f32), pm: (f32, f32), p1: (f32, f32), out: &mut Vec<(f32, f32)>) {
    let (ax, ay, bx, by, cx, cy) = (p0.0, p0.1, pm.0, pm.1, p1.0, p1.1);
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-9 { out.push(p1); return; }
    let ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d;
    let uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d;
    let r = ((ax - ux).powi(2) + (ay - uy).powi(2)).sqrt();
    let a0 = (ay - uy).atan2(ax - ux);
    let am = (by - uy).atan2(bx - ux);
    let a1 = (cy - uy).atan2(cx - ux);
    let norm = |a: f32| { let mut a = a; while a < 0.0 { a += 2.0 * PI; } while a >= 2.0 * PI { a -= 2.0 * PI; } a };
    // go from a0 to a1 in the direction that passes am
    let ccw_to_end = norm(a1 - a0);
    let ccw_to_mid = norm(am - a0);
    let sweep = if ccw_to_mid <= ccw_to_end { ccw_to_end } else { ccw_to_end - 2.0 * PI };
    let n = ((sweep.abs() / (2.0 * PI)) * 48.0).ceil().max(4.0) as usize;
    for i in 1..=n {
        let a = a0 + sweep * i as f32 / n as f32;
        out.push((ux + r * a.cos(), uy + r * a.sin()));
    }
}

fn formula_numbers(f: &str) -> Vec<f32> {
    let inner = f.split_once('(').map(|x| x.1).unwrap_or("").trim_end_matches(')');
    inner.split(',').filter_map(|t| t.trim().parse::<f32>().ok()).collect()
}

/// Geometry sections → custom paths in local inches (y-up).
fn geometry_paths(secs: &[VSection], w: f32, h: f32) -> Vec<(Vec<(f32, f32)>, bool, bool)> {
    let mut out = vec![];
    for sec in secs {
        let flag = |n: &str| sec.cells.get(n).map_or(false, |c| c.0 == "1");
        if flag("NoShow") { continue; }
        let (fill, line) = (!flag("NoFill"), !flag("NoLine"));
        let mut cur: Vec<(f32, f32)> = vec![];
        let mut paths: Vec<Vec<(f32, f32)>> = vec![];
        let mut last = (0.0, 0.0);
        for r in sec.rows.iter().filter(|r| !r.del) {
            let (x, y) = (row_num(r, "X"), row_num(r, "Y"));
            let rel = r.t.starts_with("Rel");
            let p = if rel { (x * w, y * h) } else { (x, y) };
            match r.t.as_str() {
                "MoveTo" | "RelMoveTo" => { if cur.len() > 1 { paths.push(std::mem::take(&mut cur)); } cur.clear(); cur.push(p); }
                "LineTo" | "RelLineTo" | "SplineStart" | "SplineKnot" => { if cur.is_empty() { cur.push(last); } cur.push(p); }
                "ArcTo" => {
                    if cur.is_empty() { cur.push(last); }
                    let bow = row_num(r, "A");
                    let (dx, dy) = (p.0 - last.0, p.1 - last.1);
                    let l = (dx * dx + dy * dy).sqrt();
                    if bow.abs() < 1e-6 || l < 1e-6 { cur.push(p); } else {
                        let mid = ((last.0 + p.0) / 2.0 - dy / l * bow, (last.1 + p.1) / 2.0 + dx / l * bow);
                        arc3(last, mid, p, &mut cur);
                    }
                }
                "EllipticalArcTo" | "RelEllipticalArcTo" => {
                    if cur.is_empty() { cur.push(last); }
                    let (a, b) = (row_num(r, "A"), row_num(r, "B"));
                    let ctrl = if rel { (a * w, b * h) } else { (a, b) };
                    let ang = row_num(r, "C");
                    let ratio = { let d = row_num(r, "D"); if d.abs() < 1e-6 { 1.0 } else { d } };
                    // map ellipse to circle: rotate by -C, scale minor axis by ratio
                    let (s, c) = (-ang).sin_cos();
                    let to = |q: (f32, f32)| { let x = q.0 * c - q.1 * s; let y = q.0 * s + q.1 * c; (x, y * ratio) };
                    let (si, ci) = ang.sin_cos();
                    let back = |q: (f32, f32)| { let x = q.0; let y = q.1 / ratio; (x * ci - y * si, x * si + y * ci) };
                    let mut tmp = vec![];
                    arc3(to(last), to(ctrl), to(p), &mut tmp);
                    cur.extend(tmp.into_iter().map(back));
                }
                "Ellipse" => {
                    if cur.len() > 1 { paths.push(std::mem::take(&mut cur)); }
                    let (a, b, c2, d) = (row_num(r, "A"), row_num(r, "B"), row_num(r, "C"), row_num(r, "D"));
                    let mut e = vec![];
                    for i in 0..=48 {
                        let t = i as f32 / 48.0 * 2.0 * PI;
                        e.push((x + t.cos() * (a - x) + t.sin() * (c2 - x), y + t.cos() * (b - y) + t.sin() * (d - y)));
                    }
                    paths.push(e);
                    cur.clear();
                }
                "RelCubBezTo" | "RelQuadBezTo" => {
                    if cur.is_empty() { cur.push(last); }
                    let p1 = (row_num(r, "A") * w, row_num(r, "B") * h);
                    let cubic = r.t == "RelCubBezTo";
                    let p2 = if cubic { (row_num(r, "C") * w, row_num(r, "D") * h) } else { p1 };
                    for i in 1..=16 {
                        let t = i as f32 / 16.0;
                        let u = 1.0 - t;
                        let q = if cubic {
                            (u * u * u * last.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p.0, u * u * u * last.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p.1)
                        } else {
                            (u * u * last.0 + 2.0 * u * t * p1.0 + t * t * p.0, u * u * last.1 + 2.0 * u * t * p1.1 + t * t * p.1)
                        };
                        cur.push(q);
                    }
                }
                "PolylineTo" | "NURBSTo" => {
                    if cur.is_empty() { cur.push(last); }
                    let f = r.cells.get(if r.t == "PolylineTo" { "A" } else { "E" }).map(|c| c.1.clone()).unwrap_or_default();
                    let nums = formula_numbers(&f);
                    if r.t == "PolylineTo" && nums.len() >= 2 {
                        let (xt, yt) = (nums[0], nums[1]);
                        for pair in nums[2..].chunks(2) {
                            if pair.len() == 2 { cur.push((if xt == 0.0 { pair[0] * w } else { pair[0] }, if yt == 0.0 { pair[1] * h } else { pair[1] })); }
                        }
                    } else if nums.len() >= 4 {
                        // approximate the NURBS by its control polygon
                        let (xt, yt) = (nums[2], nums[3]);
                        for q in nums[4..].chunks(4) {
                            if q.len() >= 2 { cur.push((if xt == 0.0 { q[0] * w } else { q[0] }, if yt == 0.0 { q[1] * h } else { q[1] })); }
                        }
                    }
                    cur.push(p);
                }
                _ => {}
            }
            if r.t != "Ellipse" { last = p; }
        }
        if cur.len() > 1 { paths.push(cur); }
        for pth in paths {
            let closed = pth.len() > 2 && { let (f, l) = (pth[0], pth[pth.len() - 1]); (f.0 - l.0).abs() < 1e-4 && (f.1 - l.1).abs() < 1e-4 };
            out.push((pth, fill && closed, line));
        }
    }
    out
}

struct PageCtx<'a> {
    masters: &'a HashMap<String, Vec<VShape>>,
    styles: &'a HashMap<String, VShape>,
    media: &'a HashMap<String, Vec<u8>>,
    page_h: f32,
    layers: Vec<Id>,
}

struct Out {
    shapes: Vec<Shape>,
    conns: Vec<(String, Conn)>,
    idmap: HashMap<String, Id>,
    /// (shape id, target page name) of page hyperlinks.
    links: Vec<(Id, String)>,
}

fn style_from(r: &Resolved, st: &mut Style, is_line: bool) {
    let fill_pat = r.num("FillPattern").unwrap_or(1.0);
    // Unresolvable (theme) colours fall back to Visio's own defaults: white fill, black line and text.
    if !is_line {
        st.fill = Some(if fill_pat == 0.0 { Col::NONE } else { vcolor(r.cell("FillForegnd")).map(|mut c| { if let Some(t) = r.num("FillForegndTrans") { c.0[3] = ((1.0 - t) * 255.0) as u8; } c }).unwrap_or(Col([255, 255, 255, 255])) });
    }
    let lp = r.num("LinePattern").unwrap_or(1.0);
    st.stroke = Some(if lp == 0.0 { Col::NONE } else { vcolor(r.cell("LineColor")).unwrap_or(Col([0, 0, 0, 255])) });
    st.dash = match lp as i32 { 0 | 1 => Dash::Solid, 3 | 10 | 16 => Dash::Dotted, _ => Dash::Dashed };
    if let Some(w) = r.num("LineWeight") { st.width = Some((w * PX).clamp(0.25, 24.0)); }
    st.text = Some(vcolor(r.row_cell("Character", "Color")).unwrap_or(Col([0, 0, 0, 255])));
    if let Some(sz) = r.row_cell("Character", "Size").and_then(|v| v.parse::<f32>().ok()) { st.font_size = Some((sz * PX).clamp(5.0, 144.0)); }
    let bits: i32 = r.row_cell("Character", "Style").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) as i32;
    st.bold = bits & 1 != 0;
    st.italic = bits & 2 != 0;
    st.align = r.row_cell("Paragraph", "HorzAlign").and_then(|v| v.parse::<f32>().ok()).map(|v| match v as i32 { 0 => HAlign::Left, 2 => HAlign::Right, _ => HAlign::Center });
    st.valign = r.num("VerticalAlign").map(|v| match v as i32 { 0 => VAlign::Top, 2 => VAlign::Bottom, _ => VAlign::Middle });
    st.shadow = r.num("ShdwPattern").unwrap_or(0.0) > 0.0;
}

fn arrow(v: Option<f32>) -> Arrow {
    match v.unwrap_or(0.0) as i32 { 0 => Arrow::None, 1 | 2 | 3 | 6 | 7 | 8 | 9 => Arrow::Open, 10 | 20 | 21 => Arrow::Circle, 11 | 22 | 23 => Arrow::Diamond, _ => Arrow::Filled }
}

fn import_shape(doc: &mut Document, ctx: &PageCtx, vs: &VShape, parent: &Aff, master_id: Option<&str>, group: Option<Id>, out: &mut Out) {
    if vs.typ == "Guide" { return; }
    let mid = vs.master.as_deref().or(master_id);
    let master = mid.and_then(|m| ctx.masters.get(m)).and_then(|list| match &vs.master_shape { Some(ms) => find_sub(list, ms), None => if vs.master.is_some() { list.first() } else { None } });
    let r = Resolved { me: vs, master, styles: ctx.styles };
    let w = r.num("Width").unwrap_or(0.0);
    let h = r.num("Height").unwrap_or(0.0);
    let layer = r.cell("LayerMember").and_then(|v| v.split(';').next().and_then(|x| x.trim().parse::<usize>().ok())).and_then(|i| ctx.layers.get(i).copied());

    // ---- 1-D shapes become connectors
    if r.has("BeginX") && r.has("EndX") && vs.children.is_empty() {
        let (bx, by) = parent.ap(r.num("BeginX").unwrap_or(0.0), r.num("BeginY").unwrap_or(0.0));
        let (ex, ey) = parent.ap(r.num("EndX").unwrap_or(0.0), r.num("EndY").unwrap_or(0.0));
        let id = doc.new_id();
        let mut st = Style::default();
        style_from(&r, &mut st, true);
        // routing from the stored geometry: 2 points = straight, axis-aligned polyline = right-angle
        let gpaths = geometry_paths(&r.geometry(), w.abs().max(1e-3), h.abs().max(1e-3));
        let pts: Vec<(f32, f32)> = gpaths.first().map(|p| p.0.clone()).unwrap_or_default();
        let axis = pts.windows(2).all(|q| (q[0].0 - q[1].0).abs() < 1e-3 || (q[0].1 - q[1].1).abs() < 1e-3);
        let curvy = gpaths.iter().any(|p| p.0.len() > 12) && !axis;
        // keep the drawn route of right-angle connectors as a manual route
        let bends = if pts.len() > 2 && axis {
            let (pinx, piny) = (r.num("PinX").unwrap_or(0.0), r.num("PinY").unwrap_or(0.0));
            let (lx, ly) = (r.num("LocPinX").unwrap_or(0.0), r.num("LocPinY").unwrap_or(0.0));
            let m = parent.mul(&Aff::shape(pinx, piny, r.num("Angle").unwrap_or(0.0), false, false, lx, ly));
            let world: Vec<[f32; 2]> = pts.iter().map(|(x, y)| { let (px, py) = m.ap(*x, *y); [px * PX, (ctx.page_h - py) * PX] }).collect();
            Some(world[1..world.len() - 1].to_vec())
        } else { None };
        let c = Conn {
            id,
            from: End::Free { x: bx * PX, y: (ctx.page_h - by) * PX },
            to: End::Free { x: ex * PX, y: (ctx.page_h - ey) * PX },
            routing: if pts.len() > 2 && axis { Routing::Orthogonal } else if curvy { Routing::Curved } else { Routing::Straight },
            style: st,
            arrow_start: arrow(r.num("BeginArrow")),
            arrow_end: arrow(r.num("EndArrow")),
            text: r.text(),
            layer,
            bends,
            ..Default::default()
        };
        out.idmap.insert(vs.id.clone(), id);
        out.conns.push((vs.id.clone(), c));
        return;
    }

    let pinx = r.num("PinX").unwrap_or(w / 2.0);
    let piny = r.num("PinY").unwrap_or(h / 2.0);
    let lx = r.num("LocPinX").unwrap_or(w / 2.0);
    let ly = r.num("LocPinY").unwrap_or(h / 2.0);
    let ang = r.num("Angle").unwrap_or(0.0);
    let fx = r.num("FlipX").unwrap_or(0.0) != 0.0;
    let fy = r.num("FlipY").unwrap_or(0.0) != 0.0;
    let m = parent.mul(&Aff::shape(pinx, piny, ang, fx, fy, lx, ly));

    let is_group = vs.typ == "Group" && !vs.children.is_empty();
    let gid = if is_group { Some(group.unwrap_or_else(|| doc.new_id())) } else { group };

    let geo = r.geometry();
    let paths = geometry_paths(&geo, w, h);
    let text = r.text();
    let image = vs.foreign_rel.as_ref().and_then(|rel| ctx.media.get(rel));
    let visible = !paths.is_empty() || !text.trim().is_empty() || image.is_some();
    if visible && w.abs() > 1e-5 && h.abs() > 1e-5 || (visible && !paths.is_empty()) {
        let (ww, hh) = (w.abs().max(0.01), h.abs().max(0.01));
        let (cx, cy) = m.ap(w / 2.0, h / 2.0);
        let det = m.a * m.d - m.b * m.c;
        let flip = det < 0.0;
        let theta = if flip { (-m.b).atan2(-m.a) } else { m.b.atan2(m.a) };
        let id = doc.new_id();
        let mut st = Style::default();
        style_from(&r, &mut st, false);
        let mut s = Shape {
            id,
            kind: "custom".into(),
            x: cx * PX - ww * PX / 2.0,
            y: (ctx.page_h - cy) * PX - hh * PX / 2.0,
            w: ww * PX,
            h: hh * PX,
            rot: (-theta.to_degrees()).rem_euclid(360.0),
            flip_h: flip,
            text,
            style: st,
            group: gid,
            layer,
            ..Default::default()
        };
        if let Some(bytes) = image {
            use base64::Engine;
            s.kind = "image".into();
            s.image = Some(base64::engine::general_purpose::STANDARD.encode(bytes));
        } else if paths.is_empty() {
            s.kind = "text".into();
            s.style.fill.get_or_insert(Col::NONE);
            s.style.stroke = Some(Col::NONE);
        } else {
            s.path = Some(paths.into_iter().map(|(pts, fill, line)| CPath { pts: pts.iter().map(|(x, y)| [x / ww, 1.0 - y / hh]).collect(), closed: fill || (pts.len() > 2 && { let (a, b) = (pts[0], pts[pts.len() - 1]); (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4 }), fill, stroke: line }).collect());
        }
        // explicit text block
        if let (Some(tw), Some(th)) = (r.num("TxtWidth"), r.num("TxtHeight")) {
            let tpx = r.num("TxtPinX").unwrap_or(w / 2.0);
            let tpy = r.num("TxtPinY").unwrap_or(h / 2.0);
            let tlx = r.num("TxtLocPinX").unwrap_or(tw / 2.0);
            let tly = r.num("TxtLocPinY").unwrap_or(th / 2.0);
            let tang = r.num("TxtAngle").unwrap_or(0.0);
            let vert = (tang.abs() - PI / 2.0).abs() < 0.2;
            let (bw, bh) = if vert { (th, tw) } else { (tw, th) };
            let (cxl, cyl) = (tpx - tlx + tw / 2.0, tpy - tly + th / 2.0);
            let default = (tw - w).abs() < 1e-3 && (th - h).abs() < 1e-3 && (cxl - w / 2.0).abs() < 1e-3 && (cyl - h / 2.0).abs() < 1e-3;
            if !default && tw.abs() > 1e-4 && th.abs() > 1e-4 {
                s.tbox = Some([(cxl - bw.abs() / 2.0) * PX, (h - cyl - bh.abs() / 2.0) * PX, bw.abs() * PX, bh.abs() * PX]);
                s.tvert = vert;
            }
        }
        // hyperlink to another page of this drawing: resolved to a page link once all pages are read
        if let Some(sub) = vs.sections.iter().filter(|x| x.n == "Hyperlink").flat_map(|x| x.rows.iter()).find_map(|r| {
            let addr = r.cells.get("Address").map(|c| c.0.trim().to_string()).unwrap_or_default();
            r.cells.get("SubAddress").map(|c| c.0.clone()).filter(|v| !v.trim().is_empty() && addr.is_empty())
        }) {
            // "Page/Shape" addresses point at a page; keep the page part
            let page = sub.split('/').next().unwrap_or(&sub).trim().to_string();
            out.links.push((id, page));
        }
        // user-defined shape data
        for sec in vs.sections.iter().filter(|x| x.n == "Property") {
            for row in &sec.rows {
                let label = row.cells.get("Label").map(|c| c.0.clone()).filter(|v| !v.is_empty()).unwrap_or_else(|| format!("Prop {}", row.ix));
                if let Some(v) = row.cells.get("Value") { if !v.0.is_empty() { s.data.push((label, v.0.clone())); } }
            }
        }
        out.idmap.insert(vs.id.clone(), id);
        if is_group {
            // a group's own geometry sits behind its members, its text in front of them
            let mut text_part = s.clone();
            s.text.clear();
            out.shapes.push(s);
            for ch in &vs.children { import_shape(doc, ctx, ch, &m, mid, gid, out); }
            if !text_part.text.trim().is_empty() {
                text_part.id = doc.new_id();
                text_part.kind = "text".into();
                text_part.path = None;
                text_part.style.fill = Some(Col::NONE);
                text_part.style.stroke = Some(Col::NONE);
                out.shapes.push(text_part);
            }
            return;
        }
        out.shapes.push(s);
    }
    for ch in &vs.children { import_shape(doc, ctx, ch, &m, mid, gid, out); }
}

fn read_zip_text(z: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str) -> Option<String> {
    let mut f = z.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}
fn read_zip_bytes(z: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str) -> Option<Vec<u8>> {
    let mut f = z.by_name(name).ok()?;
    let mut v = vec![];
    f.read_to_end(&mut v).ok()?;
    Some(v)
}

fn rels(xml: &str) -> HashMap<String, String> {
    let Ok(d) = roxmltree::Document::parse(xml) else { return HashMap::new() };
    d.descendants().filter(|n| n.tag_name().name() == "Relationship").filter_map(|n| Some((attr(&n, "Id")?, attr(&n, "Target")?))).collect()
}
fn rel_id(n: roxmltree::Node) -> Option<String> { kids(n, "Rel").next().and_then(|r| r.attributes().find(|a| a.name() == "id").map(|a| a.value().to_string())) }
fn join(dir: &str, target: &str) -> String {
    if let Some(t) = target.strip_prefix('/') { return t.to_string(); }
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in target.split('/') { if seg == ".." { parts.pop(); } else if seg != "." { parts.push(seg); } }
    parts.join("/")
}

pub fn import(bytes: Vec<u8>) -> Result<Imported, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a .vsdx package: {e}"))?;
    let pages_xml = read_zip_text(&mut z, "visio/pages/pages.xml").ok_or("visio/pages/pages.xml missing — is this a .vsdx (not legacy .vsd)?")?;
    let page_rels = rels(&read_zip_text(&mut z, "visio/pages/_rels/pages.xml.rels").unwrap_or_default());

    // stylesheets
    let mut styles: HashMap<String, VShape> = HashMap::new();
    if let Some(dx) = read_zip_text(&mut z, "visio/document.xml") {
        if let Ok(d) = roxmltree::Document::parse(&dx) {
            for st in d.descendants().filter(|n| n.tag_name().name() == "StyleSheet") {
                let mut v = parse_shape(st);
                v.id = attr(&st, "ID").unwrap_or_default();
                styles.insert(v.id.clone(), v);
            }
        }
    }
    // masters
    let mut masters: HashMap<String, Vec<VShape>> = HashMap::new();
    if let Some(mx) = read_zip_text(&mut z, "visio/masters/masters.xml") {
        let mrels = rels(&read_zip_text(&mut z, "visio/masters/_rels/masters.xml.rels").unwrap_or_default());
        if let Ok(d) = roxmltree::Document::parse(&mx) {
            let list: Vec<(String, String)> = d.descendants().filter(|n| n.tag_name().name() == "Master").filter_map(|m| Some((attr(&m, "ID")?, mrels.get(&rel_id(m)?)?.clone()))).collect();
            for (id, target) in list {
                if let Some(x) = read_zip_text(&mut z, &join("visio/masters", &target)) {
                    if let Ok(md) = roxmltree::Document::parse(&x) {
                        let top = md.descendants().find(|n| n.tag_name().name() == "Shapes");
                        if let Some(top) = top { masters.insert(id, kids(top, "Shape").map(parse_shape).collect()); }
                    }
                }
            }
        }
    }

    let pd = roxmltree::Document::parse(&pages_xml).map_err(|e| e.to_string())?;
    let mut doc = Document { pages: vec![], ..Document::default() };
    let mut skipped_bg = 0;
    let mut links: Vec<(Id, String)> = vec![];
    for pg in pd.descendants().filter(|n| n.tag_name().name() == "Page") {
        if attr(&pg, "Background").as_deref() == Some("1") { skipped_bg += 1; continue; }
        let name = attr(&pg, "NameU").or_else(|| attr(&pg, "Name")).unwrap_or_else(|| "Page".into());
        let sheet = kids(pg, "PageSheet").next();
        let sheet_cells = sheet.map(read_cells).unwrap_or_default();
        let pnum = |n: &str, d: f32| sheet_cells.get(n).and_then(|c| c.0.parse::<f32>().ok()).unwrap_or(d);
        let (pw, ph) = (pnum("PageWidth", 11.0), pnum("PageHeight", 8.5));
        let mut page = Page { id: doc.new_id(), name: name.clone(), width: (pw * PX).round(), height: (ph * PX).round(), line_jumps: true, avoid_shapes: false, ..Default::default() };
        // layers
        let mut layer_ids = vec![];
        if let Some(sh) = sheet {
            for sec in kids(sh, "Section").filter(|s| attr(s, "N").as_deref() == Some("Layer")) {
                for row in kids(sec, "Row") {
                    let c = read_cells(row);
                    let g = |n: &str| c.get(n).map(|x| x.0.clone()).unwrap_or_default();
                    let id = doc.new_id();
                    page.layers.push(Layer { id, name: if g("Name").is_empty() { format!("Layer {}", layer_ids.len() + 1) } else { g("Name") }, visible: g("Visible") != "0", locked: g("Lock") == "1", print: g("Print") != "0" });
                    layer_ids.push(id);
                }
            }
        }
        let Some(target) = rel_id(pg).and_then(|r| page_rels.get(&r).cloned()) else { continue };
        let ppath = join("visio/pages", &target);
        let Some(px) = read_zip_text(&mut z, &ppath) else { continue };
        let fname = ppath.rsplit('/').next().unwrap_or("");
        let prels = rels(&read_zip_text(&mut z, &format!("visio/pages/_rels/{fname}.rels")).unwrap_or_default());
        let mut media = HashMap::new();
        for (rid, t) in &prels {
            let p = join("visio/pages", t);
            if p.contains("media/") { if let Some(b) = read_zip_bytes(&mut z, &p) { media.insert(rid.clone(), b); } }
        }
        let d = roxmltree::Document::parse(&px).map_err(|e| e.to_string())?;
        let root_shapes = d.root_element().children().find(|n| n.tag_name().name() == "Shapes");
        let vshapes: Vec<VShape> = root_shapes.map(|s| kids(s, "Shape").map(parse_shape).collect()).unwrap_or_default();
        let ctx = PageCtx { masters: &masters, styles: &styles, media: &media, page_h: ph, layers: layer_ids };
        let mut out = Out { shapes: vec![], conns: vec![], idmap: HashMap::new(), links: vec![] };
        for vs in &vshapes { import_shape(&mut doc, &ctx, vs, &Aff::ID, None, None, &mut out); }
        // glue
        let mut glue: HashMap<String, (Option<Id>, Option<Id>)> = HashMap::new();
        for c in d.descendants().filter(|n| n.tag_name().name() == "Connect") {
            let (Some(from), Some(to)) = (attr(&c, "FromSheet"), attr(&c, "ToSheet")) else { continue };
            let cell = attr(&c, "FromCell").unwrap_or_default();
            let Some(sid) = out.idmap.get(&to).copied() else { continue };
            if !out.shapes.iter().any(|s| s.id == sid) { continue; }
            let e = glue.entry(from).or_default();
            if cell.starts_with("Begin") { e.0 = Some(sid); } else if cell.starts_with("End") { e.1 = Some(sid); }
        }
        for (vid, mut c) in out.conns {
            if let Some((b, e)) = glue.get(&vid) {
                if let Some(s) = b { c.from = End::Glued { shape: *s, port: -1 }; }
                if let Some(s) = e { c.to = End::Glued { shape: *s, port: -1 }; }
            }
            page.conns.push(c);
        }
        page.shapes = out.shapes;
        links.extend(out.links);
        doc.pages.push(page);
    }
    for (sid, name) in links {
        let Some(pid) = doc.pages.iter().find(|p| p.name.eq_ignore_ascii_case(&name)).map(|p| p.id) else { continue };
        for p in doc.pages.iter_mut() { let here = p.id; if let Some(s) = p.shape_mut(sid) { if here != pid { s.link = Some(pid); } } }
    }
    if doc.pages.is_empty() { return Err("the file has no foreground pages".into()); }
    doc.fix_ids();
    let mut notes = vec![];
    if skipped_bg > 0 { notes.push(format!("{skipped_bg} background page(s) skipped")); }
    Ok(Imported { doc, notes })
}

// =====================================================================================  EXPORT

fn esc(t: &str) -> String { decode_entities(t).replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn cell(o: &mut String, n: &str, v: impl std::fmt::Display) { o.push_str(&format!("<Cell N=\"{n}\" V=\"{v}\"/>")); }
fn cellf(o: &mut String, n: &str, v: impl std::fmt::Display, f: &str) { o.push_str(&format!("<Cell N=\"{n}\" V=\"{v}\" F=\"{f}\"/>")); }
fn hexc(c: egui::Color32) -> String { let [r, g, b, _] = c.to_srgba_unmultiplied(); format!("#{r:02X}{g:02X}{b:02X}") }

const NS: &str = r#"xmlns="http://schemas.microsoft.com/office/visio/2012/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xml:space="preserve""#;

fn arrow_code(a: Arrow) -> i32 { match a { Arrow::None => 0, Arrow::Open => 1, Arrow::Filled => 4, Arrow::Diamond => 11, Arrow::Circle => 10 } }

/// Page contents, page sheet, and embedded pictures (bytes, extension) referenced as rId1..n.
fn page_xml(doc: &Document, page: &Page) -> (String, String, Vec<(Vec<u8>, &'static str)>) {
    let mut media: Vec<(Vec<u8>, &'static str)> = vec![];
    let pal = theme::LIGHT;
    let ph = page.height / PX;
    let mut map: HashMap<Id, usize> = HashMap::new();
    let mut next = 1usize;
    for s in &page.shapes { map.insert(s.id, next); next += 1; }
    let layer_ix = |l: Option<Id>| l.and_then(|l| page.layers.iter().position(|x| x.id == l));
    let mut o = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<PageContents {NS}><Shapes>");
    for s in &page.shapes {
        let vid = map[&s.id];
        let (cx, cy) = s.center();
        let (w, h) = (s.w / PX, s.h / PX);
        let st = &s.style;
        let fill = crate::render::fill_of(s, &pal);
        let stroke = crate::render::stroke_of(st, pal.shape_stroke);
        let name = library::def(&s.kind).name;
        // pictures are stored as Visio "foreign" shapes with the image in the package
        let picture: Option<(Vec<u8>, &'static str)> = (s.kind == "image").then(|| s.image.as_ref()).flatten().and_then(|b64| {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD.decode(b64.as_bytes()).ok()?;
            let ext = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "png" } else if bytes.starts_with(&[0xFF, 0xD8]) { "jpeg" } else { return None };
            Some((bytes, ext))
        });
        let shape_type = if picture.is_some() { "Foreign" } else { "Shape" };
        o.push_str(&format!("<Shape ID=\"{vid}\" NameU=\"{name}.{vid}\" Name=\"{name}.{vid}\" Type=\"{shape_type}\" LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\">"));
        if picture.is_some() {
            cell(&mut o, "ImgOffsetX", 0);
            cell(&mut o, "ImgOffsetY", 0);
            cellf(&mut o, "ImgWidth", w, "Width*1");
            cellf(&mut o, "ImgHeight", h, "Height*1");
        }
        cell(&mut o, "PinX", cx / PX);
        cell(&mut o, "PinY", ph - cy / PX);
        cell(&mut o, "Width", w);
        cell(&mut o, "Height", h);
        cellf(&mut o, "LocPinX", w / 2.0, "Width*0.5");
        cellf(&mut o, "LocPinY", h / 2.0, "Height*0.5");
        cell(&mut o, "Angle", -s.rot.to_radians());
        cell(&mut o, "FlipX", s.flip_h as i32);
        cell(&mut o, "FlipY", s.flip_v as i32);
        cell(&mut o, "LineWeight", st.width.unwrap_or(1.5) / PX);
        if stroke.a() == 0 { cell(&mut o, "LinePattern", 0); } else { cell(&mut o, "LineColor", hexc(stroke)); cell(&mut o, "LinePattern", match st.dash { Dash::Solid => 1, Dash::Dashed => 2, Dash::Dotted => 3 }); }
        if fill.a() == 0 { cell(&mut o, "FillPattern", 0); } else {
            cell(&mut o, "FillForegnd", hexc(fill));
            cell(&mut o, "FillPattern", 1);
            if fill.a() < 255 { cell(&mut o, "FillForegndTrans", 1.0 - fill.a() as f32 / 255.0); }
        }
        if st.shadow {
            cell(&mut o, "ShdwPattern", 1);
            cell(&mut o, "ShdwForegnd", "#000000");
            cell(&mut o, "ShdwForegndTrans", 0.75);
            cell(&mut o, "ShapeShdwType", 1);
            cell(&mut o, "ShapeShdwOffsetX", 0.03);
            cell(&mut o, "ShapeShdwOffsetY", -0.04);
            cell(&mut o, "ShapeShdwBlur", 0.06);
        }
        let tb = library::shape_text_box(s);
        let full = tb.rect.width() >= s.w - 13.0 && tb.rect.height() >= s.h - 9.0 && !tb.rotated;
        if !full {
            let (tw, th) = if tb.rotated { (tb.rect.height(), tb.rect.width()) } else { (tb.rect.width(), tb.rect.height()) };
            cell(&mut o, "TxtPinX", tb.rect.center().x / PX);
            cell(&mut o, "TxtPinY", h - tb.rect.center().y / PX);
            cell(&mut o, "TxtWidth", tw / PX);
            cell(&mut o, "TxtHeight", th / PX);
            cell(&mut o, "TxtLocPinX", tw / PX / 2.0);
            cell(&mut o, "TxtLocPinY", th / PX / 2.0);
            if tb.rotated { cell(&mut o, "TxtAngle", PI / 2.0); }
        }
        if let Some(i) = layer_ix(s.layer) { cell(&mut o, "LayerMember", i); }
        cell(&mut o, "VerticalAlign", match st.valign.unwrap_or(if library::is_icon(&s.kind) { VAlign::Top } else { VAlign::Middle }) { VAlign::Top => 0, VAlign::Middle => 1, VAlign::Bottom => 2 });
        let tc = st.text.map(theme::to_c32).unwrap_or(pal.shape_text);
        o.push_str("<Section N=\"Character\"><Row IX=\"0\">");
        cell(&mut o, "Color", hexc(tc));
        cell(&mut o, "Size", st.font_size.unwrap_or(13.0) / PX);
        cell(&mut o, "Style", (st.bold as i32) | ((st.italic as i32) << 1));
        o.push_str("</Row></Section><Section N=\"Paragraph\"><Row IX=\"0\">");
        cell(&mut o, "HorzAlign", match st.align.unwrap_or(HAlign::Center) { HAlign::Left => 0, HAlign::Center => 1, HAlign::Right => 2 });
        o.push_str("</Row></Section><Section N=\"Connection\">");
        for (i, (nx, ny)) in library::ports(&s.kind).iter().enumerate() {
            o.push_str(&format!("<Row IX=\"{i}\">"));
            cellf(&mut o, "X", nx * w, &format!("Width*{nx}"));
            cellf(&mut o, "Y", (1.0 - ny) * h, &format!("Height*{}", 1.0 - ny));
            o.push_str("</Row>");
        }
        o.push_str("</Section>");
        // a sub-diagram link is a Visio hyperlink to the page (SubAddress = page name)
        if let Some(t) = doc.linked_page(s) {
            o.push_str(&format!("<Section N=\"Hyperlink\"><Row N=\"Row_1\"><Cell N=\"Description\" V=\"{0}\"/><Cell N=\"Address\" V=\"\"/><Cell N=\"SubAddress\" V=\"{0}\"/><Cell N=\"Default\" V=\"1\"/></Row></Section>", esc(&t.name)));
        }
        if s.data.iter().any(|(k, _)| !k.is_empty()) {
            o.push_str("<Section N=\"Property\">");
            for (i, (k, v)) in s.data.iter().enumerate() {
                o.push_str(&format!("<Row N=\"Prop{i}\"><Cell N=\"Label\" V=\"{}\"/><Cell N=\"Value\" V=\"{}\" U=\"STR\"/><Cell N=\"Type\" V=\"0\"/></Row>", esc(k), esc(v)));
            }
            o.push_str("</Section>");
        }
        let parts = library::shape_parts(s);
        let mut gi = 0;
        for p in parts.iter().filter(|p| p.kind != library::PK::Hit && p.pts.len() >= 2) {
            let (no_fill, no_line) = match p.kind { library::PK::Line => (1, 0), library::PK::Fill | library::PK::Glyph | library::PK::Tint => (0, 1), _ => ((!p.closed) as i32, 0) };
            o.push_str(&format!("<Section N=\"Geometry\" IX=\"{gi}\"><Cell N=\"NoFill\" V=\"{no_fill}\"/><Cell N=\"NoLine\" V=\"{no_line}\"/><Cell N=\"NoShow\" V=\"0\"/><Cell N=\"NoSnap\" V=\"0\"/>"));
            let mut pts: Vec<egui::Pos2> = p.pts.clone();
            if p.closed { pts.push(pts[0]); }
            for (k, q) in pts.iter().enumerate() {
                let t = if k == 0 { "RelMoveTo" } else { "RelLineTo" };
                o.push_str(&format!("<Row T=\"{t}\" IX=\"{}\"><Cell N=\"X\" V=\"{}\"/><Cell N=\"Y\" V=\"{}\"/></Row>", k + 1, q.x / s.w.max(0.001), 1.0 - q.y / s.h.max(0.001)));
            }
            o.push_str("</Section>");
            gi += 1;
        }
        if !s.text.is_empty() {
            // UML/ER compartments: Visio has no compartment text, so separators become blank lines
            let t = if library::has_compartments(&s.kind) { s.text.lines().map(|l| if l.trim() == "--" { "" } else { l }).collect::<Vec<_>>().join("\n") } else { s.text.clone() };
            o.push_str(&format!("<Text>{}</Text>", esc(&t)));
        }
        if let Some((bytes, ext)) = picture {
            media.push((bytes, ext));
            let comp = if ext == "png" { "PNG" } else { "JPEG" };
            o.push_str(&format!("<ForeignData ForeignType=\"Bitmap\" CompressionType=\"{comp}\"><Rel r:id=\"rId{}\"/></ForeignData>", media.len()));
        }
        o.push_str("</Shape>");
    }
    let mut connects = String::new();
    for c in &page.conns {
        let vid = next;
        next += 1;
        let route = geom::route(page, c);
        if route.len() < 2 { continue; }
        let (b, e) = (route[0], route[route.len() - 1]);
        let (bx, by, ex, ey) = (b.x / PX, ph - b.y / PX, e.x / PX, ph - e.y / PX);
        let stroke = crate::render::stroke_of(&c.style, pal.conn);
        o.push_str(&format!("<Shape ID=\"{vid}\" NameU=\"Dynamic connector.{vid}\" Name=\"Dynamic connector.{vid}\" Type=\"Shape\" LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\">"));
        cell(&mut o, "PinX", (bx + ex) / 2.0);
        cell(&mut o, "PinY", (by + ey) / 2.0);
        cell(&mut o, "Width", ex - bx);
        cell(&mut o, "Height", ey - by);
        cell(&mut o, "LocPinX", (ex - bx) / 2.0);
        cell(&mut o, "LocPinY", (ey - by) / 2.0);
        cell(&mut o, "Angle", 0);
        let glued = |end: &End| match end { End::Glued { shape, .. } => map.get(shape).copied(), _ => None };
        let (gb, ge) = (glued(&c.from), glued(&c.to));
        if gb.is_some() { cellf(&mut o, "BeginX", bx, "_WALKGLUE(BegTrigger,EndTrigger,WalkPreference)"); cellf(&mut o, "BeginY", by, "_WALKGLUE(BegTrigger,EndTrigger,WalkPreference)"); } else { cell(&mut o, "BeginX", bx); cell(&mut o, "BeginY", by); }
        if ge.is_some() { cellf(&mut o, "EndX", ex, "_WALKGLUE(EndTrigger,BegTrigger,WalkPreference)"); cellf(&mut o, "EndY", ey, "_WALKGLUE(EndTrigger,BegTrigger,WalkPreference)"); } else { cell(&mut o, "EndX", ex); cell(&mut o, "EndY", ey); }
        if let Some(g) = gb { cellf(&mut o, "BegTrigger", 2, &format!("_XFTRIGGER(Sheet.{g}!EventXFMod)")); }
        if let Some(g) = ge { cellf(&mut o, "EndTrigger", 2, &format!("_XFTRIGGER(Sheet.{g}!EventXFMod)")); }
        cell(&mut o, "ObjType", 2);
        cell(&mut o, "ShapeRouteStyle", match c.routing { Routing::Orthogonal => 1, _ => 16 });
        cell(&mut o, "ConLineRouteExt", match c.routing { Routing::Curved => 2, _ => 1 });
        cell(&mut o, "LineWeight", c.style.width.unwrap_or(1.5) / PX);
        cell(&mut o, "LineColor", hexc(stroke));
        cell(&mut o, "LinePattern", match c.style.dash { Dash::Solid => 1, Dash::Dashed => 2, Dash::Dotted => 3 });
        cell(&mut o, "BeginArrow", arrow_code(c.arrow_start));
        cell(&mut o, "EndArrow", arrow_code(c.arrow_end));
        cell(&mut o, "BeginArrowSize", 2);
        cell(&mut o, "EndArrowSize", 2);
        cell(&mut o, "FillPattern", 0);
        if let Some(i) = layer_ix(c.layer) { cell(&mut o, "LayerMember", i); }
        o.push_str("<Section N=\"Geometry\" IX=\"0\"><Cell N=\"NoFill\" V=\"1\"/><Cell N=\"NoLine\" V=\"0\"/><Cell N=\"NoShow\" V=\"0\"/><Cell N=\"NoSnap\" V=\"0\"/>");
        for (k, q) in route.iter().enumerate() {
            let t = if k == 0 { "MoveTo" } else { "LineTo" };
            o.push_str(&format!("<Row T=\"{t}\" IX=\"{}\"><Cell N=\"X\" V=\"{}\"/><Cell N=\"Y\" V=\"{}\"/></Row>", k + 1, q.x / PX - bx, (ph - q.y / PX) - by));
        }
        o.push_str("</Section>");
        if !c.text.is_empty() {
            let mid = geom::poly_mid(&route);
            cell(&mut o, "TxtPinX", mid.x / PX - bx);
            cell(&mut o, "TxtPinY", (ph - mid.y / PX) - by);
            cell(&mut o, "TxtWidth", 1.5);
            cell(&mut o, "TxtHeight", 0.3);
            cell(&mut o, "TxtLocPinX", 0.75);
            cell(&mut o, "TxtLocPinY", 0.15);
            o.push_str(&format!("<Text>{}</Text>", esc(&c.text)));
        }
        o.push_str("</Shape>");
        if let Some(g) = gb { connects.push_str(&format!("<Connect FromSheet=\"{vid}\" FromCell=\"BeginX\" FromPart=\"9\" ToSheet=\"{g}\" ToCell=\"PinX\" ToPart=\"3\"/>")); }
        if let Some(g) = ge { connects.push_str(&format!("<Connect FromSheet=\"{vid}\" FromCell=\"EndX\" FromPart=\"12\" ToSheet=\"{g}\" ToCell=\"PinX\" ToPart=\"3\"/>")); }
    }
    o.push_str("</Shapes>");
    if !connects.is_empty() { o.push_str(&format!("<Connects>{connects}</Connects>")); }
    o.push_str("</PageContents>");

    let mut sheet = String::from("<PageSheet LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\">");
    cell(&mut sheet, "PageWidth", page.width / PX);
    cell(&mut sheet, "PageHeight", page.height / PX);
    cell(&mut sheet, "ShdwOffsetX", 0.125);
    cell(&mut sheet, "ShdwOffsetY", -0.125);
    sheet.push_str("<Cell N=\"PageScale\" V=\"1\" U=\"IN_F\"/><Cell N=\"DrawingScale\" V=\"1\" U=\"IN_F\"/>");
    cell(&mut sheet, "DrawingSizeType", 0);
    cell(&mut sheet, "DrawingScaleType", 0);
    cell(&mut sheet, "PageLineJumpDirX", 0);
    cell(&mut sheet, "LineJumpCode", if page.line_jumps { 1 } else { 0 });
    if !page.layers.is_empty() {
        sheet.push_str("<Section N=\"Layer\">");
        for (i, l) in page.layers.iter().enumerate() {
            sheet.push_str(&format!("<Row IX=\"{i}\"><Cell N=\"Name\" V=\"{}\"/><Cell N=\"Color\" V=\"255\"/><Cell N=\"Status\" V=\"0\"/><Cell N=\"Visible\" V=\"{}\"/><Cell N=\"Print\" V=\"{}\"/><Cell N=\"Active\" V=\"{}\"/><Cell N=\"Lock\" V=\"{}\"/><Cell N=\"Snap\" V=\"1\"/><Cell N=\"Glue\" V=\"1\"/><Cell N=\"NameUniv\" V=\"{}\"/><Cell N=\"ColorTrans\" V=\"0\"/></Row>",
                esc(&l.name), l.visible as i32, l.print as i32, (page.active_layer == Some(l.id)) as i32, l.locked as i32, esc(&l.name)));
        }
        sheet.push_str("</Section>");
    }
    sheet.push_str("</PageSheet>");
    (o, sheet, media)
}

pub fn export(doc: &Document) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, body: &[u8]| -> Result<(), String> { z.start_file(name, opt).map_err(|e| e.to_string())?; z.write_all(body).map_err(|e| e.to_string()) };
        let n = doc.pages.len();
        let mut ct = String::from(r#"<?xml version="1.0" encoding="utf-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/visio/document.xml" ContentType="application/vnd.ms-visio.drawing.main+xml"/><Override PartName="/visio/pages/pages.xml" ContentType="application/vnd.ms-visio.pages+xml"/><Override PartName="/visio/windows.xml" ContentType="application/vnd.ms-visio.windows+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#);
        ct.push_str(r#"<Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/>"#);
        for i in 1..=n { ct.push_str(&format!("<Override PartName=\"/visio/pages/page{i}.xml\" ContentType=\"application/vnd.ms-visio.page+xml\"/>")); }
        ct.push_str("</Types>");
        put("[Content_Types].xml", (&ct).as_bytes())?;
        put("_rels/.rels", (r#"<?xml version="1.0" encoding="utf-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.microsoft.com/visio/2010/relationships/document" Target="visio/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#).as_bytes())?;
        put("docProps/app.xml", (r#"<?xml version="1.0" encoding="utf-8"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>Naqsha</Application><Template></Template><AppVersion>16.0000</AppVersion></Properties>"#).as_bytes())?;
        put("docProps/core.xml", (r#"<?xml version="1.0" encoding="utf-8"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:creator>Naqsha</dc:creator></cp:coreProperties>"#).as_bytes())?;
        put("visio/_rels/document.xml.rels", (r#"<?xml version="1.0" encoding="utf-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.microsoft.com/visio/2010/relationships/pages" Target="pages/pages.xml"/><Relationship Id="rId2" Type="http://schemas.microsoft.com/visio/2010/relationships/windows" Target="windows.xml"/></Relationships>"#).as_bytes())?;
        let mut d = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><VisioDocument {NS}>");
        d.push_str(r#"<DocumentSettings TopPage="0" DefaultTextStyle="0" DefaultLineStyle="0" DefaultFillStyle="0" DefaultGuideStyle="0"><GlueSettings>9</GlueSettings><SnapSettings>65847</SnapSettings><SnapExtensions>34</SnapExtensions><SnapAngles/><DynamicGridEnabled>1</DynamicGridEnabled><ProtectStyles>0</ProtectStyles><ProtectShapes>0</ProtectShapes><ProtectMasters>0</ProtectMasters><ProtectBkgnds>0</ProtectBkgnds></DocumentSettings>"#);
        d.push_str(r##"<Colors><ColorEntry IX="0" RGB="#000000"/><ColorEntry IX="1" RGB="#FFFFFF"/><ColorEntry IX="2" RGB="#FF0000"/><ColorEntry IX="3" RGB="#00FF00"/><ColorEntry IX="4" RGB="#0000FF"/><ColorEntry IX="5" RGB="#FFFF00"/><ColorEntry IX="6" RGB="#FF00FF"/><ColorEntry IX="7" RGB="#00FFFF"/></Colors>"##);
        d.push_str(r#"<FaceNames><FaceName NameU="Calibri" UnicodeRanges="-536859905 -1073732485 9 0" CharSets="536871423 0" Panose="2 15 5 2 2 2 4 3 2 4" Flags="325"/></FaceNames>"#);
        d.push_str(r##"<StyleSheets><StyleSheet ID="0" NameU="No Style" IsCustomNameU="1" Name="No Style" IsCustomName="1"><Cell N="EnableLineProps" V="1"/><Cell N="EnableFillProps" V="1"/><Cell N="EnableTextProps" V="1"/><Cell N="HideForApply" V="0"/><Cell N="LineWeight" V="0.01041666666666667"/><Cell N="LineColor" V="#000000"/><Cell N="LinePattern" V="1"/><Cell N="Rounding" V="0"/><Cell N="EndArrowSize" V="2"/><Cell N="BeginArrow" V="0"/><Cell N="EndArrow" V="0"/><Cell N="LineCap" V="0"/><Cell N="BeginArrowSize" V="2"/><Cell N="LineColorTrans" V="0"/><Cell N="FillForegnd" V="#FFFFFF"/><Cell N="FillBkgnd" V="#000000"/><Cell N="FillPattern" V="1"/><Cell N="ShdwForegnd" V="#000000"/><Cell N="ShdwPattern" V="0"/><Cell N="FillForegndTrans" V="0"/><Cell N="FillBkgndTrans" V="0"/><Cell N="ShdwForegndTrans" V="0"/><Cell N="ShapeShdwType" V="0"/><Cell N="ShapeShdwOffsetX" V="0"/><Cell N="ShapeShdwOffsetY" V="0"/><Cell N="ShapeShdwObliqueAngle" V="0"/><Cell N="ShapeShdwScaleFactor" V="1"/><Cell N="ShapeShdwBlur" V="0"/><Cell N="ShapeShdwShow" V="0"/><Cell N="LeftMargin" V="0.05555555555555555"/><Cell N="RightMargin" V="0.05555555555555555"/><Cell N="TopMargin" V="0.05555555555555555"/><Cell N="BottomMargin" V="0.05555555555555555"/><Cell N="VerticalAlign" V="1"/><Cell N="TextBkgnd" V="0"/><Cell N="DefaultTabStop" V="0.5"/><Cell N="TextDirection" V="0"/><Cell N="TextBkgndTrans" V="0"/><Section N="Character"><Row IX="0"><Cell N="Font" V="Calibri"/><Cell N="Color" V="#000000"/><Cell N="Style" V="0"/><Cell N="Case" V="0"/><Cell N="Pos" V="0"/><Cell N="FontScale" V="1"/><Cell N="Size" V="0.1666666666666667"/><Cell N="DblUnderline" V="0"/><Cell N="Overline" V="0"/><Cell N="Strikethru" V="0"/><Cell N="DoubleStrikethrough" V="0"/><Cell N="Letterspace" V="0"/><Cell N="ColorTrans" V="0"/><Cell N="AsianFont" V="0"/><Cell N="ComplexScriptFont" V="0"/><Cell N="ComplexScriptSize" V="-1"/><Cell N="LangID" V="en-US"/></Row></Section><Section N="Paragraph"><Row IX="0"><Cell N="IndFirst" V="0"/><Cell N="IndLeft" V="0"/><Cell N="IndRight" V="0"/><Cell N="SpLine" V="-1.2"/><Cell N="SpBefore" V="0"/><Cell N="SpAfter" V="0"/><Cell N="HorzAlign" V="1"/><Cell N="Bullet" V="0"/><Cell N="BulletStr" V=""/><Cell N="BulletFont" V="0"/><Cell N="TextPosAfterBullet" V="0"/><Cell N="BulletFontSize" V="-1"/><Cell N="Flags" V="0"/></Row></Section></StyleSheet></StyleSheets>"##);
        d.push_str("</VisioDocument>");
        put("visio/document.xml", (&d).as_bytes())?;
        let w0 = doc.pages.first().map(|p| (p.width / PX / 2.0, p.height / PX / 2.0)).unwrap_or((5.5, 4.25));
        put("visio/windows.xml", (&format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><Windows ClientWidth=\"1600\" ClientHeight=\"900\" {NS}><Window ID=\"0\" WindowType=\"Drawing\" WindowState=\"1073741824\" WindowLeft=\"-1\" WindowTop=\"-1\" WindowWidth=\"1600\" WindowHeight=\"900\" ContainerType=\"Page\" Page=\"0\" ViewScale=\"-1\" ViewCenterX=\"{}\" ViewCenterY=\"{}\"><ShowRulers>1</ShowRulers><ShowGrid>1</ShowGrid><ShowPageBreaks>0</ShowPageBreaks><ShowGuides>1</ShowGuides><ShowConnectionPoints>1</ShowConnectionPoints><GlueSettings>9</GlueSettings><SnapSettings>65847</SnapSettings><SnapExtensions>34</SnapExtensions><SnapAngles/><DynamicGridEnabled>1</DynamicGridEnabled><TabSplitterPos>0.5</TabSplitterPos></Window></Windows>", w0.0, w0.1)).as_bytes())?;
        let mut pages = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><Pages {NS}>");
        let mut prels = String::from(r#"<?xml version="1.0" encoding="utf-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
        for (i, p) in doc.pages.iter().enumerate() {
            let (contents, sheet, media) = page_xml(doc, p);
            put(&format!("visio/pages/page{}.xml", i + 1), contents.as_bytes())?;
            if !media.is_empty() {
                let mut rels = String::from(r#"<?xml version="1.0" encoding="utf-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
                for (k, (bytes, ext)) in media.iter().enumerate() {
                    let file = format!("image{}_{}.{ext}", i + 1, k + 1);
                    put(&format!("visio/media/{file}"), bytes)?;
                    rels.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/{file}\"/>", k + 1));
                }
                rels.push_str("</Relationships>");
                put(&format!("visio/pages/_rels/page{}.xml.rels", i + 1), rels.as_bytes())?;
            }
            pages.push_str(&format!("<Page ID=\"{i}\" NameU=\"{0}\" Name=\"{0}\" ViewScale=\"-1\" ViewCenterX=\"{1}\" ViewCenterY=\"{2}\">{sheet}<Rel r:id=\"rId{3}\"/></Page>", esc(&p.name), p.width / PX / 2.0, p.height / PX / 2.0, i + 1));
            prels.push_str(&format!("<Relationship Id=\"rId{0}\" Type=\"http://schemas.microsoft.com/visio/2010/relationships/page\" Target=\"page{0}.xml\"/>", i + 1));
        }
        pages.push_str("</Pages>");
        prels.push_str("</Relationships>");
        put("visio/pages/pages.xml", (&pages).as_bytes())?;
        put("visio/pages/_rels/pages.xml.rels", (&prels).as_bytes())?;
        z.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}
