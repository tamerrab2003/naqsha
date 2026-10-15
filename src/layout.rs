//! Auto arrange: layered (Sugiyama-style) layout with crossing reduction, grid layout,
//! container-aware nesting, and swimlane flows.
use crate::geom;
use crate::library;
use crate::model::*;
use egui::{pos2, vec2, Pos2, Rect, Vec2};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { TopDown, LeftRight, Grid }

/// A box to place. `w`/`h` include label space; `cur` is its current centre (used for ordering).
#[derive(Clone, Copy, Debug)]
struct NB { w: f32, h: f32, cur: Pos2 }

const RANK_GAP_TB: f32 = 70.0;
const RANK_GAP_LR: f32 = 90.0;
const NODE_GAP: f32 = 46.0;
const COMP_GAP: f32 = 90.0;
const PAD: f32 = 24.0;

fn label_extra(s: &Shape) -> f32 { if (library::is_icon(&s.kind) || s.kind == "image") && !s.text.is_empty() { 26.0 } else { 0.0 } }

// --------------------------------------------------------------------- core layered layout

/// Break cycles by reversing back edges found in a DFS (visiting nodes left-to-right/top-to-bottom).
fn acyclic(n: usize, edges: &[(usize, usize)], order: &[usize]) -> Vec<(usize, usize)> {
    let mut adj = vec![vec![]; n];
    for &(a, b) in edges { if a != b { adj[a].push(b); } }
    let mut state = vec![0u8; n]; // 0 new, 1 on stack, 2 done
    let mut back: HashSet<(usize, usize)> = HashSet::new();
    for &s in order {
        if state[s] != 0 { continue; }
        // iterative DFS
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        state[s] = 1;
        while let Some(&mut (v, ref mut i)) = stack.last_mut() {
            if *i < adj[v].len() {
                let w = adj[v][*i];
                *i += 1;
                match state[w] { 0 => { state[w] = 1; stack.push((w, 0)); } 1 => { back.insert((v, w)); } _ => {} }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    edges.iter().filter(|(a, b)| a != b).map(|&(a, b)| if back.contains(&(a, b)) { (b, a) } else { (a, b) }).collect()
}

/// Longest-path ranks on a DAG (sources at rank 0), then pull sinks-only nodes down next to their parents.
fn ranks(n: usize, dag: &[(usize, usize)]) -> Vec<usize> {
    let mut indeg = vec![0usize; n];
    let mut out = vec![vec![]; n];
    for &(a, b) in dag { indeg[b] += 1; out[a].push(b); }
    let mut rank = vec![0usize; n];
    let mut queue: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let mut i = 0;
    while i < queue.len() {
        let v = queue[i];
        i += 1;
        for &w in &out[v] {
            rank[w] = rank[w].max(rank[v] + 1);
            indeg[w] -= 1;
            if indeg[w] == 0 { queue.push(w); }
        }
    }
    // tighten: a source whose children all sit far below moves down to just above the nearest child
    let mut inn = vec![vec![]; n];
    for &(a, b) in dag { inn[b].push(a); }
    for v in 0..n {
        if inn[v].is_empty() && !out[v].is_empty() {
            let m = out[v].iter().map(|&w| rank[w]).min().unwrap();
            if m > 0 { rank[v] = m - 1; }
        }
    }
    rank
}

/// Layered layout of one connected component. Returns top-left positions (relative) and the extent.
fn layered(nodes: &[NB], edges: &[(usize, usize)], lr: bool) -> (Vec<Pos2>, Vec2) {
    let n = nodes.len();
    // order nodes by current position along the flow, so the existing reading order is respected
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (pa, pb) = (nodes[a].cur, nodes[b].cur);
        let (ka, kb) = if lr { (pa.x, pb.x) } else { (pa.y, pb.y) };
        ka.total_cmp(&kb)
    });
    // walk from true starting points first, so loops break at their "return" edge
    let mut indeg = vec![0usize; n];
    for &(_, b) in edges { indeg[b] += 1; }
    let mut order: Vec<usize> = order.iter().copied().filter(|&v| indeg[v] == 0).chain(order.iter().copied().filter(|&v| indeg[v] != 0)).collect();
    order.dedup();
    let dag = acyclic(n, edges, &order);
    let rank = ranks(n, &dag);
    let nr = rank.iter().copied().max().unwrap_or(0) + 1;

    // thickness along the flow axis / breadth across it
    let along = |b: &NB| if lr { b.w } else { b.h };
    let across = |b: &NB| if lr { b.h } else { b.w };

    // dummy nodes for edges spanning several ranks
    let mut sizes: Vec<f32> = nodes.iter().map(|b| across(b)).collect();
    let mut node_rank = rank.clone();
    let mut adj_edges: Vec<(usize, usize)> = vec![];
    for &(a, b) in &dag {
        let (ra, rb) = (node_rank[a], node_rank[b]);
        if rb <= ra { continue; }
        let mut prev = a;
        for r in ra + 1..rb {
            let d = sizes.len();
            sizes.push(14.0);
            node_rank.push(r);
            adj_edges.push((prev, d));
            prev = d;
        }
        adj_edges.push((prev, b));
    }
    let total = sizes.len();
    let mut layers: Vec<Vec<usize>> = vec![vec![]; nr];
    // initial order: by current cross-axis position (dummies after their source)
    let cross_cur = |v: usize| if v < n { if lr { nodes[v].cur.y } else { nodes[v].cur.x } } else { f32::MAX };
    for v in 0..total { layers[node_rank[v]].push(v); }
    for l in layers.iter_mut() { l.sort_by(|&a, &b| cross_cur(a).total_cmp(&cross_cur(b))); }
    let mut up = vec![vec![]; total];
    let mut down = vec![vec![]; total];
    for &(a, b) in &adj_edges { down[a].push(b); up[b].push(a); }
    // dummies inherit a sensible initial slot: the position of their source
    for r in 1..nr {
        let pos: HashMap<usize, usize> = layers[r - 1].iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let key = |v: usize| -> f32 { if v >= n { up[v].first().and_then(|u| pos.get(u)).map_or(0.0, |&p| p as f32) } else { -1.0 } };
        if layers[r].iter().any(|&v| v >= n) {
            let mut real: Vec<usize> = layers[r].iter().copied().filter(|&v| v < n).collect();
            let mut dums: Vec<usize> = layers[r].iter().copied().filter(|&v| v >= n).collect();
            dums.sort_by(|&a, &b| key(a).total_cmp(&key(b)));
            real.extend(dums);
            layers[r] = real;
        }
    }

    // crossing reduction: barycentre sweeps, keep the best ordering seen
    let crossings = |layers: &Vec<Vec<usize>>| -> usize {
        let mut c = 0;
        for r in 0..nr.saturating_sub(1) {
            let pos: HashMap<usize, usize> = layers[r + 1].iter().enumerate().map(|(i, &v)| (v, i)).collect();
            let mut es: Vec<(usize, usize)> = vec![];
            for (i, &v) in layers[r].iter().enumerate() { for w in &down[v] { if let Some(&j) = pos.get(w) { es.push((i, j)); } } }
            for x in 0..es.len() { for y in x + 1..es.len() { let (a, b) = (es[x], es[y]); if (a.0 < b.0 && a.1 > b.1) || (a.0 > b.0 && a.1 < b.1) { c += 1; } } }
        }
        c
    };
    let mut best = layers.clone();
    let mut best_c = crossings(&layers);
    for it in 0..12 {
        let downward = it % 2 == 0;
        let range: Vec<usize> = if downward { (1..nr).collect() } else { (0..nr.saturating_sub(1)).rev().collect() };
        for r in range {
            let fixed = if downward { r - 1 } else { r + 1 };
            let pos: HashMap<usize, f32> = layers[fixed].iter().enumerate().map(|(i, &v)| (v, i as f32)).collect();
            let cur: HashMap<usize, f32> = layers[r].iter().enumerate().map(|(i, &v)| (v, i as f32)).collect();
            let bary = |v: usize| -> f32 {
                let nb = if downward { &up[v] } else { &down[v] };
                let ps: Vec<f32> = nb.iter().filter_map(|u| pos.get(u)).copied().collect();
                if ps.is_empty() { cur[&v] } else { ps.iter().sum::<f32>() / ps.len() as f32 }
            };
            layers[r].sort_by(|&a, &b| bary(a).total_cmp(&bary(b)).then(cur[&a].total_cmp(&cur[&b])));
        }
        let c = crossings(&layers);
        if c < best_c { best_c = c; best = layers.clone(); }
        if best_c == 0 { break; }
    }
    let layers = best;

    // cross-axis coordinates: pull each node towards its neighbours, keeping order and spacing
    let mut x = vec![0.0f32; total];
    for l in &layers { let mut acc = 0.0; for &v in l { x[v] = acc + sizes[v] / 2.0; acc += sizes[v] + NODE_GAP; } }
    // priority placement: real, well-connected nodes claim their ideal spot first;
    // dummies (edge bends) and loosely connected nodes make room
    let degree: Vec<usize> = (0..total).map(|v| up[v].len() + down[v].len()).collect();
    let place = |x: &mut Vec<f32>, l: &Vec<usize>, desired: &HashMap<usize, f32>| {
        let m = l.len();
        if m == 0 { return; }
        let sep = |a: usize, b: usize| (sizes[l[a]] + sizes[l[b]]) / 2.0 + NODE_GAP;
        // cumulative separation from slot 0, so the gap between slots i<j is off[j]-off[i]
        let mut off = vec![0.0f32; m];
        for i in 1..m { off[i] = off[i - 1] + sep(i - 1, i); }
        let mut pos: Vec<f32> = l.iter().map(|v| x[*v]).collect();
        let mut fixed = vec![false; m];
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(if l[i] < n { 1000 + degree[l[i]] } else { degree[l[i]] }));
        for i in order {
            let target = desired.get(&l[i]).copied().unwrap_or(pos[i]);
            let lo = (0..i).filter(|&j| fixed[j]).map(|j| pos[j] + off[i] - off[j]).fold(f32::MIN, f32::max);
            let hi = (i + 1..m).filter(|&j| fixed[j]).map(|j| pos[j] - (off[j] - off[i])).fold(f32::MAX, f32::min);
            pos[i] = if lo <= hi { target.clamp(lo, hi) } else { lo };
            fixed[i] = true;
        }
        for i in 1..m { pos[i] = pos[i].max(pos[i - 1] + sep(i - 1, i)); }
        for (i, &v) in l.iter().enumerate() { x[v] = pos[i]; }
    };
    let median = |xs: &mut Vec<f32>| -> f32 {
        xs.sort_by(|a, b| a.total_cmp(b));
        let k = xs.len();
        if k % 2 == 1 { xs[k / 2] } else { (xs[k / 2 - 1] + xs[k / 2]) / 2.0 }
    };
    for it in 0..12 {
        let downward = it % 2 == 0;
        let range: Vec<usize> = if downward { (1..nr).collect() } else { (0..nr.saturating_sub(1)).rev().collect() };
        for r in range {
            let mut desired = HashMap::new();
            for &v in &layers[r] {
                // last two sweeps look both ways, which balances nodes between parents and children
                let nb: Vec<usize> = if it >= 10 { up[v].iter().chain(down[v].iter()).copied().collect() } else if downward { up[v].clone() } else { down[v].clone() };
                // align with real neighbours; long-edge bend points only count when there are none
                let real: Vec<usize> = nb.iter().copied().filter(|&u| u < n).collect();
                let nb = if real.is_empty() || v >= n { nb } else { real };
                if !nb.is_empty() { let mut xs: Vec<f32> = nb.iter().map(|u| x[*u]).collect(); desired.insert(v, median(&mut xs)); }
            }
            let l = layers[r].clone();
            place(&mut x, &l, &desired);
        }
    }
    let min_x = (0..n).map(|v| x[v] - sizes[v] / 2.0).fold(f32::MAX, f32::min);
    let max_x = (0..n).map(|v| x[v] + sizes[v] / 2.0).fold(f32::MIN, f32::max);

    // flow-axis coordinates: each rank as thick as its thickest node
    let gap = if lr { RANK_GAP_LR } else { RANK_GAP_TB };
    let mut thick = vec![0.0f32; nr];
    for v in 0..n { thick[rank[v]] = thick[rank[v]].max(along(&nodes[v])); }
    let mut start = vec![0.0f32; nr];
    for r in 1..nr { start[r] = start[r - 1] + thick[r - 1] + gap; }
    let flow_len = start[nr - 1] + thick[nr - 1];

    let pos: Vec<Pos2> = (0..n)
        .map(|v| {
            let c = x[v] - min_x;
            let a0 = start[rank[v]] + (thick[rank[v]] - along(&nodes[v])) / 2.0;
            let b = &nodes[v];
            if lr { pos2(a0, c - b.h / 2.0) } else { pos2(c - b.w / 2.0, a0) }
        })
        .collect();
    let breadth = max_x - min_x;
    (pos, if lr { vec2(flow_len, breadth) } else { vec2(breadth, flow_len) })
}

/// Grid of boxes in reading order of their current positions.
fn grid(nodes: &[NB]) -> (Vec<Pos2>, Vec2) {
    let n = nodes.len();
    if n == 0 { return (vec![], Vec2::ZERO); }
    let mut order: Vec<usize> = (0..n).collect();
    let band = nodes.iter().map(|b| b.h).fold(0.0, f32::max).max(20.0);
    order.sort_by(|&a, &b| {
        let (pa, pb) = (nodes[a].cur, nodes[b].cur);
        ((pa.y / band).floor()).total_cmp(&(pb.y / band).floor()).then(pa.x.total_cmp(&pb.x))
    });
    let cols = ((n as f32 * 1.6).sqrt().ceil() as usize).max(1);
    let cw = nodes.iter().map(|b| b.w).fold(0.0, f32::max) + NODE_GAP;
    let ch = nodes.iter().map(|b| b.h).fold(0.0, f32::max) + NODE_GAP;
    let mut pos = vec![Pos2::ZERO; n];
    for (k, &i) in order.iter().enumerate() {
        let (c, r) = (k % cols, k / cols);
        pos[i] = pos2(c as f32 * cw + (cw - NODE_GAP - nodes[i].w) / 2.0, r as f32 * ch + (ch - NODE_GAP - nodes[i].h) / 2.0);
    }
    let rows = n.div_ceil(cols);
    (pos, vec2(cols.min(n) as f32 * cw - NODE_GAP, rows as f32 * ch - NODE_GAP))
}

/// Arrange a set of boxes: connected components laid out separately and packed, loose boxes in a grid.
fn arrange(nodes: &[NB], edges: &[(usize, usize)], mode: Mode) -> (Vec<Pos2>, Vec2) {
    let n = nodes.len();
    if n == 0 { return (vec![], Vec2::ZERO); }
    if mode == Mode::Grid { return grid(nodes); }
    let lr = mode == Mode::LeftRight;
    // union-find components
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize { let mut r = x; while p[r] != r { r = p[r]; } let mut c = x; while p[c] != r { let nx = p[c]; p[c] = r; c = nx; } r }
    for &(a, b) in edges { let (ra, rb) = (find(&mut parent, a), find(&mut parent, b)); if ra != rb { parent[ra] = rb; } }
    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    let connected: HashSet<usize> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    for v in 0..n { if connected.contains(&v) { comps.entry(find(&mut parent, v)).or_default().push(v); } }
    let loose: Vec<usize> = (0..n).filter(|v| !connected.contains(v)).collect();
    let mut comps: Vec<Vec<usize>> = comps.into_values().collect();
    // biggest component first, then by current position
    comps.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| {
        let ka = a.iter().map(|&v| if lr { nodes[v].cur.y } else { nodes[v].cur.x }).fold(f32::MAX, f32::min);
        let kb = b.iter().map(|&v| if lr { nodes[v].cur.y } else { nodes[v].cur.x }).fold(f32::MAX, f32::min);
        ka.total_cmp(&kb)
    }));
    let mut pos = vec![Pos2::ZERO; n];
    let mut cursor = 0.0f32; // across the flow
    let mut extent = Vec2::ZERO;
    for comp in &comps {
        let idx: HashMap<usize, usize> = comp.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let sub: Vec<NB> = comp.iter().map(|&v| nodes[v]).collect();
        let se: Vec<(usize, usize)> = edges.iter().filter_map(|&(a, b)| Some((*idx.get(&a)?, *idx.get(&b)?))).collect();
        let (p, sz) = layered(&sub, &se, lr);
        for (i, &v) in comp.iter().enumerate() {
            pos[v] = if lr { p[i] + vec2(0.0, cursor) } else { p[i] + vec2(cursor, 0.0) };
        }
        if lr { cursor += sz.y + COMP_GAP; extent.x = extent.x.max(sz.x); extent.y = cursor - COMP_GAP; } else { cursor += sz.x + COMP_GAP; extent.y = extent.y.max(sz.y); extent.x = cursor - COMP_GAP; }
    }
    if !loose.is_empty() {
        let sub: Vec<NB> = loose.iter().map(|&v| nodes[v]).collect();
        let (p, sz) = grid(&sub);
        // below (top-down) or underneath (left-right) the connected part
        let off = if comps.is_empty() { Vec2::ZERO } else { vec2(0.0, extent.y + COMP_GAP) };
        for (i, &v) in loose.iter().enumerate() { pos[v] = p[i] + off; }
        extent = vec2(extent.x.max(sz.x), off.y + sz.y);
    }
    (pos, extent)
}

// --------------------------------------------------------------------- applying to a page

fn is_lane(k: &str) -> bool { matches!(k, "lane_h" | "lane_v") }

/// Space a container keeps clear for its header, around its contents.
fn container_pad(k: &str) -> (f32, f32, f32, f32) {
    // left, top, right, bottom
    match k {
        "lane_h" => (36.0 + PAD, PAD, PAD, PAD),
        "lane_v" => (PAD, 36.0 + PAD, PAD, PAD),
        "container" => (PAD, 34.0 + PAD, PAD, PAD),
        _ => (PAD, 36.0 + PAD, PAD, PAD),
    }
}

/// Everything that moves with a shape: itself and, for containers, all shapes inside.
fn subtree(id: Id, children: &HashMap<Id, Vec<Id>>, out: &mut Vec<Id>) {
    out.push(id);
    if let Some(cs) = children.get(&id) { for &c in cs { subtree(c, children, out); } }
}

fn translate(page: &mut Page, ids: &[Id], d: Vec2) {
    if d.length_sq() < 1e-6 { return; }
    for s in page.shapes.iter_mut().filter(|s| ids.contains(&s.id)) { s.x += d.x; s.y += d.y; }
}

/// Auto-arrange the page (or just the selected shapes). Returns a short summary.
pub fn auto_arrange(page: &mut Page, selection: &[Id], mode: Mode) -> Result<String, String> {
    let sel_shapes: Vec<Id> = page.shapes.iter().filter(|s| selection.contains(&s.id)).map(|s| s.id).collect();
    let use_sel = sel_shapes.len() >= 2;
    let movable = |s: &Shape| !s.locked && page.shape_editable(s) && s.kind != "phase";
    let universe: Vec<Id> = page.shapes.iter().filter(|s| movable(s) && (!use_sel || sel_shapes.contains(&s.id))).map(|s| s.id).collect();
    if universe.len() < 2 { return Err("Add a few shapes first — or select at least two to arrange".into()); }

    // containment: each shape's smallest enclosing container (only when arranging the whole page)
    let bbox: HashMap<Id, Rect> = page.shapes.iter().map(|s| (s.id, geom::bbox(s))).collect();
    let kind: HashMap<Id, String> = page.shapes.iter().map(|s| (s.id, s.kind.clone())).collect();
    let mut parent: HashMap<Id, Id> = HashMap::new();
    if !use_sel {
        let conts: Vec<Id> = universe.iter().copied().filter(|id| library::is_container(&kind[id])).collect();
        for &id in &universe {
            let b = bbox[&id];
            let best = conts.iter().copied().filter(|&c| c != id && bbox[&c].contains_rect(b) && bbox[&c].area() > b.area()).min_by(|a, b| bbox[a].area().total_cmp(&bbox[b].area()));
            if let Some(c) = best { parent.insert(id, c); }
        }
    }
    let mut children: HashMap<Id, Vec<Id>> = HashMap::new();
    for (&c, &p) in &parent { children.entry(p).or_default().push(c); }
    if use_sel {
        // selected containers carry their contents
        for &id in &universe {
            if library::is_container(&kind[&id]) {
                let b = bbox[&id];
                let inside: Vec<Id> = page.shapes.iter().filter(|s| s.id != id && b.contains_rect(bbox[&s.id]) && !sel_shapes.contains(&s.id)).map(|s| s.id).collect();
                children.insert(id, inside);
            }
        }
    }

    // edges between shapes (glued at both ends)
    let glued: Vec<(Id, Id)> = page.conns.iter().filter_map(|c| match (c.from, c.to) { (End::Glued { shape: a, .. }, End::Glued { shape: b, .. }) if a != b => Some((a, b)), _ => None }).collect();
    // lift an id to its ancestor that is a direct child of `level` (None = top level)
    let lift = |mut id: Id, level: Option<Id>| -> Option<Id> {
        loop {
            let p = parent.get(&id).copied();
            if p == level { return Some(id); }
            id = p?;
        }
    };
    let in_universe: HashSet<Id> = universe.iter().copied().collect();

    let mut moved_containers = 0;
    // 1) containers, innermost first: arrange their direct children and fit the container around them
    if !use_sel {
        let mut conts: Vec<Id> = children.keys().copied().filter(|c| !is_lane(&kind[c])).collect();
        conts.sort_by(|a, b| bbox[a].area().total_cmp(&bbox[b].area()));
        for c in conts {
            let kids: Vec<Id> = children[&c].clone();
            let nodes: Vec<NB> = kids.iter().map(|id| { let s = page.shape(*id).unwrap(); let b = geom::bbox(s); NB { w: b.width(), h: b.height() + label_extra(s), cur: b.center() } }).collect();
            let edges: Vec<(usize, usize)> = glued.iter().filter_map(|&(a, b)| {
                let (la, lb) = (lift(a, Some(c))?, lift(b, Some(c))?);
                if la == lb { return None; }
                Some((kids.iter().position(|&k| k == la)?, kids.iter().position(|&k| k == lb)?))
            }).collect();
            let (pos, ext) = arrange(&nodes, &edges, mode);
            let cb = geom::bbox(page.shape(c).unwrap());
            let (pl, pt, pr, pb) = container_pad(&kind[&c]);
            for (i, &k) in kids.iter().enumerate() {
                let b = geom::bbox(page.shape(k).unwrap());
                let target = pos2(cb.min.x + pl, cb.min.y + pt) + pos[i].to_vec2();
                let mut ids = vec![];
                subtree(k, &children, &mut ids);
                translate(page, &ids, target - b.min);
            }
            if let Some(s) = page.shape_mut(c) {
                s.w = (ext.x + pl + pr).max(160.0);
                s.h = (ext.y + pt + pb).max(100.0);
            }
            moved_containers += 1;
        }
    }

    // 2) swimlane stacks: columns follow the flow, every shape stays in its lane
    let mut lane_stacks: Vec<Vec<Id>> = vec![];
    if !use_sel {
        for horizontal in [true, false] {
            let k = if horizontal { "lane_h" } else { "lane_v" };
            let mut lanes: Vec<Id> = universe.iter().copied().filter(|id| kind[id] == k && !parent.contains_key(id)).collect();
            lanes.sort_by(|a, b| if horizontal { bbox[a].min.y.total_cmp(&bbox[b].min.y) } else { bbox[a].min.x.total_cmp(&bbox[b].min.x) });
            // stacks = lanes that share an edge and line up
            let mut stack: Vec<Id> = vec![];
            for id in lanes {
                if let Some(&last) = stack.last() {
                    let (pb, cb) = (bbox[&last], bbox[&id]);
                    let adjacent = if horizontal { (cb.min.y - pb.max.y).abs() < 6.0 && (cb.min.x - pb.min.x).abs() < 6.0 } else { (cb.min.x - pb.max.x).abs() < 6.0 && (cb.min.y - pb.min.y).abs() < 6.0 };
                    if !adjacent { lane_stacks.push(std::mem::take(&mut stack)); }
                }
                stack.push(id);
            }
            if !stack.is_empty() { lane_stacks.push(stack); }
        }
        for stack in &lane_stacks {
            let horizontal = kind[&stack[0]] == "lane_h";
            let members: Vec<(Id, Id)> = stack.iter().flat_map(|&l| children.get(&l).cloned().unwrap_or_default().into_iter().map(move |c| (c, l))).collect();
            if members.is_empty() { continue; }
            let ids: Vec<Id> = members.iter().map(|m| m.0).collect();
            let idx: HashMap<Id, usize> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
            let lane_of: HashMap<Id, Id> = members.iter().copied().collect();
            let edges: Vec<(usize, usize)> = glued.iter().filter_map(|&(a, b)| {
                let (la, lb) = (lift(a, lane_of.get(&a).copied().or(parent.get(&a).copied()))?, lift(b, lane_of.get(&b).copied().or(parent.get(&b).copied()))?);
                if la == lb { return None; }
                Some((*idx.get(&la)?, *idx.get(&lb)?))
            }).collect();
            let order: Vec<usize> = {
                let mut o: Vec<usize> = (0..ids.len()).collect();
                o.sort_by(|&a, &b| { let (ba, bb) = (bbox[&ids[a]], bbox[&ids[b]]); if horizontal { ba.min.x.total_cmp(&bb.min.x) } else { ba.min.y.total_cmp(&bb.min.y) } });
                o
            };
            let mut indeg = vec![0usize; ids.len()];
            for &(_, b) in &edges { indeg[b] += 1; }
            let order: Vec<usize> = order.iter().copied().filter(|&v| indeg[v] == 0).chain(order.iter().copied().filter(|&v| indeg[v] != 0)).collect();
            let rank = ranks(ids.len(), &acyclic(ids.len(), &edges, &order));
            let ncol = rank.iter().copied().max().unwrap_or(0) + 1;
            let sizes: HashMap<Id, Vec2> = ids.iter().map(|&id| { let s = page.shape(id).unwrap(); let b = geom::bbox(s); (id, vec2(b.width(), b.height() + label_extra(s))) }).collect();
            let size = |id: Id| sizes[&id];
            // column thickness along the lane
            let mut colw = vec![0.0f32; ncol];
            for (i, &id) in ids.iter().enumerate() { let sz = size(id); colw[rank[i]] = colw[rank[i]].max(if horizontal { sz.x } else { sz.y }); }
            let gap = RANK_GAP_LR;
            let (pl, pt, _, _) = container_pad(&kind[&stack[0]]);
            let lead = if horizontal { pl } else { pt };
            let mut col_start = vec![lead; ncol];
            for c in 1..ncol { col_start[c] = col_start[c - 1] + colw[c - 1] + gap; }
            let needed_len = col_start[ncol - 1] + colw[ncol - 1] + PAD;
            // place, lane by lane; shapes sharing a lane+column stack across the lane
            let mut lane_depth: HashMap<Id, f32> = HashMap::new();
            let first = bbox[&stack[0]];
            for &lane in stack {
                let lb = page.shape(lane).map(geom::bbox).unwrap();
                let mut cells: HashMap<usize, Vec<Id>> = HashMap::new();
                for (i, &id) in ids.iter().enumerate() { if lane_of[&id] == lane { cells.entry(rank[i]).or_default().push(id); } }
                let mut depth_needed = 0.0f32;
                for (col, list) in cells.iter_mut() {
                    list.sort_by(|a, b| if horizontal { bbox[a].min.y.total_cmp(&bbox[b].min.y) } else { bbox[a].min.x.total_cmp(&bbox[b].min.x) });
                    let total: f32 = list.iter().map(|&id| if horizontal { size(id).y } else { size(id).x }).sum::<f32>() + NODE_GAP * (list.len() as f32 - 1.0);
                    depth_needed = depth_needed.max(total + 2.0 * PAD + if horizontal { 0.0 } else { 36.0 });
                    let lane_depth_now = if horizontal { lb.height() } else { lb.width() };
                    let mut c = (lane_depth_now.max(total + 2.0 * PAD) - total) / 2.0;
                    for &id in list.iter() {
                        let sz = size(id);
                        let cw = colw[*col];
                        let (x, y) = if horizontal {
                            (first.min.x + col_start[*col] + (cw - sz.x) / 2.0, lb.min.y + c)
                        } else {
                            (lb.min.x + c, first.min.y + col_start[*col] + (cw - sz.y) / 2.0)
                        };
                        let b = bbox[&id];
                        let mut sub = vec![];
                        subtree(id, &children, &mut sub);
                        translate(page, &sub, pos2(x, y) - b.min);
                        c += (if horizontal { sz.y } else { sz.x }) + NODE_GAP;
                    }
                }
                lane_depth.insert(lane, depth_needed);
            }
            // grow lanes that are too shallow (shifting later lanes) and give every lane the needed length
            let mut shift = 0.0f32;
            for &lane in stack {
                let need = lane_depth[&lane];
                let mut sub = vec![];
                subtree(lane, &children, &mut sub);
                if horizontal { translate(page, &sub, vec2(0.0, shift)); } else { translate(page, &sub, vec2(shift, 0.0)); }
                if let Some(s) = page.shape_mut(lane) {
                    if horizontal {
                        s.w = s.w.max(needed_len);
                        if need > s.h { shift += need - s.h; s.h = need; }
                    } else {
                        s.h = s.h.max(needed_len);
                        if need > s.w { shift += need - s.w; s.w = need; }
                    }
                }
            }
            // equal lane lengths across the stack
            let len = stack.iter().filter_map(|l| page.shape(*l)).map(|s| if horizontal { s.w } else { s.h }).fold(0.0, f32::max);
            for &l in stack { if let Some(s) = page.shape_mut(l) { if horizontal { s.w = len } else { s.h = len } } }
            // re-centre shapes vertically in lanes that grew
            for &lane in stack {
                let lb = page.shape(lane).map(geom::bbox).unwrap();
                for &id in ids.iter() {
                    if lane_of[&id] != lane { continue; }
                    let b = page.shape(id).map(geom::bbox).unwrap();
                    let sz = size(id);
                    let mut sub = vec![];
                    subtree(id, &children, &mut sub);
                    // only shapes alone in their cell get centred
                    let alone = ids.iter().enumerate().filter(|(j, o)| lane_of[o] == lane && rank[*j] == rank[idx[&id]]).count() == 1;
                    if !alone { continue; }
                    if horizontal { translate(page, &sub, vec2(0.0, lb.center().y - (b.min.y + sz.y / 2.0))); } else { translate(page, &sub, vec2(lb.center().x - (b.min.x + sz.x / 2.0), 0.0)); }
                }
            }
        }
    }

    // 3) top level: free shapes, containers and lane stacks (each stack moves as one block)
    let top_level: Vec<Id> = universe.iter().copied().filter(|id| !parent.contains_key(id) && !lane_stacks.iter().flatten().any(|l| l == id)).collect();
    // blocks: (ids moved together, bbox)
    let current_box = |page: &Page, ids: &[Id]| -> Rect { ids.iter().filter_map(|id| page.shape(*id)).map(geom::bbox).reduce(|a, b| a.union(b)).unwrap_or(Rect::NOTHING) };
    let mut blocks: Vec<(Vec<Id>, Vec<Id>)> = vec![]; // (representative ids for edges, all ids moved)
    for &id in &top_level {
        let mut all = vec![];
        subtree(id, &children, &mut all);
        blocks.push((vec![id], all));
    }
    for stack in &lane_stacks {
        let mut all = vec![];
        for &l in stack { subtree(l, &children, &mut all); }
        blocks.push((stack.clone(), all));
    }
    let origin = blocks.iter().map(|(_, all)| current_box(page, all)).reduce(|a, b| a.union(b)).map(|r| r.min).unwrap_or(pos2(40.0, 40.0));
    let block_of = |id: Id| -> Option<usize> {
        // the block whose representative is this shape's top-level ancestor
        let mut top = id;
        while let Some(p) = parent.get(&top) { top = *p; }
        blocks.iter().position(|(rep, _)| rep.contains(&top))
    };
    let nodes: Vec<NB> = blocks.iter().map(|(rep, all)| {
        let b = current_box(page, all);
        let extra = if rep.len() == 1 { page.shape(rep[0]).map(label_extra).unwrap_or(0.0) } else { 0.0 };
        NB { w: b.width(), h: b.height() + extra, cur: b.center() }
    }).collect();
    let mut edges: Vec<(usize, usize)> = vec![];
    for &(a, b) in &glued {
        if !in_universe.contains(&a) || !in_universe.contains(&b) { continue; }
        if let (Some(x), Some(y)) = (block_of(a), block_of(b)) { if x != y { edges.push((x, y)); } }
    }
    let (pos, _) = arrange(&nodes, &edges, mode);
    for (i, (_, all)) in blocks.iter().enumerate() {
        let b = current_box(page, all);
        translate(page, all, (origin + pos[i].to_vec2()) - b.min);
    }

    // 4) connectors between arranged shapes: automatic routes and dynamic glue
    let mut reset = 0;
    for c in page.conns.iter_mut() {
        let touches = |e: &End| matches!(e, End::Glued { shape, .. } if in_universe.contains(shape));
        if touches(&c.from) || touches(&c.to) {
            c.bends = None;
            if let End::Glued { shape, .. } = c.from { c.from = End::Glued { shape, port: -1 }; }
            if let End::Glued { shape, .. } = c.to { c.to = End::Glued { shape, port: -1 }; }
            reset += 1;
        }
    }
    page.avoid_shapes = true;
    // make sure everything is on the page
    let content = page.shapes.iter().map(geom::bbox).reduce(|a, b| a.union(b)).unwrap_or(Rect::NOTHING);
    if content.is_finite() {
        page.width = page.width.max((content.max.x + 40.0).ceil());
        page.height = page.height.max((content.max.y + 40.0).ceil());
    }
    let what = match mode { Mode::TopDown => "top to bottom", Mode::LeftRight => "left to right", Mode::Grid => "in a grid" };
    let mut msg = format!("Arranged {} shapes {what}", universe.len());
    if moved_containers > 0 { msg.push_str(&format!(" · {moved_containers} container(s) resized")); }
    if !lane_stacks.is_empty() { msg.push_str(" · swimlanes kept"); }
    if reset > 0 { msg.push_str(&format!(" · {reset} connectors re-routed")); }
    Ok(msg)
}

/// Reset connector routes (selected connectors, or all on the page) and turn on obstacle avoidance.
pub fn tidy_connectors(page: &mut Page, selection: &[Id]) -> usize {
    let any = page.conns.iter().any(|c| selection.contains(&c.id));
    let mut n = 0;
    for c in page.conns.iter_mut() {
        if any && !selection.contains(&c.id) { continue; }
        c.bends = None;
        if let End::Glued { shape, .. } = c.from { c.from = End::Glued { shape, port: -1 }; }
        if let End::Glued { shape, .. } = c.to { c.to = End::Glued { shape, port: -1 }; }
        n += 1;
    }
    page.avoid_shapes = true;
    n
}
