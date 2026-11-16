//! draw.io / diagrams.net import and export (.drawio, .xml, compressed or plain).
use super::{decode_entities, hex_col, html_look, html_to_text, Imported};
use crate::library;
use crate::model::*;
use std::collections::HashMap;
use std::io::Read;

fn attr(n: &roxmltree::Node, a: &str) -> Option<String> { n.attribute(a).map(|s| s.to_string()) }

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// draw.io stores compressed diagrams as base64( raw-deflate( urlencode(xml) ) ).
fn inflate_diagram(text: &str) -> Result<String, String> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(text.trim().as_bytes()).map_err(|e| format!("bad base64: {e}"))?;
    let mut s = String::new();
    flate2::read::DeflateDecoder::new(&raw[..]).read_to_string(&mut s).map_err(|e| format!("bad deflate: {e}"))?;
    Ok(url_decode(&s))
}

fn parse_style(s: &str) -> (String, HashMap<String, String>) {
    let mut name = String::new();
    let mut m = HashMap::new();
    for (i, part) in s.split(';').enumerate() {
        let part = part.trim();
        if part.is_empty() { continue; }
        match part.split_once('=') {
            Some((k, v)) => { m.insert(k.trim().to_string(), v.trim().to_string()); }
            None => if i == 0 { name = part.to_string(); } else { m.insert(part.to_string(), "1".into()); },
        }
    }
    (name, m)
}

fn kind_for(name: &str, st: &HashMap<String, String>) -> &'static str {
    let shape = st.get("shape").map(|s| s.as_str()).unwrap_or(name);
    let s = shape.to_ascii_lowercase();
    let has = |k: &str| s.contains(k);
    if name == "text" || st.contains_key("text") && !st.contains_key("shape") { return "text"; }
    // stencils from other libraries (AWS, Cisco, isometric…): their names say nothing about their
    // outline ("mxgraph.aws3d.arrowNE" is not an arrow), so they stay plain boxes
    if let Some(ns) = s.strip_prefix("mxgraph.") {
        if !matches!(ns.split('.').next().unwrap_or(""), "flowchart" | "basic" | "arrows" | "arrows2" | "general" | "lean_mapping") { return "rect"; }
    }
    if s == "endstate" { return "uml_final"; }
    if s == "startstate" { return "ellipse"; }
    if matches!(s.as_str(), "umlframe" | "partialrectangle" | "table" | "tablerow" | "line" | "waypoint") { return "rect"; }
    if name == "swimlane" || s == "swimlane" { return "container"; }
    if has("ellipse") || has("circle") || has("oval") { return if has("doubleellipse") { "uml_final" } else { "ellipse" }; }
    if has("rhombus") || has("decision") { return "fc_decision"; }
    if has("terminator") { return "fc_terminator"; }
    if has("predefined") || s == "process" { return "fc_predefined"; }
    if has("multidocument") || has("multi_document") { return "fc_multidoc"; }
    if has("document") { return "fc_document"; }
    if has("manual_input") || has("manualinput") { return "fc_manual_input"; }
    if has("manual_operation") || has("manualoperation") { return "fc_manual_op"; }
    if has("preparation") || has("hexagon") { return if has("preparation") { "fc_preparation" } else { "hexagon" }; }
    if has("datastorage") || has("stored_data") { return "fc_stored"; }
    if has("delay") { return "fc_delay"; }
    if has("display") { return "fc_display"; }
    if has("offpage") || has("off_page") { return "fc_offpage"; }
    if has("cylinder") || has("database") || has("datastore") { return "cylinder"; }
    if has("cloud") { return "cloud"; }
    if has("parallelogram") || has("data") && has("flowchart") { return "parallelogram"; }
    if has("trapezoid") { return "trapezoid"; }
    if has("triangle") { return "triangle"; }
    if has("umlactor") || s == "actor" { return "uml_actor"; }
    if has("note") || has("card") { return "note"; }
    if has("callout") { return "callout"; }
    if has("step") || has("chevron") { return "chevron"; }
    if has("doublearrow") { return "arrow_2"; }
    if has("singlearrow") || has("arrow") { return "arrow_r"; }
    if has("cross") || has("plus") { return "cross"; }
    if has("star") { return "star"; }
    if has("heart") { return "heart"; }
    if has("half_circle") { return "semicircle"; }
    if has("moon") { return "crescent"; }
    if has("pentagon") { return "pentagon"; }
    if has("octagon") { return "octagon"; }
    if has("image") { return "image"; }
    if has("umllifeline") { return "uml_lifeline"; }
    if has("component") { return "uml_component"; }
    if has("folder") || has("package") { return "uml_package"; }
    if st.get("rounded").map_or(false, |v| v == "1") { return "rrect"; }
    "rect"
}

fn col(st: &HashMap<String, String>, k: &str) -> Option<Option<Col>> {
    let v = st.get(k)?;
    if v == "none" { return Some(Some(Col::NONE)); }
    if v == "default" || v.is_empty() { return None; }
    hex_col(v).map(Some)
}

fn arrow(v: Option<&String>, default: Arrow) -> Arrow {
    match v.map(|s| s.as_str()) {
        None => default,
        Some("none") | Some("") => Arrow::None,
        Some("open") | Some("openThin") | Some("openAsync") => Arrow::Open,
        Some(s) if s.starts_with("diamond") || s.starts_with("ERmandOne") => Arrow::Diamond,
        Some(s) if s.starts_with("oval") || s.starts_with("circle") || s.starts_with("ERzero") => Arrow::Circle,
        _ => Arrow::Filled,
    }
}

/// Black and white, for draw.io's default line and fill.
const BLACK_WHITE: (Col, Col) = (Col([0, 0, 0, 255]), Col([255, 255, 255, 255]));

/// dashed=1, and dotted when its dash pattern is dots (dashPattern=1 4).
fn dash(st: &HashMap<String, String>) -> Dash {
    if !st.get("dashed").map_or(false, |v| v == "1") { return Dash::Solid; }
    if st.get("dashPattern").map_or(false, |p| p.split_whitespace().next() == Some("1")) { Dash::Dotted } else { Dash::Dashed }
}

/// The connection point of a shape nearest to (fx, fy), as fractions of its width and height.
fn port_near(kind: &str, fx: f32, fy: f32) -> i32 {
    library::ports(kind).iter().enumerate()
        .min_by(|a, b| ((a.1 .0 - fx).powi(2) + (a.1 .1 - fy).powi(2)).total_cmp(&((b.1 .0 - fx).powi(2) + (b.1 .1 - fy).powi(2))))
        .map_or(-1, |(i, _)| i as i32)
}

struct Cell {
    id: String,
    parent: String,
    value: String,
    style: String,
    vertex: bool,
    edge: bool,
    source: Option<String>,
    target: Option<String>,
    geo: Option<(f32, f32, f32, f32, bool)>,
    pts: HashMap<String, (f32, f32)>,
    /// Edge waypoints (<Array as="points">).
    way: Vec<(f32, f32)>,
    visible: bool,
    data: Vec<(String, String)>,
    /// `link` attribute of a UserObject (e.g. "data:page/id,<diagram id>").
    link: Option<String>,
}

fn parse_model(model: roxmltree::Node) -> Vec<Cell> {
    let mut cells = vec![];
    let Some(root) = model.descendants().find(|n| n.has_tag_name("root")) else { return cells };
    for node in root.children().filter(|n| n.is_element()) {
        // <UserObject label=.. id=..><mxCell/></UserObject> and <object ...> wrappers
        let (wrapper, mx) = if node.has_tag_name("mxCell") { (None, node) } else { match node.children().find(|c| c.has_tag_name("mxCell")) { Some(m) => (Some(node), m), None => continue } };
        let id = wrapper.and_then(|w| attr(&w, "id")).or_else(|| attr(&mx, "id")).unwrap_or_default();
        let value = wrapper.and_then(|w| attr(&w, "label")).or_else(|| attr(&mx, "value")).unwrap_or_default();
        let mut data = vec![];
        if let Some(w) = wrapper {
            for a in w.attributes() { if !matches!(a.name(), "id" | "label" | "placeholders" | "tooltip" | "link") { data.push((a.name().to_string(), a.value().to_string())); } }
        }
        let g = mx.children().find(|c| c.has_tag_name("mxGeometry"));
        let num = |n: &roxmltree::Node, k: &str| n.attribute(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        let mut pts = HashMap::new();
        let mut way = vec![];
        if let Some(g) = g {
            for p in g.children().filter(|c| c.has_tag_name("mxPoint")) {
                if let Some(a) = p.attribute("as") { pts.insert(a.to_string(), (num(&p, "x"), num(&p, "y"))); }
            }
            if let Some(arr) = g.children().find(|c| c.has_tag_name("Array") && c.attribute("as") == Some("points")) {
                way = arr.children().filter(|c| c.has_tag_name("mxPoint")).map(|p| (num(&p, "x"), num(&p, "y"))).collect();
            }
        }
        cells.push(Cell {
            id,
            parent: attr(&mx, "parent").unwrap_or_default(),
            value,
            style: attr(&mx, "style").unwrap_or_default(),
            vertex: mx.attribute("vertex") == Some("1"),
            edge: mx.attribute("edge") == Some("1"),
            source: attr(&mx, "source"),
            target: attr(&mx, "target"),
            geo: g.map(|g| (num(&g, "x"), num(&g, "y"), num(&g, "width"), num(&g, "height"), g.attribute("relative") == Some("1"))),
            pts,
            way,
            visible: mx.attribute("visible") != Some("0"),
            data,
            link: wrapper.and_then(|w| attr(&w, "link")),
        });
    }
    cells
}

pub fn import(text: &str) -> Result<Imported, String> {
    let d = roxmltree::Document::parse(text).map_err(|e| format!("not valid XML: {e}"))?;
    let mut doc = Document { pages: vec![], ..Document::default() };
    let mut notes = vec![];
    let root = d.root_element();
    // a file Naqsha wrote: its containers are Naqsha's, one shape each
    let ours = root.attribute("host") == Some("Naqsha");
    // collect (name, model-xml-owned)
    let mut diagrams: Vec<(String, String)> = vec![];
    let mut diagram_ids: Vec<Option<String>> = vec![];
    if root.has_tag_name("mxGraphModel") {
        diagrams.push(("Page-1".into(), text.to_string()));
    } else {
        for (i, dg) in root.descendants().filter(|n| n.has_tag_name("diagram")).enumerate() {
            let name = attr(&dg, "name").unwrap_or_else(|| format!("Page-{}", i + 1));
            diagram_ids.push(attr(&dg, "id"));
            if let Some(m) = dg.children().find(|c| c.has_tag_name("mxGraphModel")) {
                diagrams.push((name, text[m.range()].to_string()));
            } else if let Some(t) = dg.text().filter(|t| !t.trim().is_empty()) {
                diagrams.push((name, inflate_diagram(t)?));
            }
        }
    }
    if diagrams.is_empty() {
        if root.has_tag_name("mxlibrary") { return Err("this is a draw.io shape library, not a diagram".into()); }
        return Err("no diagrams found in this file".into());
    }
    let mut unknown_shapes = 0usize;
    let mut links: Vec<(Id, String)> = vec![];
    let mut page_of_diagram: HashMap<String, Id> = HashMap::new();
    for (di, (name, xml)) in diagrams.into_iter().enumerate() {
        let md = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
        let model = md.root_element();
        let cells = parse_model(model);
        let byid: HashMap<&str, &Cell> = cells.iter().map(|c| (c.id.as_str(), c)).collect();
        let root_id = cells.iter().find(|c| c.parent.is_empty()).map(|c| c.id.clone()).unwrap_or_default();
        // a style colour, following "inherit" up to the parent's
        let colour = |c: &Cell, k: &str| -> Option<Option<Col>> {
            let mut cur = c;
            for _ in 0..8 {
                let (_, st) = parse_style(&cur.style);
                match st.get(k).map(|v| v.as_str()) {
                    Some("inherit") => match byid.get(cur.parent.as_str()) { Some(p) => cur = p, None => return None },
                    _ => return col(&st, k),
                }
            }
            None
        };
        let mut page = Page { id: doc.new_id(), name, line_jumps: false, avoid_shapes: false, ..Default::default() };
        if let (Some(w), Some(h)) = (model.attribute("pageWidth").and_then(|v| v.parse::<f32>().ok()), model.attribute("pageHeight").and_then(|v| v.parse::<f32>().ok())) { page.width = w; page.height = h; }
        // layers = children of the root cell
        let mut layer_map: HashMap<String, Id> = HashMap::new();
        let layer_cells: Vec<&Cell> = cells.iter().filter(|c| c.parent == root_id && !c.vertex && !c.edge).collect();
        if layer_cells.len() > 1 {
            for (i, lc) in layer_cells.iter().enumerate() {
                let id = doc.new_id();
                page.layers.push(Layer { id, name: if lc.value.is_empty() { format!("Layer {}", i + 1) } else { html_to_text(&lc.value) }, visible: lc.visible, locked: parse_style(&lc.style).1.get("locked").map_or(false, |v| v == "1"), print: true, background: false });
                layer_map.insert(lc.id.clone(), id);
            }
        }
        let is_layer = |id: &str| layer_cells.iter().any(|l| l.id == id) || id == root_id;
        // absolute origin of a cell (sum of vertex-parent offsets)
        let origin = |mut pid: String| -> ((f32, f32), Option<Id>) {
            let (mut ox, mut oy) = (0.0, 0.0);
            let mut layer = None;
            for _ in 0..64 {
                if is_layer(&pid) { layer = layer_map.get(&pid).copied(); break; }
                let Some(p) = byid.get(pid.as_str()) else { break };
                if let Some((x, y, ..)) = p.geo { if p.vertex { ox += x; oy += y; } }
                pid = p.parent.clone();
            }
            ((ox, oy), layer)
        };
        let mut idmap: HashMap<String, Id> = HashMap::new();
        let mut groups: HashMap<String, Id> = HashMap::new();
        let mut min = (f32::MAX, f32::MAX);
        let mut max = (f32::MIN, f32::MIN);
        // vertices
        for c in cells.iter().filter(|c| c.vertex) {
            let Some((x, y, w, h, relative)) = c.geo else { continue };
            let (pname, st) = parse_style(&c.style);
            let parent_is_edge = byid.get(c.parent.as_str()).map_or(false, |p| p.edge);
            if parent_is_edge || relative {
                // edge label
                continue;
            }
            let ((ox, oy), layer) = origin(c.parent.clone());
            let group_parent = byid.get(c.parent.as_str()).filter(|p| p.vertex && parse_style(&p.style).0 == "group").map(|p| p.id.clone());
            if pname == "group" { groups.entry(c.id.clone()).or_insert_with(|| doc.new_id()); continue; }
            let kind = kind_for(&pname, &st);
            let shape_name = st.get("shape").map(|v| v.to_ascii_lowercase()).unwrap_or_else(|| pname.to_ascii_lowercase());
            if kind == "rect" && st.contains_key("shape") && !matches!(shape_name.as_str(), "umlframe" | "partialrectangle" | "table" | "tablerow" | "line" | "waypoint") { unknown_shapes += 1; }
            let id = doc.new_id();
            let mut s = Shape { id, kind: kind.into(), x: ox + x, y: oy + y, w: w.max(4.0), h: h.max(4.0), style: library::default_style(kind), layer, ..Default::default() };
            let html = st.get("html").map_or(false, |v| v == "1");
            s.text = if html { html_to_text(&c.value) } else { decode_entities(&c.value) };
            // draw.io's own defaults (white shapes, black lines and text, 1 px, 12 px type), not
            // Naqsha's theme: the diagram should look the way it did in draw.io
            let plain = !matches!(kind, "text" | "image");
            if plain { s.style.fill = Some(BLACK_WHITE.1); s.style.stroke = Some(BLACK_WHITE.0); s.style.width = Some(1.0); }
            s.style.text = Some(BLACK_WHITE.0);
            s.style.font_size = Some(12.0);
            if let Some(v) = colour(&c, "fillColor") { s.style.fill = v; }
            if let Some(v) = colour(&c, "strokeColor") { s.style.stroke = v; }
            if let Some(v) = colour(&c, "fontColor") { s.style.text = v; }
            if let Some(v) = st.get("strokeWidth").and_then(|v| v.parse().ok()) { s.style.width = Some(v); }
            if let Some(v) = st.get("fontSize").and_then(|v| v.parse().ok()) { s.style.font_size = Some(v); }
            if let Some(v) = st.get("opacity").and_then(|v| v.parse::<f32>().ok()) { s.style.opacity = Some(v / 100.0); }
            if let Some(v) = st.get("rotation").and_then(|v| v.parse::<f32>().ok()) { s.rot = v.rem_euclid(360.0); }
            if st.get("flipH").map_or(false, |v| v == "1") { s.flip_h = true; }
            if st.get("flipV").map_or(false, |v| v == "1") { s.flip_v = true; }
            s.style.dash = dash(&st);
            if st.get("shadow").map_or(false, |v| v == "1") { s.style.shadow = true; }
            let fs: i32 = st.get("fontStyle").and_then(|v| v.parse().ok()).unwrap_or(0);
            s.style.bold = fs & 1 != 0 || (kind == "container" && fs == 0 && !st.contains_key("fontStyle"));
            s.style.italic = fs & 2 != 0;
            // formatting inside an HTML label: <b>, <font size/color>, style="font-size: 20px"
            if html {
                let look = html_look(&c.value);
                if look.bold == Some(true) { s.style.bold = true; }
                if look.italic == Some(true) { s.style.italic = true; }
                if let Some(z) = look.size { s.style.font_size = Some(z); }
                if let Some(col) = look.color { s.style.text = Some(col); }
            }
            // an AWS 3D stencil: a light dashed placeholder (its colours mean something only to the
            // stencil: its "edges" are filled black); other stencils stay boxes in their colours
            if kind == "rect" && shape_name.starts_with("mxgraph.aws3d.") {
                s.style.fill = Some(Col::NONE);
                s.style.stroke = Some(Col([160, 160, 170, 255]));
                s.style.dash = Dash::Dashed;
            }
            match shape_name.as_str() {
                // a UML frame: an outline with its name in the corner
                "umlframe" => {
                    if !st.contains_key("fillColor") { s.style.fill = Some(Col::NONE); }
                    let tab = |k: &str, d: f32| st.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(d);
                    s.tbox = Some([0.0, 0.0, tab("width", 60.0).max(w * 0.3), tab("height", 30.0)]);
                }
                // table cells draw only some of their sides; Naqsha draws all or none
                // table cells draw only some of their sides; Naqsha draws all or none: none for a
                // cell with one side, all for a data store (top and bottom)
                "partialrectangle" if ["top", "left", "right", "bottom"].iter().filter(|k| st.get(**k).map_or(true, |v| v != "0")).count() < 2 => s.style.stroke = Some(Col::NONE),
                "tablerow" => { if !st.contains_key("fillColor") { s.style.fill = Some(Col::NONE); } if !st.contains_key("strokeColor") { s.style.stroke = Some(Col::NONE); } }
                _ => {}
            }
            s.style.align = st.get("align").map(|a| match a.as_str() { "left" => HAlign::Left, "right" => HAlign::Right, _ => HAlign::Center }).or(s.style.align);
            s.style.valign = st.get("verticalAlign").map(|a| match a.as_str() { "top" => VAlign::Top, "bottom" => VAlign::Bottom, _ => VAlign::Middle }).or(s.style.valign);
            if kind == "image" {
                if let Some(img) = st.get("image") {
                    // draw.io writes data URIs as data:image/png,BASE64 (no ;base64)
                    if let Some(b64) = img.split_once(',').map(|x| x.1).filter(|_| img.starts_with("data:image")) { s.image = Some(b64.to_string()); } else { s.kind = "rect".into(); s.text = format!("{}\n(linked image)", s.text).trim().to_string(); }
                }
            }
            // padding around the label (a table row's text starts after its ID column)
            let sp = |k: &str| st.get(k).and_then(|v| v.parse::<f32>().ok());
            if ["spacingLeft", "spacingRight", "spacingTop", "spacingBottom"].iter().any(|k| sp(k).map_or(false, |v| v > 10.0)) {
                let base = sp("spacing").unwrap_or(2.0);
                let (l, r, t, b) = (base + sp("spacingLeft").unwrap_or(0.0), base + sp("spacingRight").unwrap_or(0.0), base + sp("spacingTop").unwrap_or(0.0), base + sp("spacingBottom").unwrap_or(0.0));
                if l + r < w - 4.0 && t + b < h - 4.0 { s.tbox = Some([l, t, w - l - r, h - t - b]); }
            }
            // labels placed outside the shape
            let vlp = st.get("verticalLabelPosition").map(|s| s.as_str());
            let lp = st.get("labelPosition").map(|s| s.as_str());
            if !library::is_icon(kind) && (vlp.is_some_and(|v| v != "middle") || lp.is_some_and(|v| v != "center")) {
                let (lw, lh) = ((w + 80.0).max(120.0), 40.0);
                let x = match lp { Some("left") => -lw - 4.0, Some("right") => w + 4.0, _ => (w - lw) / 2.0 };
                let y = match vlp { Some("top") => -lh - 2.0, Some("bottom") => h + 2.0, _ => (h - lh) / 2.0 };
                s.tbox = Some([x, y, lw, lh]);
                s.style.valign = Some(match vlp { Some("top") => VAlign::Bottom, Some("bottom") => VAlign::Top, _ => VAlign::Middle });
                if lp == Some("left") { s.style.align = Some(HAlign::Right); } else if lp == Some("right") { s.style.align = Some(HAlign::Left); }
            }
            // a junction dot (shape=waypoint): a small filled circle in the middle of its box
            if shape_name == "waypoint" {
                let d = st.get("size").and_then(|v| v.parse::<f32>().ok()).unwrap_or(6.0);
                let (cx, cy) = s.center();
                s.kind = "ellipse".into();
                (s.x, s.y, s.w, s.h) = (cx - d / 2.0, cy - d / 2.0, d, d);
                let dot = match s.style.fill { Some(f) if f != Col::NONE => f, _ => s.style.stroke.filter(|c| *c != Col::NONE).unwrap_or(BLACK_WHITE.0) };
                s.style.fill = Some(dot);
                s.style.stroke = Some(dot);
            }
            // text with a fill or an outline is a box with text in it
            if kind == "text" && [s.style.fill, s.style.stroke].iter().any(|c| c.map_or(false, |c| c != Col::NONE)) { s.kind = "rect".into(); }
            // a swimlane: the header bar in its fill colour with the title, the body in
            // swimlaneFillColor (transparent by default, as in draw.io)
            let mut header: Option<Shape> = None;
            if kind == "container" && !ours {
                let start = st.get("startSize").and_then(|v| v.parse::<f32>().ok()).unwrap_or(23.0);
                let vertical = st.get("horizontal").map_or(false, |v| v == "0");
                if start > 2.0 && start < (if vertical { s.w } else { s.h }) {
                    let mut hd = Shape { id: doc.new_id(), kind: "rect".into(), style: s.style.clone(), layer, text: std::mem::take(&mut s.text), ..Default::default() };
                    hd.style.align = st.get("align").map(|a| match a.as_str() { "left" => HAlign::Left, "right" => HAlign::Right, _ => HAlign::Center }).or(Some(HAlign::Center));
                    hd.style.valign = Some(VAlign::Middle);
                    hd.style.shadow = false;
                    if vertical {
                        // the header runs down the left side, its title turned to read upwards
                        (hd.w, hd.h) = (s.h, start);
                        hd.x = s.x + start / 2.0 - s.h / 2.0;
                        hd.y = s.y + s.h / 2.0 - start / 2.0;
                        hd.rot = 270.0;
                    } else {
                        (hd.x, hd.y, hd.w, hd.h) = (s.x, s.y, s.w, start);
                    }
                    s.style.fill = colour(&c, "swimlaneFillColor").unwrap_or(Some(Col::NONE));
                    header = Some(hd);
                }
            }
            s.data = c.data.clone();
            if let Some(l) = c.link.as_deref().and_then(|l| l.strip_prefix("data:page/id,")) { links.push((id, l.to_string())); }
            if let Some(gp) = group_parent { s.group = Some(*groups.entry(gp).or_insert_with(|| doc.new_id())); }
            min = (min.0.min(s.x), min.1.min(s.y));
            max = (max.0.max(s.x + s.w), max.1.max(s.y + s.h));
            idmap.insert(c.id.clone(), id);
            if library::is_container(&s.kind) { page.shapes.insert(0, s); } else { page.shapes.push(s); }
            if let Some(mut hd) = header { hd.group = page.shapes.iter().find(|x| x.id == id).and_then(|x| x.group); page.shapes.push(hd); }
        }
        // edges
        for c in cells.iter().filter(|c| c.edge) {
            let (_, st) = parse_style(&c.style);
            let ((ox, oy), layer) = origin(c.parent.clone());
            let end = |id: &Option<String>, pt: &str| -> End {
                if let Some(s) = id.as_ref().and_then(|i| idmap.get(i)) { return End::Glued { shape: *s, port: -1 }; }
                let (x, y) = c.pts.get(pt).copied().unwrap_or((0.0, 0.0));
                End::Free { x: ox + x, y: oy + y }
            };
            // draw.io's defaults: black, 1 px, 11 px labels
            let mut style = Style { stroke: Some(BLACK_WHITE.0), text: Some(BLACK_WHITE.0), width: Some(1.0), font_size: Some(11.0), ..Default::default() };
            if let Some(v) = colour(c, "strokeColor") { style.stroke = v; }
            if let Some(v) = colour(c, "fontColor") { style.text = v; }
            if let Some(v) = st.get("strokeWidth").and_then(|v| v.parse().ok()) { style.width = Some(v); }
            if let Some(v) = st.get("fontSize").and_then(|v| v.parse().ok()) { style.font_size = Some(v); }
            style.dash = dash(&st);
            let mut text = if st.get("html").map_or(false, |v| v == "1") { html_to_text(&c.value) } else { decode_entities(&c.value) };
            let mut look = if st.get("html").map_or(false, |v| v == "1") { html_look(&c.value) } else { Default::default() };
            if text.is_empty() {
                // the label is its own cell on the edge, with its own size and colour
                if let Some(l) = cells.iter().find(|l| l.parent == c.id && l.vertex && !l.value.is_empty()) {
                    let (_, lst) = parse_style(&l.style);
                    text = html_to_text(&l.value);
                    // its own style, not the edge's (draw.io doesn't pass the edge's font on)
                    style.font_size = Some(lst.get("fontSize").and_then(|v| v.parse().ok()).unwrap_or(11.0));
                    style.text = col(&lst, "fontColor").unwrap_or(Some(BLACK_WHITE.0));
                    look = html_look(&l.value);
                }
            }
            if let Some(z) = look.size { style.font_size = Some(z); }
            if let Some(col) = look.color { style.text = Some(col); }
            if look.bold == Some(true) { style.bold = true; }
            if look.italic == Some(true) { style.italic = true; }
            let routing = match (st.get("edgeStyle").map(|s| s.as_str()), st.get("curved").map(|s| s.as_str())) {
                // curved through its waypoints; with none, draw.io draws it straight
                (e, Some("1")) => if c.way.is_empty() && !e.map_or(false, |e| e.contains("orthogonal") || e.contains("elbow")) { Routing::Straight } else { Routing::Curved },
                (Some(e), _) if e.contains("orthogonal") || e.contains("elbow") || e.contains("entityRelation") || e.contains("isometric") => Routing::Orthogonal,
                _ => Routing::Straight,
            };
            // where on the shapes it starts and ends: exitX/exitY and entryX/entryY, or a vertical /
            // horizontal elbow (org charts leave from the bottom and arrive at the top)
            let shape_of = |id: &Option<String>| id.as_ref().and_then(|i| idmap.get(i)).and_then(|sid| page.shapes.iter().find(|s| s.id == *sid));
            let (sa, sb) = (shape_of(&c.source), shape_of(&c.target));
            let num = |k: &str| st.get(k).and_then(|v| v.parse::<f32>().ok());
            let elbow = st.get("elbow").map(|v| v.as_str()).filter(|_| routing == Routing::Orthogonal && c.way.is_empty());
            let way: Vec<(f32, f32)> = c.way.iter().map(|(x, y)| (ox + x, oy + y)).collect();
            let side = |me: Option<&Shape>, other: Option<&Shape>, fx: Option<f32>, fy: Option<f32>, toward: Option<(f32, f32)>| -> i32 {
                let Some(me) = me else { return -1 };
                if let (Some(fx), Some(fy)) = (fx, fy) { return port_near(&me.kind, fx, fy); }
                // leave toward the nearest waypoint: straight below it → the bottom, and so on
                if let Some((wx, wy)) = toward.filter(|_| routing == Routing::Orthogonal && me.rot == 0.0) {
                    let (inx, iny) = (wx >= me.x && wx <= me.x + me.w, wy >= me.y && wy <= me.y + me.h);
                    let (cx, cy) = me.center();
                    let vertical = if inx != iny { inx } else { (wy - cy).abs() / me.h.max(1.0) > (wx - cx).abs() / me.w.max(1.0) };
                    return if vertical { port_near(&me.kind, 0.5, if wy >= cy { 1.0 } else { 0.0 }) } else { port_near(&me.kind, if wx >= cx { 1.0 } else { 0.0 }, 0.5) };
                }
                let Some(o) = other else { return -1 };
                let (d, e) = (o.center().0 - me.center().0, o.center().1 - me.center().1);
                match elbow {
                    Some("vertical") => port_near(&me.kind, 0.5, if e >= 0.0 { 1.0 } else { 0.0 }),
                    Some("horizontal") => port_near(&me.kind, if d >= 0.0 { 1.0 } else { 0.0 }, 0.5),
                    _ => -1,
                }
            };
            let (pa, pb) = (side(sa, sb, num("exitX"), num("exitY"), way.first().copied()), side(sb, sa, num("entryX"), num("entryY"), way.last().copied()));
            let glue = |e: End, port: i32| match e { End::Glued { shape, .. } => End::Glued { shape, port }, e => e };
            page.conns.push(Conn {
                id: doc.new_id(),
                from: glue(end(&c.source, "sourcePoint"), pa),
                to: glue(end(&c.target, "targetPoint"), pb),
                routing,
                style,
                arrow_start: arrow(st.get("startArrow"), Arrow::None),
                arrow_end: arrow(st.get("endArrow"), Arrow::Filled),
                text,
                layer,
                data: c.data.clone(),
                bends: if c.way.is_empty() { None } else { Some(c.way.iter().map(|(x, y)| [ox + x, oy + y]).collect()) },
            });
        }
        // fit page to content when the model had no page size
        if model.attribute("pageWidth").is_none() && min.0 < f32::MAX {
            page.width = (max.0 + 40.0).max(800.0);
            page.height = (max.1 + 40.0).max(600.0);
        } else if min.0 < f32::MAX {
            page.width = page.width.max(max.0 + 20.0);
            page.height = page.height.max(max.1 + 20.0);
        }
        if let Some(Some(did)) = diagram_ids.get(di) { page_of_diagram.insert(did.clone(), page.id); }
        doc.pages.push(page);
    }
    for (sid, did) in links {
        let Some(&pid) = page_of_diagram.get(&did) else { continue };
        for p in doc.pages.iter_mut() { let here = p.id; if let Some(s) = p.shape_mut(sid) { if here != pid { s.link = Some(pid); } } }
    }
    if unknown_shapes > 0 { notes.push(format!("{unknown_shapes} specialised draw.io shape(s) shown as rectangles")); }
    doc.fix_ids();
    Ok(Imported { doc, notes })
}

// --------------------------------------------------------------------- export

fn esc(t: &str) -> String { t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\n', "&#xa;") }

fn style_name(kind: &str) -> &'static str {
    match kind {
        "rrect" => "rounded=1;whiteSpace=wrap;",
        "ellipse" | "circle" | "uml_usecase" | "fc_onpage" => "ellipse;whiteSpace=wrap;",
        "diamond" | "fc_decision" => "rhombus;whiteSpace=wrap;",
        "triangle" => "triangle;direction=north;whiteSpace=wrap;",
        "hexagon" | "fc_preparation" => "shape=hexagon;perimeter=hexagonPerimeter2;whiteSpace=wrap;",
        "parallelogram" | "fc_data" => "shape=parallelogram;perimeter=parallelogramPerimeter;whiteSpace=wrap;",
        "trapezoid" => "shape=trapezoid;perimeter=trapezoidPerimeter;whiteSpace=wrap;",
        "cylinder" | "fc_database" => "shape=cylinder3;whiteSpace=wrap;boundedLbl=1;size=15;",
        "cloud" | "n_cloud" => "ellipse;shape=cloud;whiteSpace=wrap;",
        "note" | "uml_note" => "shape=note;whiteSpace=wrap;size=14;",
        "fc_terminator" => "rounded=1;arcSize=50;whiteSpace=wrap;",
        "fc_document" => "shape=document;whiteSpace=wrap;boundedLbl=1;",
        "fc_multidoc" => "shape=mxgraph.flowchart.multi-document;whiteSpace=wrap;",
        "fc_predefined" => "shape=process;whiteSpace=wrap;backgroundOutline=1;",
        "fc_manual_input" => "shape=manualInput;whiteSpace=wrap;",
        "fc_stored" => "shape=dataStorage;whiteSpace=wrap;",
        "fc_delay" => "shape=delay;whiteSpace=wrap;",
        "fc_display" => "shape=display;whiteSpace=wrap;",
        "fc_offpage" => "shape=offPageConnector;whiteSpace=wrap;",
        "chevron" => "shape=step;perimeter=stepPerimeter;whiteSpace=wrap;",
        "arrow_r" => "shape=singleArrow;whiteSpace=wrap;",
        "arrow_2" => "shape=doubleArrow;whiteSpace=wrap;",
        "cross" => "shape=cross;whiteSpace=wrap;",
        "star" => "shape=mxgraph.basic.star;whiteSpace=wrap;",
        "semicircle" => "shape=mxgraph.basic.half_circle;whiteSpace=wrap;",
        "heart" => "shape=mxgraph.basic.heart;whiteSpace=wrap;",
        "crescent" => "shape=mxgraph.basic.moon;whiteSpace=wrap;",
        "uml_actor" | "n_user" => "shape=umlActor;verticalLabelPosition=bottom;verticalAlign=top;",
        "uml_class" | "uml_interface" | "er_entity" => "swimlane;fontStyle=1;childLayout=stackLayout;horizontal=1;startSize=26;",
        "container" | "lane_v" => "swimlane;whiteSpace=wrap;",
        "lane_h" => "swimlane;horizontal=0;whiteSpace=wrap;",
        "text" | "heading" => "text;whiteSpace=wrap;",
        _ => "whiteSpace=wrap;",
    }
}

pub fn export(doc: &Document) -> String {
    let pal = crate::theme::LIGHT;
    let hex = |c: egui::Color32| if c.a() == 0 { "none".to_string() } else { let [r, g, b, _] = c.to_srgba_unmultiplied(); format!("#{r:02X}{g:02X}{b:02X}") };
    let mut o = String::from("<mxfile host=\"Naqsha\" type=\"device\">");
    for p in &doc.pages {
        o.push_str(&format!("<diagram name=\"{}\" id=\"p{}\"><mxGraphModel dx=\"1000\" dy=\"800\" grid=\"1\" gridSize=\"10\" guides=\"1\" tooltips=\"1\" connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" pageWidth=\"{}\" pageHeight=\"{}\"><root><mxCell id=\"0\"/>", esc(&p.name), p.id, p.width, p.height));
        if p.layers.is_empty() { o.push_str("<mxCell id=\"1\" parent=\"0\"/>"); }
        for (i, l) in p.layers.iter().enumerate() {
            let id = if i == 0 { "1".to_string() } else { format!("L{}", l.id) };
            o.push_str(&format!("<mxCell id=\"{id}\" value=\"{}\" parent=\"0\"{}/>", esc(&l.name), if l.visible { "" } else { " visible=\"0\"" }));
        }
        let layer_cell = |l: Option<Id>| match l.and_then(|l| p.layers.iter().position(|x| x.id == l)) { Some(0) | None => "1".to_string(), Some(_) => format!("L{}", l.unwrap()) };
        for s in &p.shapes {
            let st = &s.style;
            let fill = crate::render::fill_of(s, &pal);
            let stroke = crate::render::stroke_of(st, pal.shape_stroke);
            let mut style = style_name(&s.kind).to_string();
            if library::is_icon(&s.kind) && !style.contains("umlActor") { style.push_str("verticalLabelPosition=bottom;verticalAlign=top;"); }
            if s.kind == "image" { if let Some(b) = &s.image { style = format!("shape=image;image=data:image/png,{b};verticalLabelPosition=bottom;verticalAlign=top;"); } }
            style.push_str(&format!("html=1;fillColor={};strokeColor={};fontColor={};", hex(fill), hex(stroke), hex(st.text.map(crate::theme::to_c32).unwrap_or(pal.shape_text))));
            if let Some(w) = st.width { style.push_str(&format!("strokeWidth={w};")); }
            if let Some(f) = st.font_size { style.push_str(&format!("fontSize={f};")); }
            if st.dash != Dash::Solid { style.push_str("dashed=1;"); }
            if st.shadow { style.push_str("shadow=1;"); }
            if st.mono { style.push_str("fontFamily=Courier New;"); }
            if st.bold || st.italic { style.push_str(&format!("fontStyle={};", st.bold as i32 | ((st.italic as i32) << 1))); }
            if let Some(a) = st.align { style.push_str(match a { HAlign::Left => "align=left;", HAlign::Right => "align=right;", HAlign::Center => "align=center;" }); }
            if s.rot != 0.0 { style.push_str(&format!("rotation={};", s.rot)); }
            if s.flip_h { style.push_str("flipH=1;"); }
            if s.flip_v { style.push_str("flipV=1;"); }
            let value = esc(&s.text).replace("&#xa;", "&lt;br&gt;");
            let cell = format!("<mxCell id=\"s{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"{}\"><mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\"/></mxCell>", s.id, if s.data.is_empty() && doc.linked_page(s).is_none() { value.clone() } else { String::new() }, esc(&style), layer_cell(s.layer), s.x, s.y, s.w, s.h);
            let link = doc.linked_page(s).map(|t| format!(" link=\"data:page/id,p{}\"", t.id)).unwrap_or_default();
            if s.data.is_empty() && link.is_empty() {
                o.push_str(&cell);
            } else {
                let mut attrs: String = s.data.iter().filter(|(k, _)| !k.is_empty() && k != "link" && k.chars().all(|c| c.is_alphanumeric() || c == '_')).map(|(k, v)| format!(" {k}=\"{}\"", esc(v))).collect();
                attrs.push_str(&link);
                o.push_str(&format!("<UserObject label=\"{value}\" id=\"s{}\"{attrs}>{}</UserObject>", s.id, cell.replacen(&format!("id=\"s{}\" ", s.id), "", 1)));
            }
        }
        for c in &p.conns {
            let mut style = match c.routing { Routing::Orthogonal => "edgeStyle=orthogonalEdgeStyle;rounded=0;".to_string(), Routing::Curved => "curved=1;".into(), Routing::Straight => String::new() };
            let a = |x: Arrow| match x { Arrow::None => "none", Arrow::Filled => "classic", Arrow::Open => "open", Arrow::Diamond => "diamond", Arrow::Circle => "oval" };
            style.push_str(&format!("html=1;endArrow={};startArrow={};strokeColor={};", a(c.arrow_end), a(c.arrow_start), hex(crate::render::stroke_of(&c.style, pal.conn))));
            if let Some(w) = c.style.width { style.push_str(&format!("strokeWidth={w};")); }
            if c.style.dash != Dash::Solid { style.push_str("dashed=1;"); }
            let mut ends = String::new();
            let mut pts = String::new();
            for (e, src) in [(c.from, true), (c.to, false)] {
                match e {
                    End::Glued { shape, .. } => ends.push_str(&format!(" {}=\"s{shape}\"", if src { "source" } else { "target" })),
                    End::Free { x, y } => pts.push_str(&format!("<mxPoint x=\"{x}\" y=\"{y}\" as=\"{}\"/>", if src { "sourcePoint" } else { "targetPoint" })),
                }
            }
            if let Some(b) = c.bends.as_ref().filter(|b| !b.is_empty() && c.routing != Routing::Curved) {
                pts.push_str("<Array as=\"points\">");
                for p in b { pts.push_str(&format!("<mxPoint x=\"{}\" y=\"{}\"/>", p[0], p[1])); }
                pts.push_str("</Array>");
            }
            o.push_str(&format!("<mxCell id=\"c{}\" value=\"{}\" style=\"{}\" edge=\"1\" parent=\"{}\"{ends}><mxGeometry relative=\"1\" as=\"geometry\">{pts}</mxGeometry></mxCell>", c.id, esc(&c.text), esc(&style), layer_cell(c.layer)));
        }
        o.push_str("</root></mxGraphModel></diagram>");
    }
    o.push_str("</mxfile>");
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(cells: &str) -> Page {
        let xml = format!("<mxGraphModel><root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>{cells}</root></mxGraphModel>");
        import(&xml).unwrap().doc.pages.remove(0)
    }

    #[test]
    fn keeps_the_drawio_look() {
        let p = page(r#"<mxCell id="a" value="&lt;b&gt;&lt;font style=&quot;font-size: 20px&quot;&gt;Activities&lt;/font&gt;&lt;/b&gt;" style="text;html=1;" vertex="1" parent="1"><mxGeometry x="0" y="0" width="100" height="40" as="geometry"/></mxCell>
            <mxCell id="b" value="Box" style="rounded=0;html=1;" vertex="1" parent="1"><mxGeometry x="200" y="0" width="100" height="40" as="geometry"/></mxCell>"#);
        let (a, b) = (&p.shapes[0], &p.shapes[1]);
        assert_eq!(a.text, "Activities");
        assert!(a.style.bold);
        assert_eq!(a.style.font_size, Some(20.0));
        // draw.io's defaults, not the theme's
        assert_eq!(b.style.fill, Some(Col([255, 255, 255, 255])));
        assert_eq!(b.style.stroke, Some(Col([0, 0, 0, 255])));
        assert_eq!(b.style.font_size, Some(12.0));
    }

    #[test]
    fn connectors_leave_where_drawio_says() {
        let p = page(r#"<mxCell id="a" value="A" style="rounded=0;" vertex="1" parent="1"><mxGeometry x="100" y="0" width="100" height="40" as="geometry"/></mxCell>
            <mxCell id="b" value="B" style="rounded=0;" vertex="1" parent="1"><mxGeometry x="0" y="200" width="100" height="40" as="geometry"/></mxCell>
            <mxCell id="e" style="edgeStyle=orthogonalEdgeStyle;elbow=vertical;dashed=1;dashPattern=1 4;" edge="1" parent="1" source="a" target="b"><mxGeometry relative="1" as="geometry"/></mxCell>
            <mxCell id="f" style="exitX=1;exitY=0.5;entryX=0;entryY=0.5;curved=1;" edge="1" parent="1" source="a" target="b"><mxGeometry relative="1" as="geometry"/></mxCell>"#);
        let ports = library::ports("rect");
        let at = |e: End| match e { End::Glued { port, .. } if port >= 0 => ports[port as usize], _ => (-1.0, -1.0) };
        // an elbow=vertical org-chart line: out of the bottom, into the top
        assert_eq!(at(p.conns[0].from), (0.5, 1.0));
        assert_eq!(at(p.conns[0].to), (0.5, 0.0));
        assert_eq!(p.conns[0].style.dash, Dash::Dotted);
        // exitX/entryX, and curved with no waypoints is drawn straight
        assert_eq!(at(p.conns[1].from), (1.0, 0.5));
        assert_eq!(at(p.conns[1].to), (0.0, 0.5));
        assert_eq!(p.conns[1].routing, Routing::Straight);
    }

    #[test]
    fn swimlanes_get_a_header() {
        let p = page(r#"<mxCell id="s" value="Lane" style="swimlane;startSize=30;fillColor=#4D4D4D;swimlaneFillColor=#FFFFFF;fontColor=#FFFFFF;" vertex="1" parent="1"><mxGeometry x="0" y="0" width="200" height="120" as="geometry"/></mxCell>
            <mxCell id="w" style="shape=waypoint;size=6;" vertex="1" parent="1"><mxGeometry x="300" y="0" width="40" height="40" as="geometry"/></mxCell>"#);
        let lane = p.shapes.iter().find(|s| s.kind == "container").unwrap();
        let head = p.shapes.iter().find(|s| s.text == "Lane").unwrap();
        assert_eq!((head.h, lane.text.as_str()), (30.0, ""));
        assert_eq!(head.style.fill, Some(Col([0x4D, 0x4D, 0x4D, 255])));
        assert_eq!(lane.style.fill, Some(Col([255, 255, 255, 255])));
        let dot = p.shapes.iter().find(|s| s.kind == "ellipse").unwrap();
        assert_eq!((dot.w, dot.x + dot.w / 2.0), (6.0, 320.0));
    }
}
