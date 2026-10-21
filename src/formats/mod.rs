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
