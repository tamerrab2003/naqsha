//! Node-RED flows → diagrams.
//!
//! Reads a flows export (the JSON Node-RED copies with Export, or a `flows.json` file): every
//! flow tab becomes a page and every subflow its own page, opened from the nodes that use it.
//! Nodes keep their Node-RED position, colour and label; wires become curved connectors, link
//! nodes dashed ones; groups become labelled boundaries. Configuration nodes (brokers, servers…)
//! are listed in a note on the first page. Each shape carries the node's type, id and its main
//! settings as shape data.
use super::{hex_col, Imported};
use crate::model::*;
use serde_json::Value;
use std::collections::HashMap;

/// Does this text look like a Node-RED flows export?
pub fn looks_like(text: &str) -> bool {
    let t = text.trim_start();
    if !(t.starts_with('[') || t.starts_with('{')) { return false; }
    nodes_of(text).map_or(false, |v| v.iter().any(|n| n["type"].is_string() && n["id"].is_string() && (n["wires"].is_array() || n["type"] == "tab")))
}

/// The node list: a plain array, or `{flows: [...]}` as the Admin API returns it.
fn nodes_of(text: &str) -> Option<Vec<Value>> {
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Array(a) => Some(a),
        Value::Object(o) => o.get("flows").and_then(|f| f.as_array().cloned()),
        _ => None,
    }
}

fn st<'a>(n: &'a Value, k: &str) -> &'a str { n[k].as_str().unwrap_or("") }

/// Node-RED's palette colours for the core nodes and the common contrib ones.
fn color(ty: &str) -> &'static str {
    match ty {
        "inject" => "#A6BBCF",
        "debug" => "#87A980",
        "function" => "#FDD0A2",
        "change" | "switch" | "range" | "template" | "split" | "join" | "sort" | "batch" => "#E2D96E",
        "delay" | "trigger" => "#E6E0F8",
        "exec" => "#E9967A",
        "comment" => "#FFFFFF",
        "catch" => "#E49191",
        "status" => "#94C1D0",
        "complete" => "#C0EDC0",
        "link in" | "link out" | "link call" => "#DDDDDD",
        "naqsha-view" => "#C9BFFF",
        "mqtt in" | "mqtt out" => "#D8BFD8",
        "http in" | "http response" | "http request" => "#E7E7AE",
        "websocket in" | "websocket out" => "#D7D7A0",
        "tcp in" | "tcp out" | "tcp request" | "udp in" | "udp out" => "#C0C0C0",
        "csv" | "html" | "json" | "xml" | "yaml" => "#DEBD5C",
        "file" | "file in" | "watch" => "#BDC7D6",
        "rbe" | "filter" => "#E2D96E",
        t if t.starts_with("ui_") || t.starts_with("ui-") => "#3FADB5",
        t if t.starts_with("modbus") => "#E9A94C",
        t if t.starts_with("OpcUa") || t.starts_with("opcua") => "#3FADB5",
        t if t.contains("s7") => "#3FADB5",
        _ => "#DDDDDD",
    }
}

/// What Node-RED shows on a node without a name.
fn label(n: &Value, subflows: &HashMap<String, String>) -> String {
    let name = st(n, "name").trim();
    if !name.is_empty() { return name.to_string(); }
    let ty = st(n, "type");
    if let Some(id) = ty.strip_prefix("subflow:") { return subflows.get(id).cloned().unwrap_or_else(|| "subflow".into()); }
    match ty {
        "mqtt in" | "mqtt out" if !st(n, "topic").is_empty() => st(n, "topic").to_string(),
        "http in" => format!("[{}] {}", st(n, "method"), st(n, "url")),
        "http request" if !st(n, "url").is_empty() => st(n, "url").to_string(),
        "inject" => match st(n, "payloadType") { "date" | "" => "timestamp".into(), _ => "inject".into() },
        "debug" => "debug".into(),
        "link in" | "link out" => String::new(),
        _ if !st(n, "label").is_empty() => st(n, "label").to_string(),
        _ => ty.to_string(),
    }
}

/// Settings worth keeping as shape data, by key.
const KEEP: &[&str] = &["topic", "url", "method", "broker", "qos", "property", "rules", "repeat", "crontab", "path", "group", "server", "host", "port", "unitid", "dataType", "adr", "quantity", "endpoint", "nodeId"];

pub fn import(text: &str) -> Result<Imported, String> {
    let nodes = nodes_of(text).ok_or("not a Node-RED flows export")?;
    let mut notes = vec![];
    let subflows: HashMap<String, String> = nodes.iter().filter(|n| n["type"] == "subflow").map(|n| (st(n, "id").to_string(), st(n, "name").to_string())).collect();
    // flow tabs in order, then subflows; nodes without a tab (pasted from one flow) get a page of their own
    let mut tabs: Vec<(String, String)> = nodes.iter().filter(|n| n["type"] == "tab").map(|n| (st(n, "id").to_string(), st(n, "label").to_string())).collect();
    tabs.extend(nodes.iter().filter(|n| n["type"] == "subflow").map(|n| (st(n, "id").to_string(), format!("Subflow: {}", st(n, "name")))));
    let placed = |n: &&Value| n["x"].is_number() && n["y"].is_number() && !matches!(st(n, "type"), "tab" | "subflow" | "group");
    if nodes.iter().filter(placed).any(|n| !tabs.iter().any(|(id, _)| id == st(n, "z"))) {
        tabs.push((String::new(), "Flow".into()));
    }
    if tabs.is_empty() || !nodes.iter().any(|n| placed(&n)) { return Err("no flow nodes in this export".into()); }

    let mut doc = Document { pages: vec![], ..Document::default() };
    let mut page_of: HashMap<String, usize> = HashMap::new();
    for (id, name) in &tabs {
        let pid = doc.new_id();
        page_of.insert(id.clone(), doc.pages.len());
        doc.pages.push(Page { id: pid, name: if name.is_empty() { "Flow".into() } else { name.clone() }, line_jumps: false, avoid_shapes: true, ..Default::default() });
    }
    let page_for = |z: &str| page_of.get(z).copied().or_else(|| page_of.get("").copied());

    // groups first, so they sit behind their nodes
    for g in nodes.iter().filter(|n| n["type"] == "group") {
        let Some(pi) = page_for(st(g, "z")) else { continue };
        let (x, y, w, h) = (g["x"].as_f64().unwrap_or(0.0) as f32, g["y"].as_f64().unwrap_or(0.0) as f32, g["w"].as_f64().unwrap_or(200.0) as f32, g["h"].as_f64().unwrap_or(100.0) as f32);
        let gs = &g["style"];
        let mut style = crate::library::default_style("boundary");
        style.stroke = hex_col(gs["stroke"].as_str().unwrap_or("#999999")).or(Some(Col::hex("#999999")));
        style.dash = Dash::Solid;
        style.width = Some(1.0);
        style.fill = match gs["fill"].as_str().and_then(hex_col) {
            Some(mut c) => { c.0[3] = (gs["fill-opacity"].as_str().and_then(|o| o.parse::<f32>().ok()).or(gs["fill-opacity"].as_f64().map(|o| o as f32)).unwrap_or(0.5).clamp(0.0, 1.0) * 255.0) as u8; Some(c) }
            None => Some(Col::NONE),
        };
        style.text = gs["color"].as_str().and_then(hex_col);
        style.font_size = Some(12.0);
        let id = doc.new_id();
        doc.pages[pi].shapes.push(Shape { id, kind: "boundary".into(), x, y, w, h, text: st(g, "name").into(), style, data: vec![("Node-RED type".into(), "group".into())], ..Default::default() });
    }

    // nodes
    let mut shape_of: HashMap<String, (usize, Id)> = HashMap::new();
    for n in nodes.iter().filter(placed) {
        let Some(pi) = page_for(st(n, "z")) else { continue };
        let ty = st(n, "type");
        let (cx, cy) = (n["x"].as_f64().unwrap_or(0.0) as f32, n["y"].as_f64().unwrap_or(0.0) as f32);
        let id = doc.new_id();
        let mut data = vec![("Node-RED type".to_string(), ty.to_string()), ("Node-RED id".to_string(), st(n, "id").to_string())];
        for k in KEEP {
            match &n[*k] {
                Value::String(v) if !v.is_empty() => data.push((k.to_string(), v.clone())),
                Value::Number(v) => data.push((k.to_string(), v.to_string())),
                _ => {}
            }
        }
        if ty == "function" { data.push(("Lines of code".into(), st(n, "func").lines().count().to_string())); }
        if !st(n, "info").is_empty() { data.push(("Description".into(), st(n, "info").chars().take(400).collect())); }
        let shape = match ty {
            "junction" => Shape { id, kind: "ellipse".into(), x: cx - 5.0, y: cy - 5.0, w: 10.0, h: 10.0, style: Style { fill: Some(Col::hex("#999999")), stroke: Some(Col::hex("#999999")), width: Some(1.0), ..Default::default() }, data, ..Default::default() },
            "comment" => {
                let mut style = crate::library::default_style("text");
                style.italic = true;
                style.align = Some(HAlign::Left);
                let t = label(n, &subflows);
                let w = (t.chars().count() as f32 * 7.2 + 24.0).max(80.0);
                Shape { id, kind: "text".into(), x: cx - w / 2.0, y: cy - 12.0, w, h: 24.0, text: t, style, data, ..Default::default() }
            }
            _ => {
                let t = label(n, &subflows);
                let outputs = n["wires"].as_array().map_or(1, |w| w.len()).max(1) as f32;
                let w = if t.is_empty() { 30.0 } else { (t.chars().count() as f32 * 7.0 + 46.0).max(100.0) };
                let h = (outputs * 13.0 + 4.0).max(30.0);
                let fill = if let Some(sf) = ty.strip_prefix("subflow:") {
                    nodes.iter().find(|s| s["type"] == "subflow" && st(s, "id") == sf).and_then(|s| s["color"].as_str()).and_then(hex_col).unwrap_or(Col::hex("#DDDDDD"))
                } else { Col::hex(color(ty)) };
                let mut style = Style { fill: Some(fill), stroke: Some(Col::hex("#999999")), width: Some(1.0), text: Some(Col::hex("#333333")), font_size: Some(12.0), ..Default::default() };
                if n["d"] == true { style.opacity = Some(0.45); style.dash = Dash::Dashed; data.push(("Disabled".into(), "yes".into())); }
                let link = ty.strip_prefix("subflow:").and_then(|sf| page_of.get(sf)).map(|i| doc.pages[*i].id);
                Shape { id, kind: "rrect".into(), x: cx - w / 2.0, y: cy - h / 2.0, w, h, text: t, style, data, link, ..Default::default() }
            }
        };
        doc.pages[pi].shapes.push(shape);
        shape_of.insert(st(n, "id").to_string(), (pi, id));
    }

    // subflow inputs and outputs: small grey port boxes
    for sf in nodes.iter().filter(|n| n["type"] == "subflow") {
        let Some(&pi) = page_of.get(st(sf, "id")) else { continue };
        for (key, label) in [("in", "input"), ("out", "output")] {
            for (k, port) in sf[key].as_array().into_iter().flatten().enumerate() {
                let (cx, cy) = (port["x"].as_f64().unwrap_or(0.0) as f32, port["y"].as_f64().unwrap_or(0.0) as f32);
                let id = doc.new_id();
                let style = Style { fill: Some(Col::hex("#EEEEEE")), stroke: Some(Col::hex("#999999")), width: Some(1.0), font_size: Some(10.0), ..Default::default() };
                doc.pages[pi].shapes.push(Shape { id, kind: "rrect".into(), x: cx - 20.0, y: cy - 15.0, w: 40.0, h: 30.0, text: if key == "out" { format!("{}", k + 1) } else { "in".into() }, style, data: vec![("Node-RED type".into(), format!("subflow {label}"))], ..Default::default() });
                let wires: Vec<String> = port["wires"].as_array().into_iter().flatten().filter_map(|w| w["id"].as_str().map(String::from)).collect();
                for w in wires {
                    let Some(&(wpi, other)) = shape_of.get(&w) else { continue };
                    if wpi != pi { continue; }
                    let (from, to) = if key == "in" { (id, other) } else { (other, id) };
                    let cid = doc.new_id();
                    doc.pages[pi].conns.push(wire(cid, from, to, false));
                }
            }
        }
    }

    // wires and links
    for n in nodes.iter().filter(placed) {
        let Some(&(pi, from)) = shape_of.get(st(n, "id")) else { continue };
        let mut targets: Vec<(String, bool)> = n["wires"].as_array().into_iter().flatten().flat_map(|port| port.as_array().into_iter().flatten()).filter_map(|t| t.as_str().map(|s| (s.to_string(), false))).collect();
        if n["type"] == "link out" || n["type"] == "link call" {
            targets.extend(n["links"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(|s| (s.to_string(), true))));
        }
        for (t, dashed) in targets {
            let Some(&(tpi, to)) = shape_of.get(&t) else { continue };
            if tpi != pi {
                if dashed { notes.push("Links to other flows are listed in each link node's data, not drawn".into()); }
                continue;
            }
            let id = doc.new_id();
            let mut w = wire(id, from, to, dashed);
            // a wire that runs back to the left goes around the nodes, as Node-RED draws it
            let x_of = |sid: Id| doc.pages[pi].shape(sid).map_or(0.0, |s| s.x);
            if x_of(to) < x_of(from) { w.routing = Routing::Orthogonal; }
            doc.pages[pi].conns.push(w);
        }
    }

    // configuration nodes: a note on the first page
    let config: Vec<String> = nodes.iter().filter(|n| !n["x"].is_number() && !matches!(st(n, "type"), "tab" | "subflow" | "group")).map(|n| {
        let detail = [st(n, "broker"), st(n, "host"), st(n, "url"), st(n, "port")].iter().filter(|v| !v.is_empty()).cloned().collect::<Vec<_>>().join(":");
        let name = if st(n, "name").is_empty() { st(n, "id") } else { st(n, "name") };
        if detail.is_empty() { format!("{} — {name}", st(n, "type")) } else { format!("{} — {name} ({detail})", st(n, "type")) }
    }).collect();

    let note_id = doc.new_id();
    for (pi, p) in doc.pages.iter_mut().enumerate() {
        // keep everything clear of the top-left corner and size the page to its flow
        let mut maxx: f32 = 0.0;
        let mut maxy: f32 = 0.0;
        for s in &p.shapes { maxx = maxx.max(s.x + s.w); maxy = maxy.max(s.y + s.h); }
        p.width = (maxx + 80.0).max(1400.0);
        p.height = (maxy + 80.0).max(990.0);
        if pi == 0 && !config.is_empty() {
            let id = note_id;
            let text = format!("Configuration nodes\n{}", config.join("\n"));
            let h = 22.0 + config.len() as f32 * 17.0;
            let mut style = crate::library::default_style("note");
            style.align = Some(HAlign::Left);
            style.font_size = Some(11.0);
            p.height += h + 40.0;
            let w = (config.iter().map(|c| c.chars().count()).max().unwrap_or(20) as f32 * 6.6 + 30.0).clamp(220.0, 900.0);
            p.shapes.push(Shape { id, kind: "note".into(), x: 40.0, y: maxy + 40.0, w, h: h + 12.0, text, style, ..Default::default() });
        }
    }
    doc.fix_ids();
    notes.dedup();
    let flows = tabs.iter().filter(|(_, n)| !n.starts_with("Subflow")).count();
    notes.insert(0, format!("Node-RED: {flows} flow{} and {} subflow{} imported", if flows == 1 { "" } else { "s" }, subflows.len(), if subflows.len() == 1 { "" } else { "s" }));
    Ok(Imported { doc, notes })
}

fn wire(id: Id, from: Id, to: Id, dashed: bool) -> Conn {
    Conn {
        id,
        from: End::Glued { shape: from, port: 1 },
        to: End::Glued { shape: to, port: 3 },
        routing: Routing::Curved,
        arrow_end: Arrow::None,
        style: Style { stroke: Some(Col::hex("#999999")), width: Some(2.0), dash: if dashed { Dash::Dashed } else { Dash::Solid }, ..Default::default() },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOWS: &str = r##"[
      {"id":"t1","type":"tab","label":"Tank monitor"},
      {"id":"b1","type":"mqtt-broker","name":"Plant broker","broker":"10.0.0.2","port":"1883"},
      {"id":"g1","type":"group","z":"t1","name":"Ingest","style":{"stroke":"#2563EB","fill":"#dbeafe","fill-opacity":"0.4"},"nodes":["n1","n2"],"x":34,"y":39,"w":412,"h":82},
      {"id":"n1","type":"mqtt in","z":"t1","g":"g1","name":"","topic":"plant/tank1/level","broker":"b1","x":130,"y":80,"wires":[["n2"]]},
      {"id":"n2","type":"function","z":"t1","g":"g1","name":"scale to %","func":"msg.payload = msg.payload / 4;\nreturn msg;","outputs":1,"x":340,"y":80,"wires":[["n3","n4"]]},
      {"id":"n3","type":"debug","z":"t1","name":"","x":560,"y":60,"wires":[]},
      {"id":"n4","type":"subflow:s1","z":"t1","name":"","x":560,"y":120,"wires":[[]]},
      {"id":"c1","type":"comment","z":"t1","name":"Level from the radar sensor","x":160,"y":180,"wires":[]},
      {"id":"s1","type":"subflow","name":"Alarm check","in":[{"x":60,"y":80,"wires":[{"id":"s1n1"}]}],"out":[{"x":360,"y":80,"wires":[{"id":"s1n1","port":0}]}],"color":"#E9967A"},
      {"id":"s1n1","type":"switch","z":"s1","name":"over 90 %","x":200,"y":80,"wires":[[]]}
    ]"##;

    #[test]
    fn reads_tabs_subflows_groups_and_wires() {
        assert!(looks_like(FLOWS));
        assert!(!looks_like(r#"{"pages":[]}"#));
        let imp = import(FLOWS).unwrap();
        let d = &imp.doc;
        assert_eq!(d.pages.len(), 2);
        assert_eq!(d.pages[0].name, "Tank monitor");
        assert_eq!(d.pages[1].name, "Subflow: Alarm check");
        let p = &d.pages[0];
        let names: Vec<&str> = p.shapes.iter().map(|s| s.text.as_str()).collect();
        assert!(names.contains(&"plant/tank1/level") && names.contains(&"scale to %") && names.contains(&"Alarm check") && names.contains(&"Ingest"), "{names:?}");
        assert_eq!(p.conns.len(), 3);
        // the subflow instance opens the subflow page
        let inst = p.shapes.iter().find(|s| s.text == "Alarm check").unwrap();
        assert_eq!(inst.link, Some(d.pages[1].id));
        // the group is behind its nodes, translucent
        assert_eq!(p.shapes[0].kind, "boundary");
        // the broker is listed
        assert!(p.shapes.iter().any(|s| s.kind == "note" && s.text.contains("Plant broker (10.0.0.2:1883)")));
        // subflow page: in → switch → out
        assert_eq!(d.pages[1].conns.len(), 2);
        assert!(imp.notes[0].contains("1 flow and 1 subflow"));
    }
}
