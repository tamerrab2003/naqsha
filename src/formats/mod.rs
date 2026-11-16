//! Interchange formats: Visio (.vsdx), draw.io (.drawio/.xml), SVG, Node-RED flows.
pub mod drawio;
pub mod nodered;
pub mod pipeline;
pub mod svgin;
pub mod vsdx;

use crate::model::{Col, Document};

pub struct Imported {
    pub doc: Document,
    /// Things the user should know about (skipped content, approximations).
    pub notes: Vec<String>,
}

pub fn hex_col(s: &str) -> Option<Col> {
    let s = s.trim().trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
    match s.len() {
        6 => Some(Col([p(0)?, p(2)?, p(4)?, 255])),
        8 => Some(Col([p(0)?, p(2)?, p(4)?, p(6)?])),
        3 => { let q = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|v| v * 17); Some(Col([q(0)?, q(1)?, q(2)?, 255])) }
        _ => None,
    }
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e < 10) else { out.push('&'); rest = &rest[1..]; continue };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'), "apos" => Some('\''), "nbsp" => Some(' '),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch { Some(c) => { out.push(c); rest = &rest[end + 1..]; } None => { out.push('&'); rest = &rest[1..]; } }
    }
    out.push_str(rest);
    out
}

/// Convert an HTML label (draw.io `html=1`) to plain text with line breaks.
/// The formatting most of an HTML label's text has (bold, italic, size, colour), so a label like
/// `<b><font style="font-size: 20px">Activities</font></b>` keeps its look when the tags go.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlLook { pub bold: Option<bool>, pub italic: Option<bool>, pub size: Option<f32>, pub color: Option<Col> }

fn css_color(v: &str) -> Option<Col> {
    let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
    if v.starts_with('#') { return hex_col(v); }
    let inner = v.strip_prefix("rgba(").or_else(|| v.strip_prefix("rgb("))?.trim_end_matches(')');
    let n: Vec<u8> = inner.split(',').take(3).filter_map(|x| x.trim().parse::<f32>().ok()).map(|x| x.clamp(0.0, 255.0) as u8).collect();
    (n.len() == 3).then(|| Col([n[0], n[1], n[2], 255]))
}

fn css_size(v: &str) -> Option<f32> {
    let v = v.trim();
    let num = |s: &str| s.trim().parse::<f32>().ok();
    if let Some(x) = v.strip_suffix("px") { return num(x); }
    if let Some(x) = v.strip_suffix("pt") { return num(x).map(|p| p * 4.0 / 3.0); }
    num(v)
}

pub fn html_look(s: &str) -> HtmlLook {
    #[derive(Clone, Default)]
    struct F { bold: bool, italic: bool, size: Option<f32>, color: Option<Col> }
    let mut stack: Vec<(String, F)> = vec![];
    let mut cur = F::default();
    let (mut total, mut bold, mut italic) = (0usize, 0usize, 0usize);
    let mut sizes: Vec<(f32, usize)> = vec![];
    let mut colors: Vec<(Col, usize)> = vec![];
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'<' {
            let end = s[i..].find('>').map(|e| i + e).unwrap_or(s.len() - 1);
            let inner = &s[i + 1..end];
            let closing = inner.starts_with('/');
            let name = inner.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
            if closing {
                if let Some(pos) = stack.iter().rposition(|(n, _)| *n == name) { cur = stack[pos].1.clone(); stack.truncate(pos); }
            } else if !matches!(name.as_str(), "br" | "img" | "hr" | "") && !inner.ends_with('/') {
                stack.push((name.clone(), cur.clone()));
                if matches!(name.as_str(), "b" | "strong" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6") { cur.bold = true; }
                if matches!(name.as_str(), "i" | "em") { cur.italic = true; }
                // attributes: <font size="5" color="#f00">, style="font-size: 20px; color: red; font-weight: bold"
                let attr = |a: &str| -> Option<String> {
                    let lower = inner.to_ascii_lowercase();
                    let k = lower.find(&format!("{a}="))?;
                    let rest = &inner[k + a.len() + 1..];
                    let q = rest.chars().next()?;
                    if q == '"' || q == '\'' { rest[1..].find(q).map(|e| rest[1..1 + e].to_string()) } else { Some(rest.split_whitespace().next()?.to_string()) }
                };
                if name == "font" {
                    if let Some(n) = attr("size").and_then(|v| v.trim().parse::<usize>().ok()) { cur.size = [10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0].get(n.clamp(1, 7) - 1).copied(); }
                    if let Some(c) = attr("color").and_then(|v| css_color(&v)) { cur.color = Some(c); }
                }
                if let Some(st) = attr("style") {
                    for decl in st.split(';') {
                        let Some((k, v)) = decl.split_once(':') else { continue };
                        match k.trim().to_ascii_lowercase().as_str() {
                            "font-size" => if let Some(z) = css_size(v) { cur.size = Some(z); },
                            "color" => if let Some(c) = css_color(v) { cur.color = Some(c); },
                            "font-weight" => { let v = v.trim(); cur.bold = v == "bold" || v.parse::<u32>().map_or(false, |w| w >= 600); }
                            "font-style" => cur.italic = v.trim() == "italic",
                            _ => {}
                        }
                    }
                }
            }
            i = end + 1;
        } else {
            let next = s[i..].find('<').map(|e| i + e).unwrap_or(s.len());
            let n = decode_entities(&s[i..next]).chars().filter(|c| !c.is_whitespace()).count();
            if n > 0 {
                total += n;
                if cur.bold { bold += n; }
                if cur.italic { italic += n; }
                if let Some(z) = cur.size { match sizes.iter_mut().find(|x| (x.0 - z).abs() < 0.01) { Some(x) => x.1 += n, None => sizes.push((z, n)) } }
                if let Some(c) = cur.color { match colors.iter_mut().find(|x| x.0 == c) { Some(x) => x.1 += n, None => colors.push((c, n)) } }
            }
            i = next;
        }
    }
    if total == 0 { return HtmlLook::default(); }
    // what most of the text has (a label with one bold word stays plain)
    let most = |n: usize| n * 10 >= total * 6;
    HtmlLook {
        bold: most(bold).then_some(true),
        italic: most(italic).then_some(true),
        size: sizes.iter().max_by_key(|x| x.1).filter(|x| x.1 * 2 >= total).map(|x| x.0),
        color: colors.iter().max_by_key(|x| x.1).filter(|x| x.1 * 2 >= total).map(|x| x.0),
    }
}

pub fn html_to_text(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            let end = s[i..].find('>').map(|e| i + e).unwrap_or(b.len() - 1);
            let tag = s[i + 1..end].trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
            let closing = s[i + 1..].starts_with('/');
            if tag == "br" || (closing && matches!(tag.as_str(), "div" | "p" | "li" | "tr")) || (!closing && matches!(tag.as_str(), "div" | "p") && !out.is_empty() && !out.ends_with('\n')) {
                out.push('\n');
            }
            i = end + 1;
        } else {
            let next = s[i..].find('<').map(|e| i + e).unwrap_or(b.len());
            out.push_str(&s[i..next]);
            i = next;
        }
    }
    let t = decode_entities(&out);
    let lines: Vec<&str> = t.lines().map(|l| l.trim_end()).collect();
    lines.join("\n").trim_matches('\n').to_string()
}

#[cfg(test)]
mod html_tests {
    use super::*;

    #[test]
    fn reads_html_formatting() {
        let l = html_look("<b><font style=\"font-size: 20px\">Activities</font></b>");
        assert_eq!((l.bold, l.size), (Some(true), Some(20.0)));
        let l = html_look("<font color=\"#ff0000\" size=\"5\">Red</font>");
        assert_eq!((l.color, l.size), (Some(Col([255, 0, 0, 255])), Some(24.0)));
        // one bold word in a sentence: the label stays plain
        assert_eq!(html_look("A <b>bold</b> word in a longer sentence").bold, None);
        assert_eq!(html_look("<span style=\"color: rgb(0, 128, 0); font-weight: 700\">ok</span>").color, Some(Col([0, 128, 0, 255])));
    }
}
