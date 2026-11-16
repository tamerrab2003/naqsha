//! Story mode: a page's steps, shown one after another when presenting. Each step can make items
//! appear or disappear, start or stop their animation, raise an alarm, change their text, point
//! at some of them and move the view. The page itself never changes: like live values, a step is
//! drawn as an overlay of the page.
use crate::model::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// While writing the story: hidden items stay faintly visible so they can still be edited.
    Preview,
    /// Presenting, `t` seconds into the step: hidden items are gone, new ones come on one by one.
    Play { t: f32 },
}

pub const ALARM: &str = "#DC2626";

/// Items that appear at some step: hidden until then.
pub fn appearing(p: &Page) -> HashSet<Id> {
    let mut first: HashMap<Id, bool> = HashMap::new();
    for st in &p.story {
        for c in &st.changes {
            match c.act { Act::Show => { first.entry(c.id).or_insert(true); } Act::Hide => { first.entry(c.id).or_insert(false); } _ => {} }
        }
    }
    first.into_iter().filter(|(_, show)| *show).map(|(id, _)| id).collect()
}

/// The state after steps 0..=k: hidden items, their looks and texts.
struct State { hidden: HashSet<Id>, look: HashMap<Id, Act>, text: HashMap<Id, String> }

fn state(p: &Page, k: usize) -> State {
    let mut s = State { hidden: appearing(p), look: HashMap::new(), text: HashMap::new() };
    for st in p.story.iter().take(k + 1) {
        for c in &st.changes {
            match &c.act {
                Act::Show => { s.hidden.remove(&c.id); }
                Act::Hide => { s.hidden.insert(c.id); }
                Act::Run | Act::Stop | Act::Alarm => { s.look.insert(c.id, c.act.clone()); }
                Act::Normal => { s.look.remove(&c.id); s.text.remove(&c.id); }
                Act::Text(t) => { s.text.insert(c.id, t.clone()); }
                Act::Send => {}
            }
        }
    }
    s
}

fn look(st: &mut Style, a: &Act, conn: bool) {
    match a {
        Act::Run => { if st.anim == Anim::None { st.anim = if conn { Anim::Flow } else { Anim::Pulse }; } }
        Act::Stop => st.anim = Anim::None,
        Act::Alarm => { st.anim = Anim::Glow; st.anim_color = Some(Col::hex(ALARM)); st.anim_speed = Some(2.0); }
        _ => {}
    }
}

fn fade(st: &mut Style, f: f32) { st.opacity = Some(st.opacity.unwrap_or(1.0) * f); }

fn ease(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }
fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }
/// Overshoots a little past 1 before settling: the bounce of a pop.
fn back_out(t: f32) -> f32 { let (c1, u) = (1.70158, t.clamp(0.0, 1.0) - 1.0); 1.0 + (c1 + 1.0) * u * u * u + c1 * u * u }

/// Seconds one item takes to come on, to go, a number to count up, and before the first one starts.
const DUR: f32 = 0.55;
const OUT: f32 = 0.4;
const COUNT: f32 = 1.2;
const LEAD: f32 = 0.12;
/// A sent dot takes TRAVEL seconds along its connector, then starts again every CYCLE seconds.
const TRAVEL: f32 = 1.3;
const CYCLE: f32 = 1.9;
const SHAKE: f32 = 0.7;
/// How long a step plays in the editor when you preview its motion.
pub const PREVIEW_SECS: f32 = 4.5;

fn ends(c: &Conn) -> impl Iterator<Item = Id> + '_ { [c.from, c.to].into_iter().filter_map(|e| match e { End::Glued { shape, .. } => Some(shape), _ => None }) }
fn conn_vis(c: &Conn, hidden: &HashSet<Id>) -> bool { !hidden.contains(&c.id) && !ends(c).any(|s| hidden.contains(&s)) }

/// What changes in view at step `k`: what comes on (in order, from when) and what goes.
struct Timeline { start: HashMap<Id, f32>, leaving: HashSet<Id>, end: f32 }

fn timeline(p: &Page, k: usize) -> Timeline {
    let now = state(p, k).hidden;
    let before = if k == 0 { appearing(p) } else { state(p, k - 1).hidden };
    let step = &p.story[k];
    let new_shape = |id: &Id| p.shape(*id).is_some() && !now.contains(id) && before.contains(id);
    let new_conns: Vec<&Conn> = p.conns.iter().filter(|c| conn_vis(c, &now) && !conn_vis(c, &before)).collect();
    // shapes in the order they were added to the step; each one's connectors right after it,
    // once the shape at their other end is there too
    let mut order: Vec<Id> = vec![];
    let mut placed: HashSet<Id> = HashSet::new();
    let ready = |c: &Conn, placed: &HashSet<Id>| ends(c).all(|s| !(new_shape(&s)) || placed.contains(&s));
    let mut shapes: Vec<Id> = step.changes.iter().filter(|c| c.act == Act::Show && new_shape(&c.id)).map(|c| c.id).collect();
    for sh in &p.shapes { if new_shape(&sh.id) && !shapes.contains(&sh.id) { shapes.push(sh.id); } }
    for id in shapes {
        if !placed.insert(id) { continue; }
        order.push(id);
        for c in &new_conns {
            if !placed.contains(&c.id) && ends(c).any(|s| s == id) && ready(c, &placed) { placed.insert(c.id); order.push(c.id); }
        }
    }
    for c in &new_conns { if placed.insert(c.id) { order.push(c.id); } }
    let n = order.len();
    let gap = if step.together || n < 2 { 0.0 } else { (1.6 / (n - 1) as f32).min(0.22) };
    let start: HashMap<Id, f32> = order.iter().enumerate().map(|(i, id)| (*id, LEAD + i as f32 * gap)).collect();
    let end = if n == 0 { 0.0 } else { LEAD + (n - 1) as f32 * gap + DUR };
    let mut leaving: HashSet<Id> = p.shapes.iter().filter(|s| now.contains(&s.id) && !before.contains(&s.id)).map(|s| s.id).collect();
    leaving.extend(p.conns.iter().filter(|c| !conn_vis(c, &now) && conn_vis(c, &before)).map(|c| c.id));
    Timeline { start, leaving, end }
}

/// The first number in a text: where it is, its value, its decimals and whether it groups thousands.
fn number(s: &str) -> Option<(usize, usize, f64, usize, bool)> {
    let b = s.as_bytes();
    let i = b.iter().position(|c| c.is_ascii_digit())?;
    let a = if i > 0 && b[i - 1] == b'-' { i - 1 } else { i };
    let mut j = i;
    while j < b.len() && (b[j].is_ascii_digit() || ((b[j] == b'.' || b[j] == b',') && b.get(j + 1).map_or(false, |c| c.is_ascii_digit()))) { j += 1; }
    let t = &s[a..j];
    let v: f64 = t.replace(',', "").parse().ok()?;
    let dec = t.rfind('.').map_or(0, |d| t.len() - d - 1);
    Some((a, j, v, dec, t.contains(',')))
}

/// `new` with its number partway (`f`) from the one in `old` (0 when there is none).
pub fn count_up(old: &str, new: &str, f: f32) -> Option<String> {
    let (a, b, v1, dec, group) = number(new)?;
    let v0 = number(old).map_or(0.0, |n| n.2);
    if (v1 - v0).abs() < 1e-9 { return None; }
    let v = v0 + (v1 - v0) * f as f64;
    let mut txt = format!("{:.*}", dec, v);
    if group {
        let (sign, rest) = if txt.starts_with('-') { ("-", &txt[1..]) } else { ("", &txt[..]) };
        let (int, frac) = rest.split_once('.').map_or((rest, None), |(i, f)| (i, Some(f)));
        let mut g = String::new();
        for (i, ch) in int.chars().enumerate() { if i > 0 && (int.len() - i) % 3 == 0 { g.push(','); } g.push(ch); }
        txt = format!("{sign}{g}{}", frac.map_or(String::new(), |d| [".", d].concat()));
    }
    Some(format!("{}{}{}", &new[..a], txt, &new[b..]))
}

/// Scales a shape about its centre (and its text with it).
fn scale(sh: &mut Shape, k: f32) {
    let (cx, cy) = sh.center();
    sh.w *= k; sh.h *= k;
    sh.x = cx - sh.w / 2.0; sh.y = cy - sh.h / 2.0;
    sh.style.font_size = Some(sh.style.font_size.unwrap_or(13.0) * k.max(0.05));
}

/// The page as it looks at step `k` (None: the page as drawn, no story).
pub fn apply(p: &Page, k: Option<usize>, mode: Mode, accent: Col) -> Cow<'_, Page> {
    let Some(k) = k.filter(|k| *k < p.story.len()) else { return Cow::Borrowed(p) };
    let s = state(p, k);
    let step = &p.story[k];
    let mut out = p.clone();
    let play = match mode { Mode::Play { t } => Some(t), Mode::Preview => None };
    let tl = timeline(p, k);
    let prev_text = if k == 0 { HashMap::new() } else { state(p, k - 1).text };
    let here = |id: Id, a: &Act| step.changes.iter().any(|c| c.id == id && &c.act == a);
    let spot: HashSet<Id> = step.spot.iter().copied().collect();
    let hidden_conns: HashSet<Id> = p.conns.iter().filter(|c| !conn_vis(c, &s.hidden)).map(|c| c.id).collect();
    for sh in &mut out.shapes {
        if let Some(t) = s.text.get(&sh.id) {
            let old = prev_text.get(&sh.id).cloned().unwrap_or_else(|| sh.text.clone());
            sh.text = match play { Some(tt) if tt < COUNT && here(sh.id, &Act::Text(t.clone())) => count_up(&old, t, ease_out(tt / COUNT)).unwrap_or_else(|| t.clone()), _ => t.clone() };
        }
        if let Some(a) = s.look.get(&sh.id) { look(&mut sh.style, a, false); }
        if matches!(play, Some(t) if t < SHAKE) && here(sh.id, &Act::Alarm) { sh.style.anim = Anim::Shake; sh.style.anim_speed = Some(1.0); }
        if spot.contains(&sh.id) { sh.style.anim = Anim::Glow; sh.style.anim_color = Some(accent); sh.style.anim_speed = Some(1.4); }
        else if step.dim && !spot.is_empty() { fade(&mut sh.style, 0.3); }
        match play {
            None => if s.hidden.contains(&sh.id) { fade(&mut sh.style, 0.16); },
            Some(t) => {
                if tl.leaving.contains(&sh.id) {
                    let f = (t / OUT).clamp(0.0, 1.0);
                    fade(&mut sh.style, 1.0 - ease(f));
                    scale(sh, 1.0 - 0.15 * ease(f));
                } else if let Some(st) = tl.start.get(&sh.id) {
                    let f = ((t - st) / DUR).clamp(0.0, 1.0);
                    match step.enter {
                        Enter::Pop => { fade(&mut sh.style, (f * 3.0).min(1.0)); if f < 1.0 { scale(sh, 0.55 + 0.45 * back_out(f)); } }
                        Enter::Rise => { fade(&mut sh.style, ease_out(f)); sh.y += 26.0 * (1.0 - ease_out(f)); }
                        Enter::Fade => fade(&mut sh.style, ease(f)),
                    }
                }
            }
        }
    }
    for c in &mut out.conns {
        if let Some(t) = s.text.get(&c.id) { c.text = t.clone(); }
        if let Some(a) = s.look.get(&c.id) { look(&mut c.style, a, true); }
        if spot.contains(&c.id) { c.style.anim = Anim::Glow; c.style.anim_color = Some(accent); c.style.anim_speed = Some(1.4); }
        else if step.dim && !spot.is_empty() { fade(&mut c.style, 0.3); }
        match play {
            None => if hidden_conns.contains(&c.id) { fade(&mut c.style, 0.16); },
            Some(t) => {
                if tl.leaving.contains(&c.id) { fade(&mut c.style, 1.0 - ease(t / OUT)); }
                else if let (Some(st), Enter::Fade) = (tl.start.get(&c.id), step.enter) { fade(&mut c.style, ease((t - st) / DUR)); }
            }
        }
    }
    if let Some(t) = play {
        let going = |id: &Id| tl.leaving.contains(id) && t < OUT;
        out.shapes.retain(|sh| !s.hidden.contains(&sh.id) || going(&sh.id));
        out.conns.retain(|c| !hidden_conns.contains(&c.id) || going(&c.id));
    }
    Cow::Owned(out)
}

/// What is drawn on top of a step's page while presenting: connectors partly drawn (how much, 0..1)
/// and dots travelling along connectors (how far, 0..1).
#[derive(Default, Debug)]
pub struct Fx { pub reveal: HashMap<Id, f32>, pub dots: Vec<(Id, f32)> }

pub fn fx(p: &Page, k: Option<usize>, t: f32) -> Fx {
    let mut fx = Fx::default();
    let Some(k) = k.filter(|k| *k < p.story.len()) else { return fx };
    let step = &p.story[k];
    let tl = timeline(p, k);
    if step.enter != Enter::Fade {
        for c in &p.conns {
            if let Some(st) = tl.start.get(&c.id) { let f = ((t - st) / DUR).clamp(0.0, 1.0); if f < 1.0 { fx.reveal.insert(c.id, ease_out(f)); } }
        }
    }
    let from = t - tl.end;
    if from >= 0.0 {
        let u = from % CYCLE;
        if u < TRAVEL {
            let f = ease(u / TRAVEL);
            let hidden = state(p, k).hidden;
            for ch in step.changes.iter().filter(|c| c.act == Act::Send) {
                for c in p.conns.iter().filter(|c| conn_vis(c, &hidden)) {
                    let leaves = matches!(c.from, End::Glued { shape, .. } if shape == ch.id);
                    if (c.id == ch.id || leaves) && !fx.dots.iter().any(|d| d.0 == c.id) { fx.dots.push((c.id, f)); }
                }
            }
        }
    }
    fx
}

/// Whether anything is still moving at time `t` of a step (sent dots keep going all step).
pub fn settling(p: &Page, k: usize, t: f32) -> bool {
    let Some(step) = p.story.get(k) else { return false };
    let tl = timeline(p, k);
    t < tl.end + 0.05 || (!tl.leaving.is_empty() && t < OUT + 0.05)
        || step.changes.iter().any(|c| c.act == Act::Send || (matches!(c.act, Act::Text(_)) && t < COUNT + 0.05) || (c.act == Act::Alarm && t < SHAKE + 0.05))
}

/// How a change reads in the step's list: "Appears", "Runs"…
pub fn act_label(a: &Act) -> &'static str {
    match a { Act::Show => "Appears", Act::Hide => "Disappears", Act::Run => "Runs", Act::Stop => "Stops", Act::Alarm => "Alarm", Act::Normal => "Back to normal", Act::Text(_) => "New text", Act::Send => "Sends" }
}

/// Sets what a step does to `id`. Showing and hiding are one kind of change, the look (run, stop,
/// alarm, normal) another, the text a third: a new one replaces the step's earlier one of its kind.
pub fn set_change(step: &mut Step, id: Id, act: Act) {
    let kind = |a: &Act| match a { Act::Show | Act::Hide => 0, Act::Text(_) => 2, Act::Send => 3, _ => 1 };
    let k = kind(&act);
    step.changes.retain(|c| !(c.id == id && (kind(&c.act) == k || (k == 1 && act == Act::Normal && kind(&c.act) == 2))));
    step.changes.push(Change { id, act });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        let mut p = Page::default();
        for (id, x) in [(1, 0.0), (2, 200.0), (3, 400.0)] { p.shapes.push(Shape { id, kind: "rect".into(), x, y: 0.0, w: 100.0, h: 60.0, text: format!("S{id}"), ..Default::default() }); }
        p.conns.push(Conn { id: 10, from: End::Glued { shape: 1, port: -1 }, to: End::Glued { shape: 2, port: -1 }, ..Default::default() });
        p
    }

    #[test]
    fn steps_build_up() {
        let mut p = page();
        p.story = vec![
            Step { caption: "one".into(), ..Default::default() },
            Step { changes: vec![Change { id: 2, act: Act::Show }, Change { id: 1, act: Act::Run }], ..Default::default() },
            Step { changes: vec![Change { id: 2, act: Act::Alarm }, Change { id: 3, act: Act::Hide }, Change { id: 1, act: Act::Text("on".into()) }], spot: vec![2], dim: true, ..Default::default() },
        ];
        let acc = Col::hex("#6D4AFF");
        // step 0: shape 2 appears later, so it and the connector to it are not there yet
        let s0 = apply(&p, Some(0), Mode::Play { t: 1.0 }, acc);
        assert!(s0.shape(2).is_none() && s0.conns.is_empty() && s0.shape(1).is_some());
        // while writing the story they stay, faintly
        let pv = apply(&p, Some(0), Mode::Preview, acc);
        assert!(pv.shape(2).unwrap().style.opacity.unwrap() < 0.5);
        // step 1: it fades in with its connector; shape 1 runs
        let s1 = apply(&p, Some(1), Mode::Play { t: 0.1 }, acc);
        assert!(s1.shape(2).unwrap().style.opacity.unwrap() < 0.5 && s1.conns.len() == 1);
        assert_eq!(s1.shape(1).unwrap().style.anim, Anim::Pulse);
        assert_eq!(apply(&p, Some(1), Mode::Play { t: 2.0 }, acc).shape(2).unwrap().style.opacity, Some(1.0));
        // step 2: changes add up; the spotlight and dimming last for this step only
        let s2 = apply(&p, Some(2), Mode::Play { t: 2.0 }, acc);
        assert!(s2.shape(3).is_none());
        assert_eq!(s2.shape(1).unwrap().text, "on");
        assert_eq!(s2.shape(1).unwrap().style.anim, Anim::Pulse, "still running");
        assert_eq!(s2.shape(2).unwrap().style.anim_color, Some(acc), "spotlight");
        assert!(s2.shape(1).unwrap().style.opacity.unwrap() < 0.5, "dimmed");
        // the page itself never changes
        assert!(p.shapes.iter().all(|s| s.style.opacity.is_none() && s.style.anim == Anim::None));
        assert!(matches!(apply(&p, None, Mode::Preview, acc), Cow::Borrowed(_)));
    }

    #[test]
    fn items_come_on_one_after_another() {
        let mut p = page();
        p.story = vec![
            Step::default(),
            Step { changes: vec![Change { id: 2, act: Act::Show }, Change { id: 3, act: Act::Show }, Change { id: 10, act: Act::Send }], ..Default::default() },
            Step { changes: vec![Change { id: 2, act: Act::Hide }], ..Default::default() },
        ];
        let tl = timeline(&p, 1);
        // shape 2, then the connector to it, then shape 3
        assert!(tl.start[&2] < tl.start[&10] && tl.start[&10] < tl.start[&3]);
        let acc = Col::hex("#6D4AFF");
        // early on, shape 3 hasn't started: not drawn yet; shape 2 pops (smaller than drawn)
        let early = apply(&p, Some(1), Mode::Play { t: tl.start[&2] + 0.1 }, acc);
        assert_eq!(early.shape(3).unwrap().style.opacity, Some(0.0));
        assert!(early.shape(2).unwrap().w < 100.0);
        // the connector is drawn partway, and the dot waits until everything is on
        let f = fx(&p, Some(1), tl.start[&10] + 0.2);
        assert!(f.reveal[&10] > 0.0 && f.reveal[&10] < 1.0 && f.dots.is_empty());
        let later = fx(&p, Some(1), tl.end + 0.5);
        assert!(later.reveal.is_empty() && later.dots.len() == 1);
        // all at once: same start
        p.story[1].together = true;
        let tl = timeline(&p, 1);
        assert_eq!(tl.start[&2], tl.start[&3]);
        // leaving: still there, fading, for a moment; the dot from shape 1 is gone with its connector
        let out = apply(&p, Some(2), Mode::Play { t: 0.1 }, acc);
        assert!(out.shape(2).unwrap().style.opacity.unwrap() < 1.0);
        assert!(apply(&p, Some(2), Mode::Play { t: 1.0 }, acc).shape(2).is_none());
        assert!(settling(&p, 1, 30.0), "a sent dot keeps going");
        assert!(!settling(&p, 2, 1.0));
    }

    #[test]
    fn numbers_count_up() {
        assert_eq!(count_up("0%", "80%", 0.5).unwrap(), "40%");
        assert_eq!(count_up("Unit tests", "Unit tests\n1,248 passed", 1.0).unwrap(), "Unit tests\n1,248 passed");
        assert_eq!(count_up("", "1,248", 0.5).unwrap(), "624");
        assert_eq!(count_up("", "12,480", 0.5).unwrap(), "6,240");
        assert_eq!(count_up("1.5 s", "3.0 s", 0.5).unwrap(), "2.2 s");
        assert!(count_up("", "OK", 0.5).is_none());
        assert!(count_up("7", "7 left", 0.5).is_none());
    }

    #[test]
    fn a_new_change_replaces_one_of_its_kind() {
        let mut st = Step::default();
        set_change(&mut st, 1, Act::Show);
        set_change(&mut st, 1, Act::Run);
        set_change(&mut st, 1, Act::Alarm);
        set_change(&mut st, 1, Act::Hide);
        assert_eq!(st.changes, vec![Change { id: 1, act: Act::Alarm }, Change { id: 1, act: Act::Hide }]);
    }
}
