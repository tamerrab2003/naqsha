// values only arrive in the browser build (the desktop app is never embedded)
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
//! Live values on a diagram (monitoring only: nothing is ever sent back).
//!
//! A host such as the Node-RED add-on sends values by tag. A value is matched to the shape whose
//! tag (the first line of its text, e.g. "LS-101") or whose "Live tag" shape-data field equals the
//! key, and to connectors by their "Live tag" field or their label. What a value does:
//! - a number or text is shown in a badge on the item (with its unit);
//! - true / "run" / "on" lets the item's animation play (a running pump spins, a pipe flows) and
//!   puts a green dot on it; false / "stop" / "off" stops the animation and greys the dot;
//! - "alarm" / "fault" / "trip" makes the item glow red and its badge turn red.
//!
//! A value can also be an object: `{value, unit, state}` with state run / stop / alarm.
use crate::model::*;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State { #[default] None, Run, Stop, Alarm }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Live {
    /// What the badge shows (None: no badge).
    pub text: Option<String>,
    pub state: State,
}

pub const TAG_FIELD: &str = "Live tag";
pub const UNIT_FIELD: &str = "Unit";

fn field<'a>(data: &'a [(String, String)], k: &str) -> Option<&'a str> { data.iter().find(|(n, _)| n == k).map(|(_, v)| v.trim()).filter(|v| !v.is_empty()) }

/// The key a shape answers to.
pub fn shape_key(s: &Shape) -> Option<String> {
    if let Some(k) = field(&s.data, TAG_FIELD) { return Some(k.to_string()); }
    if crate::library::is_instrument(&s.kind) {
        // an instrument bubble reads "LIT" over "101": its tag is LIT-101
        let parts: Vec<&str> = s.text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        return (!parts.is_empty()).then(|| parts.join("-"));
    }
    s.text.lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
}
/// The key a connector answers to.
pub fn conn_key(c: &Conn) -> Option<String> {
    field(&c.data, TAG_FIELD).map(String::from).or_else(|| Some(c.text.trim().to_string()).filter(|t| !t.is_empty()))
}

fn state_word(s: &str) -> Option<State> {
    match s.trim().to_lowercase().as_str() {
        "run" | "running" | "on" | "open" | "true" | "start" | "started" | "ok" => Some(State::Run),
        "stop" | "stopped" | "off" | "closed" | "false" | "idle" => Some(State::Stop),
        "alarm" | "fault" | "trip" | "tripped" | "error" | "fail" | "failed" => Some(State::Alarm),
        _ => None,
    }
}

fn number(v: f64) -> String {
    if v.fract().abs() < 1e-9 && v.abs() < 1e12 { format!("{}", v as i64) } else if v.abs() >= 100.0 { format!("{v:.1}") } else { format!("{v:.2}").trim_end_matches('0').trim_end_matches('.').to_string() }
}

/// Read one value as the host sent it.
pub fn parse(v: &Value) -> Live {
    match v {
        Value::Bool(b) => Live { text: None, state: if *b { State::Run } else { State::Stop } },
        Value::Number(n) => Live { text: n.as_f64().map(number), state: State::None },
        Value::String(s) => match state_word(s) { Some(st) => Live { text: None, state: st }, None => Live { text: Some(s.clone()), state: State::None } },
        Value::Object(o) => {
            let mut l = o.get("value").map(parse).unwrap_or_default();
            if let Some(st) = o.get("state").and_then(|s| s.as_str()).and_then(state_word) { l.state = st; }
            if o.get("alarm").and_then(|a| a.as_bool()) == Some(true) { l.state = State::Alarm; }
            if let (Some(t), Some(u)) = (l.text.as_mut(), o.get("unit").and_then(|u| u.as_str())) { if !u.is_empty() { t.push(' '); t.push_str(u); } }
            l
        }
        _ => Live::default(),
    }
}

/// The page as it should look with these values: stopped items stand still, alarms glow red.
pub fn apply<'a>(p: &'a Page, live: &HashMap<String, Live>) -> Cow<'a, Page> {
    if live.is_empty() { return Cow::Borrowed(p); }
    let mut out = p.clone();
    let set = |st: &mut Style, l: &Live| match l.state {
        State::Stop => st.anim = Anim::None,
        State::Alarm => { st.anim = Anim::Glow; st.anim_color = Some(Col::hex("#DC2626")); st.anim_speed = Some(2.0); }
        _ => {}
    };
    for s in &mut out.shapes { if let Some(l) = shape_key(s).and_then(|k| live.get(&k)) { set(&mut s.style, l); } }
    for c in &mut out.conns { if let Some(l) = conn_key(c).and_then(|k| live.get(&k)) { set(&mut c.style, l); } }
    Cow::Owned(out)
}

/// Badge text with the item's unit (from its "Unit" shape data) when the value has none.
pub fn badge_text(l: &Live, data: &[(String, String)]) -> Option<String> {
    let t = l.text.clone()?;
    match field(data, UNIT_FIELD) { Some(u) if t.parse::<f64>().is_ok() => Some(format!("{t} {u}")), _ => Some(t) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn values_states_and_matching() {
        assert_eq!(parse(&json!(72.456)).text.as_deref(), Some("72.46"));
        assert_eq!(parse(&json!(1500)).text.as_deref(), Some("1500"));
        assert_eq!(parse(&json!(true)).state, State::Run);
        assert_eq!(parse(&json!("Tripped")).state, State::Alarm);
        let o = parse(&json!({"value": 3.5, "unit": "bar", "state": "run"}));
        assert_eq!((o.text.as_deref(), o.state), (Some("3.5 bar"), State::Run));
        let mut p = Page::default();
        p.shapes.push(Shape { id: 1, text: "P-101\nDuty pump".into(), style: Style { anim: Anim::Spin, ..Default::default() }, ..Default::default() });
        p.conns.push(Conn { id: 2, data: vec![(TAG_FIELD.into(), "P-101".into())], style: Style { anim: Anim::Flow, ..Default::default() }, ..Default::default() });
        let mut live = HashMap::new();
        live.insert("P-101".to_string(), parse(&json!(false)));
        let q = apply(&p, &live);
        assert_eq!(q.shapes[0].style.anim, Anim::None);
        assert_eq!(q.conns[0].style.anim, Anim::None);
        live.insert("P-101".to_string(), parse(&json!("alarm")));
        assert_eq!(apply(&p, &live).shapes[0].style.anim, Anim::Glow);
        let unit = vec![(UNIT_FIELD.to_string(), "%".to_string())];
        assert_eq!(badge_text(&parse(&json!(64)), &unit).as_deref(), Some("64 %"));
        let lit = Shape { kind: "xi_field".into(), text: "LIT\n101".into(), ..Default::default() };
        assert_eq!(shape_key(&lit).as_deref(), Some("LIT-101"));
    }
}
