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
    /// Presenting, `t` seconds into the step: hidden items are gone, new ones fade in.
    Play { t: f32 },
}

pub const FADE: f32 = 0.45;
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

/// The page as it looks at step `k` (None: the page as drawn, no story).
pub fn apply(p: &Page, k: Option<usize>, mode: Mode, accent: Col) -> Cow<'_, Page> {
    let Some(k) = k.filter(|k| *k < p.story.len()) else { return Cow::Borrowed(p) };
    let s = state(p, k);
    let step = &p.story[k];
    let mut out = p.clone();
    let new_now: HashSet<Id> = step.changes.iter().filter(|c| c.act == Act::Show).map(|c| c.id).collect();
    // a connector goes with the shapes it is glued to
    let hidden_conn = |c: &Conn| s.hidden.contains(&c.id) || [c.from, c.to].iter().any(|e| matches!(e, End::Glued { shape, .. } if s.hidden.contains(shape)));
    let conn_new = |c: &Conn| new_now.contains(&c.id) || [c.from, c.to].iter().any(|e| matches!(e, End::Glued { shape, .. } if new_now.contains(shape)));
    let spot: HashSet<Id> = step.spot.iter().copied().collect();
    let in_fade = match mode { Mode::Play { t } => ease(t / FADE), Mode::Preview => 1.0 };
    for sh in &mut out.shapes {
        if let Some(t) = s.text.get(&sh.id) { sh.text = t.clone(); }
        if let Some(a) = s.look.get(&sh.id) { look(&mut sh.style, a, false); }
        if spot.contains(&sh.id) { sh.style.anim = Anim::Glow; sh.style.anim_color = Some(accent); sh.style.anim_speed = Some(1.4); }
        else if step.dim && !spot.is_empty() { fade(&mut sh.style, 0.3); }
        if s.hidden.contains(&sh.id) { fade(&mut sh.style, 0.16); }
        else if new_now.contains(&sh.id) { fade(&mut sh.style, in_fade); }
    }
    let hidden_conns: HashSet<Id> = out.conns.iter().filter(|c| hidden_conn(c)).map(|c| c.id).collect();
    for c in &mut out.conns {
        if let Some(t) = s.text.get(&c.id) { c.text = t.clone(); }
        if let Some(a) = s.look.get(&c.id) { look(&mut c.style, a, true); }
        if spot.contains(&c.id) { c.style.anim = Anim::Glow; c.style.anim_color = Some(accent); c.style.anim_speed = Some(1.4); }
        else if step.dim && !spot.is_empty() { fade(&mut c.style, 0.3); }
        if hidden_conns.contains(&c.id) { fade(&mut c.style, 0.16); }
        else if conn_new(c) { fade(&mut c.style, in_fade); }
    }
    if let Mode::Play { .. } = mode {
        out.shapes.retain(|sh| !s.hidden.contains(&sh.id));
        out.conns.retain(|c| !hidden_conns.contains(&c.id));
    }
    Cow::Owned(out)
}

/// Whether anything is still fading in at time `t` of a step.
pub fn settling(p: &Page, k: usize, t: f32) -> bool {
    t < FADE && p.story.get(k).map_or(false, |s| s.changes.iter().any(|c| c.act == Act::Show))
}

/// How a change reads in the step's list: "Appears", "Runs"…
pub fn act_label(a: &Act) -> &'static str {
    match a { Act::Show => "Appears", Act::Hide => "Disappears", Act::Run => "Runs", Act::Stop => "Stops", Act::Alarm => "Alarm", Act::Normal => "Back to normal", Act::Text(_) => "New text" }
}

/// Sets what a step does to `id`. Showing and hiding are one kind of change, the look (run, stop,
/// alarm, normal) another, the text a third: a new one replaces the step's earlier one of its kind.
pub fn set_change(step: &mut Step, id: Id, act: Act) {
    let kind = |a: &Act| match a { Act::Show | Act::Hide => 0, Act::Text(_) => 2, _ => 1 };
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
    fn a_new_change_replaces_one_of_its_kind() {
        let mut st = Step::default();
        set_change(&mut st, 1, Act::Show);
        set_change(&mut st, 1, Act::Run);
        set_change(&mut st, 1, Act::Alarm);
        set_change(&mut st, 1, Act::Hide);
        assert_eq!(st.changes, vec![Change { id: 1, act: Act::Alarm }, Change { id: 1, act: Act::Hide }]);
    }
}
