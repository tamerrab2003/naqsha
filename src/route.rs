//! Obstacle-avoiding orthogonal routing and line jumps.
use egui::{pos2, vec2, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{Hash, Hasher};

const STUB: f32 = 20.0;
const MARGIN: f32 = 12.0;
const BEND: f32 = 28.0;

fn dir_idx(v: Vec2) -> usize {
    if v.x.abs() >= v.y.abs() { if v.x >= 0.0 { 0 } else { 2 } } else if v.y >= 0.0 { 1 } else { 3 }
}
const DIRS: [Vec2; 4] = [Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 0.0, y: 1.0 }, Vec2 { x: -1.0, y: 0.0 }, Vec2 { x: 0.0, y: -1.0 }];

fn strictly_inside(r: &Rect, p: Pos2) -> bool { p.x > r.min.x + 0.01 && p.x < r.max.x - 0.01 && p.y > r.min.y + 0.01 && p.y < r.max.y - 0.01 }

thread_local! {
    static CACHE: RefCell<HashMap<u64, Option<Vec<Pos2>>>> = RefCell::new(HashMap::new());
}

/// Right-angle route from `a` (leaving along `ad`) to `b` (entering against `bd`) that avoids `obstacles`.
/// Returns None when no route is found (caller falls back to the simple elbow route).
pub fn avoid(a: Pos2, ad: Vec2, b: Pos2, bd: Vec2, obstacles: &[Rect]) -> Option<Vec<Pos2>> {
    let st = crate::geom::stub_len(a, ad, b, bd).min(STUB);
    let a1 = a + DIRS[dir_idx(ad)] * st;
    let b1 = b + DIRS[dir_idx(bd)] * st;
    let window = Rect::from_two_pos(a1, b1).expand(180.0);
    let obs: Vec<Rect> = obstacles
        .iter()
        .map(|r| r.expand(MARGIN))
        .filter(|r| r.intersects(window) && !strictly_inside(r, a1) && !strictly_inside(r, b1))
        .collect();
    if obs.is_empty() { return None; }
    // widen once: anything inside the area the detour may use also counts
    let reach = obs.iter().fold(window, |acc, r| acc.union(*r)).expand(24.0);
    let obs: Vec<Rect> = obstacles
        .iter()
        .map(|r| r.expand(MARGIN))
        .filter(|r| r.intersects(reach) && !strictly_inside(r, a1) && !strictly_inside(r, b1))
        .collect();

    let mut h = std::collections::hash_map::DefaultHasher::new();
    for v in [a.x, a.y, ad.x, ad.y, b.x, b.y, bd.x, bd.y] { v.to_bits().hash(&mut h); }
    for r in &obs { for v in [r.min.x, r.min.y, r.max.x, r.max.y] { v.to_bits().hash(&mut h); } }
    let key = h.finish();
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) { return hit; }
    let res = solve(a, a1, ad, b, b1, bd, &obs, window);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 4000 { c.clear(); }
        c.insert(key, res.clone());
    });
    res
}

fn coords(mut v: Vec<f32>) -> Vec<f32> {
    v.sort_by(|a, b| a.total_cmp(b));
    v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    v
}

fn solve(a: Pos2, a1: Pos2, ad: Vec2, b: Pos2, b1: Pos2, bd: Vec2, obs: &[Rect], window: Rect) -> Option<Vec<Pos2>> {
    let bounds = obs.iter().fold(window, |acc, r| acc.union(*r)).expand(24.0);
    let mut xs = vec![a1.x, b1.x, bounds.min.x, bounds.max.x, (a1.x + b1.x) / 2.0];
    let mut ys = vec![a1.y, b1.y, bounds.min.y, bounds.max.y, (a1.y + b1.y) / 2.0];
    for r in obs { xs.extend([r.min.x, r.max.x]); ys.extend([r.min.y, r.max.y]); }
    let xs = coords(xs);
    let ys = coords(ys);
    let (nx, ny) = (xs.len(), ys.len());
    let find = |v: &[f32], t: f32| v.iter().position(|x| (x - t).abs() < 0.5);
    let (si, sj) = (find(&xs, a1.x)?, find(&ys, a1.y)?);
    let (gi, gj) = (find(&xs, b1.x)?, find(&ys, b1.y)?);
    let blocked = |p: Pos2| obs.iter().any(|r| strictly_inside(r, p));
    let node_ok: Vec<bool> = (0..nx * ny).map(|k| !blocked(pos2(xs[k % nx], ys[k / nx]))).collect();
    let state = |i: usize, j: usize, d: usize| (j * nx + i) * 4 + d;
    let mut dist = vec![f32::INFINITY; nx * ny * 4];
    let mut prev = vec![usize::MAX; nx * ny * 4];
    let mut heap = BinaryHeap::new();
    let d0 = dir_idx(ad);
    let s0 = state(si, sj, d0);
    dist[s0] = 0.0;
    heap.push(Reverse((0u32, s0)));
    let goal_dir = dir_idx(-DIRS[dir_idx(bd)]);
    let mut best: Option<(f32, usize)> = None;
    while let Some(Reverse((cbits, s))) = heap.pop() {
        let c = f32::from_bits(cbits);
        if c > dist[s] { continue; }
        let d = s % 4;
        let k = s / 4;
        let (i, j) = (k % nx, k / nx);
        if (i, j) == (gi, gj) {
            let total = c + if d == goal_dir { 0.0 } else { BEND };
            if best.map_or(true, |(bc, _)| total < bc) { best = Some((total, s)); }
            continue;
        }
        if let Some((bc, _)) = best { if c > bc { break; } }
        for nd in 0..4 {
            if nd == (d + 2) % 4 { continue; } // no U-turns
            let (ni, nj) = match nd { 0 => (i as i64 + 1, j as i64), 1 => (i as i64, j as i64 + 1), 2 => (i as i64 - 1, j as i64), _ => (i as i64, j as i64 - 1) };
            if ni < 0 || nj < 0 || ni >= nx as i64 || nj >= ny as i64 { continue; }
            let (ni, nj) = (ni as usize, nj as usize);
            if !node_ok[nj * nx + ni] { continue; }
            let p = pos2(xs[i], ys[j]);
            let q = pos2(xs[ni], ys[nj]);
            if blocked(pos2((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)) { continue; }
            let nc = c + (q - p).length() + if nd == d { 0.0 } else { BEND };
            let ns = state(ni, nj, nd);
            if nc < dist[ns] { dist[ns] = nc; prev[ns] = s; heap.push(Reverse((nc.to_bits(), ns))); }
        }
    }
    let (_, mut s) = best?;
    let mut pts = vec![];
    loop {
        let k = s / 4;
        pts.push(pos2(xs[k % nx], ys[k / nx]));
        if s == s0 || prev[s] == usize::MAX { break; }
        s = prev[s];
    }
    pts.reverse();
    let mut out = vec![a];
    out.extend(pts);
    out.push(b);
    Some(simplify(out))
}

/// Drop repeated points and any point in the middle of a straight run. That includes a run that
/// doubles back over itself (A→B→C with C behind B), which would otherwise draw as a spike and
/// leave a leg nobody can grab: it becomes the single leg A→C.
fn simplify_once(pts: Vec<Pos2>) -> Vec<Pos2> {
    let mut out: Vec<Pos2> = Vec::with_capacity(pts.len());
    for p in pts {
        if let Some(l) = out.last() { if (p - *l).length() < 0.01 { continue; } }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let cross = (b - a).x * (p - b).y - (b - a).y * (p - b).x;
            if cross.abs() < 0.01 * (b - a).length().max(1.0) { out.pop(); } else { break; }
        }
        if let Some(l) = out.last() { if (p - *l).length() < 0.01 { continue; } }
        out.push(p);
    }
    out
}

pub fn simplify(pts: Vec<Pos2>) -> Vec<Pos2> {
    let mut v = simplify_once(pts);
    // remove jogs of a few pixels between two parallel runs: shift an inner run onto the other
    for _ in 0..8 {
        let n = v.len();
        let mut changed = false;
        for i in 1..n.saturating_sub(2) {
            let d = v[i + 1] - v[i];
            if d.length() >= 3.0 { continue; }
            let horizontal = d.x.abs() > d.y.abs();
            if i + 2 < n - 1 {
                // move the run after the jog (p[i+1]..p[i+2]) onto the run before it
                if horizontal { v[i + 1].x = v[i].x; v[i + 2].x = v[i].x; } else { v[i + 1].y = v[i].y; v[i + 2].y = v[i].y; }
            } else if i >= 2 {
                // last inner run: move the run before the jog instead
                if horizontal { v[i].x = v[i + 1].x; v[i - 1].x = v[i + 1].x; } else { v[i].y = v[i + 1].y; v[i - 1].y = v[i + 1].y; }
            } else { continue; }
            changed = true;
            break;
        }
        if !changed { break; }
        v = simplify_once(v);
    }
    v
}

fn seg_intersect(p: Pos2, q: Pos2, u: Pos2, v: Pos2) -> Option<(f32, f32)> {
    let r = q - p;
    let s = v - u;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-6 { return None; }
    let w = u - p;
    let t = (w.x * s.y - w.y * s.x) / den;
    let k = (w.x * r.y - w.y * r.x) / den;
    if t > 0.0 && t < 1.0 && k > 0.0 && k < 1.0 { Some((t, k)) } else { None }
}

/// Insert small semicircular hops where connector `i` crosses other connectors.
/// The more-horizontal segment hops; ties go to the connector drawn later (Visio's default).
pub fn with_jumps(routes: &[Vec<Pos2>], i: usize, r: f32) -> Vec<Pos2> {
    let me = &routes[i];
    if me.len() < 2 { return me.clone(); }
    let mut out = vec![me[0]];
    for w in me.windows(2) {
        let (p, q) = (w[0], w[1]);
        let len = (q - p).length();
        if len < r * 4.0 { out.push(q); continue; }
        let dir = (q - p) / len;
        let hs = dir.x.abs();
        let mut ts: Vec<f32> = vec![];
        for (j, other) in routes.iter().enumerate() {
            if j == i { continue; }
            for w2 in other.windows(2) {
                let (u, v) = (w2[0], w2[1]);
                let l2 = (v - u).length();
                if l2 < 1e-3 { continue; }
                let hu = ((v - u) / l2).x.abs();
                let jumps = if (hs - hu).abs() > 0.1 { hs > hu } else { i > j };
                if !jumps { continue; }
                if let Some((t, _)) = seg_intersect(p, q, u, v) {
                    let d = t * len;
                    if d > r * 1.5 && d < len - r * 1.5 { ts.push(d); }
                }
            }
        }
        ts.sort_by(|a, b| a.total_cmp(b));
        let mut n = vec2(-dir.y, dir.x);
        if n.y > 0.01 || (n.y.abs() <= 0.01 && n.x > 0.0) { n = -n; }
        let mut last = -1e9;
        for d in ts {
            if d - last < r * 2.5 { continue; }
            last = d;
            let c = p + dir * d;
            for k in 0..=10 {
                let phi = std::f32::consts::PI * k as f32 / 10.0;
                out.push(c - dir * (r * phi.cos()) + n * (r * phi.sin()));
            }
        }
        out.push(q);
    }
    out
}
