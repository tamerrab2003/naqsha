//! Moving diagrams. Every animation is a loop: its state at time `t` comes from `phase(t)`, a
//! number from 0 to 1 that wraps once per period (2 s at normal speed). The editor draws the
//! state at the current time, the GIF export bakes frames over one loop, and the SVG export
//! writes the same motion as CSS keyframes.
use crate::model::*;
use egui::{Pos2, Vec2};

/// Speeds offered in the format pane.
pub const SPEEDS: [(f32, &str); 3] = [(0.5, "Slow"), (1.0, "Normal"), (2.0, "Fast")];

pub fn speed(st: &Style) -> f32 { st.anim_speed.unwrap_or(1.0).clamp(0.25, 4.0) }
/// Seconds per loop of this style's animation.
pub fn period(st: &Style) -> f32 { 2.0 / speed(st) }
/// Where in its loop the animation is at time `t` (0..1).
pub fn phase(st: &Style, t: f64) -> f32 { ((t / period(st) as f64).rem_euclid(1.0)) as f32 }

pub fn name(a: Anim) -> &'static str {
    match a { Anim::None => "None", Anim::Flow => "Flow", Anim::Dots => "Moving dots", Anim::Glow => "Glow", Anim::Pulse => "Pulse", Anim::Blink => "Blink", Anim::Spin => "Spin", Anim::Shake => "Shake" }
}
pub const FOR_SHAPES: [Anim; 6] = [Anim::None, Anim::Glow, Anim::Pulse, Anim::Blink, Anim::Spin, Anim::Shake];
pub const FOR_CONNS: [Anim; 5] = [Anim::None, Anim::Flow, Anim::Dots, Anim::Glow, Anim::Blink];

pub fn page_moves(p: &Page) -> bool {
    p.shapes.iter().any(|s| s.style.anim != Anim::None) || p.conns.iter().any(|c| c.style.anim != Anim::None)
}

/// Length of one loop of everything on the page (the slowest period; the others divide it).
pub fn loop_secs(p: &Page) -> f32 {
    p.shapes.iter().map(|s| &s.style).chain(p.conns.iter().map(|c| &c.style))
        .filter(|st| st.anim != Anim::None).map(period).fold(0.0, f32::max).max(0.5)
}

/// 0..1..0 once per loop, smooth.
pub fn wave(ph: f32) -> f32 { 0.5 - 0.5 * (ph * std::f32::consts::TAU).cos() }

/// How strongly a glowing item glows (0.25..1).
pub fn glow_level(st: &Style, t: f64) -> f32 { 0.25 + 0.75 * wave(phase(st, t)) }

/// Is a blinking item in its dim half?
pub fn blink_off(st: &Style, t: f64) -> bool { phase(st, t) >= 0.5 }

/// The shape's geometry as drawn at time `t` (pulse, spin, shake and blink move or fade it).
pub fn shape_at(s: &Shape, t: f64) -> Option<Shape> {
    let ph = phase(&s.style, t);
    let mut a = s.clone();
    match s.style.anim {
        Anim::Pulse => {
            let k = 1.0 + 0.08 * wave(ph);
            let (cx, cy) = s.center();
            a.w = s.w * k; a.h = s.h * k;
            a.x = cx - a.w / 2.0; a.y = cy - a.h / 2.0;
        }
        Anim::Spin => a.rot = (s.rot + ph * 360.0).rem_euclid(360.0),
        Anim::Shake => a.x = s.x + 3.0 * (ph * std::f32::consts::TAU * 6.0).sin(),
        Anim::Blink if blink_off(&s.style, t) => a.style.opacity = Some(s.style.opacity.unwrap_or(1.0) * 0.18),
        _ => return None,
    }
    Some(a)
}

/// Flow dash pattern for a line of width `w` (dash, gap), in page units.
pub fn flow_dash(w: f32) -> (f32, f32) { (w * 3.0 + 9.0, w * 2.0 + 7.0) }
/// Distance between moving dots, and their radius.
pub fn dot_spacing(w: f32) -> (f32, f32) { (w * 6.0 + 18.0, w * 0.9 + 2.2) }
/// How far the pattern has moved along the line at this phase (patterns per loop: 3 for flow,
/// 2 for dots, so normal speed travels about 30 px a second).
pub fn travel(a: Anim, w: f32, ph: f32) -> f32 {
    match a {
        Anim::Flow => { let (d, g) = flow_dash(w); 3.0 * (d + g) * ph }
        Anim::Dots => 2.0 * dot_spacing(w).0 * ph,
        _ => 0.0,
    }
}

/// Pieces of a polyline covered by a dash pattern shifted `offset` along it (toward the end).
pub fn dashes(pts: &[Pos2], on: f32, off: f32, offset: f32) -> Vec<Vec<Pos2>> {
    let per = on + off;
    let mut out = vec![];
    if pts.len() < 2 || per <= 0.0 { return out; }
    // position s along the line is "on" when (s - offset) mod per < on
    let mut cur: Vec<Pos2> = vec![];
    let mut s0 = 0.0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b - a).length();
        if len <= 1e-4 { continue; }
        let dir = (b - a) / len;
        let mut s = 0.0;
        while s < len {
            let m = (s0 + s - offset).rem_euclid(per);
            let (on_now, left) = if m < on { (true, on - m) } else { (false, per - m) };
            let e = (s + left).min(len);
            if on_now {
                if cur.is_empty() { cur.push(a + dir * s); }
                cur.push(a + dir * e);
                // stopped by the pattern (not the corner): the dash ends here
                if e < len { out.push(std::mem::take(&mut cur)); }
            } else if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            s = e + 1e-4;
        }
        s0 += len;
    }
    if cur.len() > 1 { out.push(cur); }
    out
}

/// Centres of the moving dots along a polyline.
pub fn dots(pts: &[Pos2], spacing: f32, offset: f32) -> Vec<Pos2> {
    let mut out = vec![];
    if pts.len() < 2 || spacing <= 0.0 { return out; }
    let mut next = offset.rem_euclid(spacing);
    let mut s0 = 0.0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b - a).length();
        if len <= 1e-4 { continue; }
        let dir: Vec2 = (b - a) / len;
        while next <= s0 + len {
            out.push(a + dir * (next - s0));
            next += spacing;
        }
        s0 += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn dashes_cover_the_pattern_and_move() {
        let line = [pos2(0.0, 0.0), pos2(100.0, 0.0)];
        let d = dashes(&line, 10.0, 10.0, 0.0);
        assert_eq!(d.len(), 5);
        assert!((d[0][0].x - 0.0).abs() < 1e-2 && (d[0].last().unwrap().x - 10.0).abs() < 1e-2);
        let d = dashes(&line, 10.0, 10.0, 5.0);
        assert!((d[0][0].x - 5.0).abs() < 1e-2 && (d[0].last().unwrap().x - 15.0).abs() < 1e-2, "{:?}", d[0]);
        // a dash turns the corner of a right-angle route as one piece
        let l = [pos2(0.0, 0.0), pos2(25.0, 0.0), pos2(25.0, 50.0)];
        let d = dashes(&l, 10.0, 10.0, 0.0);
        assert_eq!(d[1].len(), 3, "{:?}", d[1]);
    }

    #[test]
    fn dots_and_loops() {
        let line = [pos2(0.0, 0.0), pos2(100.0, 0.0)];
        assert_eq!(dots(&line, 25.0, 0.0).len(), 5);
        assert!((dots(&line, 25.0, 10.0)[0].x - 10.0).abs() < 1e-3);
        let mut st = Style { anim: Anim::Glow, ..Default::default() };
        assert!((phase(&st, 0.5) - 0.25).abs() < 1e-5);
        st.anim_speed = Some(0.5);
        assert!((period(&st) - 4.0).abs() < 1e-5);
        let s = Shape { w: 100.0, h: 50.0, style: Style { anim: Anim::Pulse, ..Default::default() }, ..Default::default() };
        let big = shape_at(&s, 1.0).unwrap();
        assert!(big.w > 107.0 && (big.center().0 - 50.0).abs() < 1e-3);
    }
}
