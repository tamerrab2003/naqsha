//! Application state and document commands.
use crate::geom;
use crate::library;
use crate::model::*;
use crate::theme::{self, Pal};
use egui::{pos2, vec2, Pos2, Rect, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool { Pointer, Connector, Text, Pan }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode { System, Light, Dark }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab { File, Home, Insert, Design, Arrange, View, Pro }

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub grid: bool,
    pub rulers: bool,
    pub snap: bool,
    pub guides: bool,
    pub show_left: bool,
    pub show_right: bool,
    pub grid_size: f32,
    pub recent: Vec<String>,
    pub routing: Routing,
    /// Folders of SVG icons shown as extra shape groups.
    pub stencil_dirs: Vec<String>,
    /// Last auto-arrange style: "tb", "lr" or "grid".
    pub layout: String,
    /// Naqsha Pro licence and trial (used by official builds; see src/pro_stub.rs).
    pub pro: ProLicense,
}

/// A Naqsha Pro licence as last confirmed by the store, and the trial. Plain data: the logic that
/// reads it lives in the Pro module, which only official builds include.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProLicense {
    /// The licence key the customer was emailed.
    pub key: String,
    /// This device's activation (one seat).
    pub instance: String,
    pub customer: String,
    pub email: String,
    /// "Subscription" or "Lifetime" and the plan's name in the store.
    pub plan: String,
    pub lifetime: bool,
    /// The store's word for the key: active, expired, disabled…
    pub status: String,
    /// Unix seconds: when the licence was bought, and the last time the store confirmed it.
    pub created: u64,
    pub checked: u64,
    /// Unix seconds when the free trial started (0: not started).
    pub trial_start: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { mode: Mode::System, grid: true, rulers: true, snap: true, guides: true, show_left: true, show_right: true, grid_size: 20.0, recent: vec![], routing: Routing::Orthogonal, stencil_dirs: vec![], layout: "tb".into(), pro: ProLicense::default() }
    }
}

pub enum Drag {
    None,
    Pan { start: Pos2, pan0: Vec2 },
    Move { start: Pos2, orig: Vec<(Id, f32, f32)>, free: Vec<(Id, End, End, Option<Vec<[f32; 2]>>)>, before: Document, dup_done: bool },
    /// Dragging a connector segment: `pts` is the route (with duplicated end points when an end
    /// segment is dragged), `j` the index of the dragged segment (or point, for straight lines).
    Seg { id: Id, start: Pos2, pts: Vec<Pos2>, j: usize, horizontal: bool, free: bool, before: Document, ends: (End, End) },
    Resize { id: Id, hx: i32, hy: i32, orig: Shape, before: Document },
    MResize { hx: i32, hy: i32, bbox: Rect, orig: Vec<Shape>, before: Document },
    Rotate { id: Id, before: Document },
    Marquee { start: Pos2, cur: Pos2, additive: bool },
    NewConn { id: Id, before: Document },
    ConnEnd { id: Id, to: bool, before: Document },
}

pub struct Edit {
    pub id: Id,
    pub conn: bool,
    pub buf: String,
    pub before: Document,
    pub focus: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pending { Close, OpenPath }

pub enum Modal { Unsaved(Pending), About, Shortcuts, NewDiagram, Pro }

/// A request for the window manager to open another window.
pub enum Spawn {
    /// A new, untitled document (from a template), with its display name.
    Doc(Document, String),
    /// A file to open.
    Path(PathBuf),
}

/// State of the New Diagram (template gallery) dialog.
#[derive(Default)]
pub struct Gallery {
    pub cat: usize,
    pub search: String,
    pub sel: Option<String>,
    pub entries: Option<Vec<crate::templates::Entry>>,
}

pub struct App {
    pub doc: Document,
    pub pi: usize,
    pub sel: Vec<Id>,
    pub tool: Tool,
    pub zoom: f32,
    pub pan: Vec2,
    pub fit_pending: bool,
    pub set: Settings,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub undo: Vec<(Document, usize)>,
    pub redo: Vec<(Document, usize)>,
    pub coalesce: Option<(String, f64)>,
    pub drag: Drag,
    pub hover: Option<Id>,
    pub guides: Vec<(Pos2, Pos2)>,
    pub glue: Option<(Id, i32)>,
    pub edit: Option<Edit>,
    pub painter_style: Option<Style>,
    pub tab: Tab,
    pub search: String,
    pub images: HashMap<Id, (usize, TextureHandle)>,
    pub toast: Option<(String, f64)>,
    pub modal: Option<Modal>,
    pub open_path_pending: Option<PathBuf>,
    pub allow_close: bool,
    pub renaming: Option<(usize, String)>,
    pub last_title: String,
    pub recovery: Option<Document>,
    pub pal: Pal,
    pub ctx_menu: Option<Pos2>,
    /// Show the template gallery once the launch has settled (no file was opened).
    pub offer_gallery: bool,
    /// Page the context menu was opened on (it closes when the page changes).
    pub ctx_menu_page: usize,
    /// When the context menu opened (presses before that don't dismiss it).
    pub ctx_menu_t: f64,
    pub stencil_drag: Option<String>,
    pub canvas_rect: Rect,
    pub mouse_world: Option<Pos2>,
    pub paste_count: f32,
    pub now: f64,
    /// Name of a file that was imported from another format (not yet saved as .nqs).
    pub imported_name: Option<String>,
    /// Shape groups loaded from the user's stencil folders.
    pub user_lib: Vec<crate::userlib::UserGroup>,
    pub user_lib_loaded: bool,
    /// Stencil-panel thumbnails for user icons (None = failed to render).
    pub thumbs: HashMap<PathBuf, Option<TextureHandle>>,
    /// Sub-diagram trail: (page id, shape id) of each linked shape we opened.
    pub nav: Vec<(Id, Id)>,
    /// Shape to bring into view once the page has been fitted.
    pub focus_shape: Option<Id>,
    /// Windows this one asks the window manager to open.
    pub spawn: Vec<Spawn>,
    /// Files chosen in a file dialog, waiting to be used.
    pub picked: Vec<crate::files::Picked>,
    /// The window has agreed to close (nothing unsaved, or the user chose).
    pub want_close: bool,
    /// The window had keyboard focus last frame.
    pub focused: bool,
    /// Native menu commands routed to this window, run during its next frame.
    pub menu_queue: Vec<String>,
    pub gallery: Gallery,
}

pub const CLIP_TAG: &str = "NAQSHA-CLIP:";
/// Clipboard tag written by versions released as Schemix (still accepted on paste).
pub const OLD_CLIP_TAG: &str = "SCHEMIX-CLIP:";

#[derive(Serialize, Deserialize)]
struct Clip { shapes: Vec<Shape>, conns: Vec<Conn> }

impl App {
    pub fn new(set: Settings, recovery: Option<Document>) -> Self {
        App {
            doc: Document::default(), pi: 0, sel: vec![], tool: Tool::Pointer, zoom: 1.0, pan: vec2(40.0, 40.0), fit_pending: true, set, path: None, dirty: false,
            undo: vec![], redo: vec![], coalesce: None, drag: Drag::None, hover: None, guides: vec![], glue: None, edit: None, painter_style: None,
            tab: Tab::Home, search: String::new(), images: HashMap::new(), toast: None, modal: None, open_path_pending: None, allow_close: false, renaming: None,
            last_title: String::new(), recovery, pal: theme::LIGHT, ctx_menu: None, offer_gallery: false, ctx_menu_page: 0, ctx_menu_t: 0.0, stencil_drag: None, canvas_rect: Rect::NOTHING, mouse_world: None, paste_count: 0.0, now: 0.0, imported_name: None, user_lib: vec![], user_lib_loaded: false, thumbs: HashMap::new(), nav: vec![], focus_shape: None, spawn: vec![], picked: vec![], want_close: false, focused: false, menu_queue: vec![], gallery: Gallery::default(),
        }
    }

    pub fn page(&self) -> &Page { &self.doc.pages[self.pi] }
    pub fn page_mut(&mut self) -> &mut Page { &mut self.doc.pages[self.pi] }

    pub fn notify(&mut self, msg: impl Into<String>) { self.toast = Some((crate::platform::keys(&msg.into()), self.now)); }

    // ------------------------------------------------------------- undo
    pub fn checkpoint(&mut self) {
        self.undo.push((self.doc.clone(), self.pi));
        if self.undo.len() > 200 { self.undo.remove(0); }
        self.redo.clear();
        self.dirty = true;
        self.coalesce = None;
    }
    /// Checkpoint unless the same continuous edit happened less than a second ago.
    pub fn checkpoint_key(&mut self, key: &str) {
        if let Some((k, t)) = &self.coalesce {
            if k == key && self.now - *t < 1.0 { self.coalesce = Some((key.into(), self.now)); self.dirty = true; return; }
        }
        self.checkpoint();
        self.coalesce = Some((key.into(), self.now));
    }
    pub fn push_before(&mut self, before: Document) {
        if before != self.doc {
            self.undo.push((before, self.pi));
            if self.undo.len() > 200 { self.undo.remove(0); }
            self.redo.clear();
            self.dirty = true;
            self.coalesce = None;
        }
    }
    pub fn do_undo(&mut self) {
        if let Some((d, pi)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.doc, d), self.pi));
            self.pi = pi.min(self.doc.pages.len() - 1);
            self.after_restore();
        }
    }
    pub fn do_redo(&mut self) {
        if let Some((d, pi)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.doc, d), self.pi));
            self.pi = pi.min(self.doc.pages.len() - 1);
            self.after_restore();
        }
    }
    fn after_restore(&mut self) {
        let p = &self.doc.pages[self.pi];
        self.sel.retain(|id| p.contains(*id));
        self.edit = None;
        self.dirty = true;
        self.coalesce = None;
    }

    // ------------------------------------------------------------- selection
    pub fn is_sel(&self, id: Id) -> bool { self.sel.contains(&id) }
    pub fn sel_shapes(&self) -> Vec<&Shape> { self.page().shapes.iter().filter(|s| self.sel.contains(&s.id)).collect() }
    pub fn sel_conns(&self) -> Vec<&Conn> { self.page().conns.iter().filter(|c| self.sel.contains(&c.id)).collect() }
    pub fn expand_groups(&mut self) {
        let groups: Vec<Id> = self.page().shapes.iter().filter(|s| self.sel.contains(&s.id)).filter_map(|s| s.group).collect();
        if groups.is_empty() { return; }
        let extra: Vec<Id> = self.page().shapes.iter().filter(|s| s.group.map_or(false, |g| groups.contains(&g))).map(|s| s.id).collect();
        for id in extra { if !self.sel.contains(&id) { self.sel.push(id); } }
    }
    pub fn select_only(&mut self, id: Id) { self.sel = vec![id]; self.expand_groups(); }
    pub fn sel_bbox(&self) -> Option<Rect> {
        let mut r = None;
        for s in self.sel_shapes() { r = geom::rect_union(r, geom::bbox(s)); }
        r
    }
    pub fn select_all(&mut self) {
        let p = self.page();
        self.sel = p.shapes.iter().filter(|s| p.shape_editable(s)).map(|s| s.id).chain(p.conns.iter().filter(|c| p.conn_editable(c)).map(|c| c.id)).collect();
    }

    // ------------------------------------------------------------- creation
    pub fn make_shape(&mut self, kind: &str, center: Pos2) -> Shape {
        let d = library::def(kind);
        let id = self.doc.new_id();
        let layer = self.page().active_layer.filter(|l| self.page().layer(Some(*l)).is_some());
        let mut s = Shape { id, kind: kind.into(), w: d.w, h: d.h, text: d.text0.into(), style: library::default_style(kind), layer, ..Default::default() };
        s.x = center.x - s.w / 2.0;
        s.y = center.y - s.h / 2.0;
        if self.set.snap { s.x = geom::snap(s.x, self.set.grid_size / 2.0); s.y = geom::snap(s.y, self.set.grid_size / 2.0); }
        s
    }

    pub fn add_shape(&mut self, kind: &str, center: Pos2) -> Id {
        if let Some(p) = kind.strip_prefix("user:") {
            let name = self.user_lib.iter().flat_map(|g| g.icons.iter()).find(|i| i.path.to_string_lossy() == p).map(|i| i.name.clone()).unwrap_or_default();
            return self.add_user_icon(std::path::Path::new(p), &name, center);
        }
        self.checkpoint();
        let s = self.make_shape(kind, center);
        let id = s.id;
        // Containers go behind everything (Visio behaviour).
        if library::is_container(&kind) { self.page_mut().shapes.insert(0, s); } else { self.page_mut().shapes.push(s); }
        self.sel = vec![id];
        id
    }

    /// Insert an icon from a stencil folder as a picture with its name underneath.
    pub fn add_user_icon(&mut self, path: &std::path::Path, name: &str, center: Pos2) -> Id {
        use base64::Engine;
        let Some((png, aspect)) = crate::userlib::to_png(path, 320) else { self.notify(format!("Couldn't read {}", path.display())); return 0 };
        self.checkpoint();
        let mut s = self.make_shape("image", center);
        let side = 64.0;
        (s.w, s.h) = if aspect >= 1.0 { (side, (side / aspect).round()) } else { ((side * aspect).round(), side) };
        s.x = center.x - s.w / 2.0;
        s.y = center.y - s.h / 2.0;
        if self.set.snap { s.x = geom::snap(s.x, self.set.grid_size / 2.0); s.y = geom::snap(s.y, self.set.grid_size / 2.0); }
        s.text = name.to_string();
        s.image = Some(base64::engine::general_purpose::STANDARD.encode(png));
        let id = s.id;
        self.page_mut().shapes.push(s);
        self.sel = vec![id];
        id
    }

    pub fn load_user_lib(&mut self) {
        self.user_lib = self.set.stencil_dirs.iter().flat_map(|d| crate::userlib::scan(d)).collect();
        self.user_lib_loaded = true;
    }

    pub fn add_stencil_folder(&mut self) {
        #[cfg(target_arch = "wasm32")]
        { self.notify("Stencil folders need the desktop app"); return; }
        #[cfg(not(target_arch = "wasm32"))]
        let Some(dir) = rfd::FileDialog::new().set_title("Choose a folder of SVG icons").pick_folder() else { return };
        #[cfg(not(target_arch = "wasm32"))]
        self.add_stencil_dir(dir);
    }
    #[allow(dead_code)]
    fn add_stencil_dir(&mut self, dir: PathBuf) {
        let d = dir.to_string_lossy().to_string();
        if !self.set.stencil_dirs.contains(&d) { self.set.stencil_dirs.push(d.clone()); }
        let before = self.user_lib.iter().map(|g| g.icons.len()).sum::<usize>();
        self.load_user_lib();
        let n = self.user_lib.iter().map(|g| g.icons.len()).sum::<usize>() - before.min(self.user_lib.iter().map(|g| g.icons.len()).sum::<usize>());
        if n == 0 && !self.user_lib.iter().any(|g| g.root == d) {
            self.set.stencil_dirs.retain(|x| x != &d);
            self.notify("No SVG icons found in that folder");
        } else {
            let groups = self.user_lib.iter().filter(|g| g.root == d).count();
            self.notify(format!("Added {n} icons in {groups} group(s)"));
            self.set.show_left = true;
        }
    }

    pub fn remove_stencil_folder(&mut self, root: &str) {
        self.set.stencil_dirs.retain(|x| x != root);
        self.load_user_lib();
    }

    pub fn view_center_world(&self) -> Pos2 {
        let c = self.canvas_rect.center();
        if !self.canvas_rect.is_finite() { return pos2(400.0, 300.0); }
        ((c - self.canvas_rect.min - self.pan) / self.zoom).to_pos2()
    }

    /// AutoConnect: clone `src` in direction `dir` and connect them.
    pub fn auto_connect(&mut self, src: Id, dir: Vec2) {
        let Some(s) = self.page().shape(src).cloned() else { return };
        self.checkpoint();
        let gap = 70.0;
        let mut n = s.clone();
        n.id = self.doc.new_id();
        n.group = None;
        n.data = s.data.clone();
        n.text = library::def(&s.kind).text0.into();
        n.x += dir.x * (s.w + gap);
        n.y += dir.y * (s.h + gap);
        let nid = n.id;
        let cid = self.doc.new_id();
        let routing = self.set.routing;
        let layer = s.layer;
        let p = self.page_mut();
        p.shapes.push(n);
        p.conns.push(Conn { id: cid, from: End::Glued { shape: src, port: -1 }, to: End::Glued { shape: nid, port: -1 }, routing, layer, ..Default::default() });
        self.sel = vec![nid];
    }

    // ------------------------------------------------------------- clipboard
    pub fn clip_json(&self) -> Option<String> {
        let shapes: Vec<Shape> = self.sel_shapes().into_iter().cloned().collect();
        let ids: Vec<Id> = shapes.iter().map(|s| s.id).collect();
        let glued_in = |e: &End| match e { End::Glued { shape, .. } => ids.contains(shape), End::Free { .. } => true };
        let conns: Vec<Conn> = self.page().conns.iter().filter(|c| self.sel.contains(&c.id) || (glued_in(&c.from) && glued_in(&c.to) && !matches!((c.from, c.to), (End::Free { .. }, End::Free { .. })))).cloned().collect();
        if shapes.is_empty() && conns.is_empty() { return None; }
        Some(format!("{}{}", CLIP_TAG, serde_json::to_string(&Clip { shapes, conns }).ok()?))
    }

    pub fn delete_sel(&mut self) {
        if self.sel.is_empty() { return; }
        self.checkpoint();
        let sel = self.sel.clone();
        let p = self.page_mut();
        let locked: Vec<Id> = p.shapes.iter().filter(|s| s.locked).map(|s| s.id).collect();
        p.shapes.retain(|s| !sel.contains(&s.id) || s.locked);
        // connectors glued to removed shapes become free at their last position
        let removed: Vec<Id> = sel.iter().copied().filter(|id| !locked.contains(id)).collect();
        // resolve positions using the pre-delete state
        let before = self.undo.last().map(|(d, _)| d.pages[self.pi].clone());
        let p = &mut self.doc.pages[self.pi];
        if let Some(bp) = before {
            for c in p.conns.iter_mut() {
                for to in [false, true] {
                    let (e, other) = if to { (c.to, c.from) } else { (c.from, c.to) };
                    if let End::Glued { shape, .. } = e {
                        if removed.contains(&shape) {
                            let (pt, _) = geom::resolve_end(&bp, &e, &other);
                            let free = End::Free { x: pt.x, y: pt.y };
                            if to { c.to = free } else { c.from = free }
                        }
                    }
                }
            }
        }
        p.conns.retain(|c| !sel.contains(&c.id));
        p.conns.retain(|c| !matches!((c.from, c.to), (End::Free { x: a, y: b }, End::Free { x: c2, y: d }) if (a - c2).abs() + (b - d).abs() < 1.0));
        self.sel.clear();
    }

    pub fn paste_str(&mut self, text: &str) {
        if let Some(json) = text.strip_prefix(CLIP_TAG).or_else(|| text.strip_prefix(OLD_CLIP_TAG)) {
            if let Ok(clip) = serde_json::from_str::<Clip>(json) {
                self.paste_clip(clip);
                return;
            }
        }
        if text.trim().is_empty() { return; }
        // Plain text becomes a text shape (like Visio).
        let c = self.mouse_world.filter(|p| self.canvas_rect.contains(pos2(p.x, p.y)) || true).unwrap_or_else(|| self.view_center_world());
        let c = if self.canvas_rect.is_finite() { self.view_center_world() } else { c };
        self.checkpoint();
        let mut s = self.make_shape("text", c);
        s.text = text.to_string();
        let lines = text.lines().count().max(1) as f32;
        s.w = (text.lines().map(|l| l.chars().count()).max().unwrap_or(4) as f32 * 7.5 + 20.0).clamp(60.0, 520.0);
        s.h = lines * 17.0 + 14.0;
        let id = s.id;
        self.page_mut().shapes.push(s);
        self.sel = vec![id];
    }

    fn paste_clip(&mut self, clip: Clip) {
        self.checkpoint();
        self.paste_count += 1.0;
        let off = 20.0 * self.paste_count;
        let mut map: HashMap<Id, Id> = HashMap::new();
        let mut gmap: HashMap<Id, Id> = HashMap::new();
        let mut new_sel = vec![];
        let mut shapes = vec![];
        for mut s in clip.shapes {
            let nid = self.doc.new_id();
            map.insert(s.id, nid);
            s.id = nid;
            s.x += off;
            s.y += off;
            if let Some(g) = s.group { let ng = *gmap.entry(g).or_insert_with(|| { let i = self.doc.next_id; self.doc.next_id += 1; i }); s.group = Some(ng); }
            new_sel.push(nid);
            shapes.push(s);
        }
        let mut conns = vec![];
        for mut c in clip.conns {
            c.id = self.doc.new_id();
            for e in [&mut c.from, &mut c.to] {
                match e {
                    End::Glued { shape, port } => {
                        if let Some(n) = map.get(shape) { *e = End::Glued { shape: *n, port: *port }; }
                    }
                    End::Free { x, y } => { *x += off; *y += off; }
                }
            }
            c.translate_bends(off, off);
            new_sel.push(c.id);
            conns.push(c);
        }
        let p = self.page_mut();
        p.shapes.extend(shapes);
        p.conns.extend(conns);
        self.sel = new_sel;
    }

    pub fn duplicate(&mut self) {
        if let Some(j) = self.clip_json() { self.paste_str(&j); }
    }

    // ------------------------------------------------------------- arrange
    fn map_sel_shapes(&mut self, f: impl Fn(&mut Shape)) {
        let sel = self.sel.clone();
        for s in self.page_mut().shapes.iter_mut().filter(|s| sel.contains(&s.id) && !s.locked) { f(s); }
    }

    pub fn group(&mut self) {
        if self.sel_shapes().len() < 2 { return; }
        self.checkpoint();
        let g = self.doc.new_id();
        self.map_sel_shapes(|s| s.group = Some(g));
    }
    pub fn ungroup(&mut self) {
        if !self.sel_shapes().iter().any(|s| s.group.is_some()) { return; }
        self.checkpoint();
        self.map_sel_shapes(|s| s.group = None);
    }
    pub fn z_order(&mut self, front: bool, step: bool) {
        if self.sel_shapes().is_empty() { return; }
        self.checkpoint();
        let sel = self.sel.clone();
        let shapes = &mut self.page_mut().shapes;
        if step {
            if front {
                for i in (0..shapes.len().saturating_sub(1)).rev() {
                    if sel.contains(&shapes[i].id) && !sel.contains(&shapes[i + 1].id) { shapes.swap(i, i + 1); }
                }
            } else {
                for i in 1..shapes.len() {
                    if sel.contains(&shapes[i].id) && !sel.contains(&shapes[i - 1].id) { shapes.swap(i, i - 1); }
                }
            }
        } else {
            let (mut a, mut b): (Vec<Shape>, Vec<Shape>) = shapes.drain(..).partition(|s| sel.contains(&s.id));
            if front { b.append(&mut a); *shapes = b; } else { a.append(&mut b); *shapes = a; }
        }
    }
    /// how: 0 left,1 center,2 right,3 top,4 middle,5 bottom
    pub fn align(&mut self, how: u8) {
        let ss = self.sel_shapes();
        if ss.len() < 2 { return; }
        let bb = self.sel_bbox().unwrap();
        // Visio aligns to the primary (first selected) shape
        let first = self.sel.iter().find_map(|id| self.page().shape(*id)).map(geom::bbox).unwrap_or(bb);
        self.checkpoint();
        let sel = self.sel.clone();
        for s in self.page_mut().shapes.iter_mut().filter(|s| sel.contains(&s.id) && !s.locked) {
            let b = geom::bbox(s);
            let (dx, dy) = match how {
                0 => (first.min.x - b.min.x, 0.0),
                1 => (first.center().x - b.center().x, 0.0),
                2 => (first.max.x - b.max.x, 0.0),
                3 => (0.0, first.min.y - b.min.y),
                4 => (0.0, first.center().y - b.center().y),
                _ => (0.0, first.max.y - b.max.y),
            };
            s.x += dx;
            s.y += dy;
        }
    }
    pub fn distribute(&mut self, horizontal: bool) {
        let mut ss: Vec<(Id, Rect)> = self.sel_shapes().iter().map(|s| (s.id, geom::bbox(s))).collect();
        if ss.len() < 3 { return; }
        self.checkpoint();
        ss.sort_by(|a, b| if horizontal { a.1.center().x.total_cmp(&b.1.center().x) } else { a.1.center().y.total_cmp(&b.1.center().y) });
        let n = ss.len();
        let total: f32 = ss.iter().map(|(_, r)| if horizontal { r.width() } else { r.height() }).sum();
        let (start, end) = if horizontal { (ss[0].1.min.x, ss[n - 1].1.max.x) } else { (ss[0].1.min.y, ss[n - 1].1.max.y) };
        let gap = ((end - start) - total) / (n - 1) as f32;
        let mut cur = start;
        for (id, r) in ss {
            if let Some(s) = self.page_mut().shape_mut(id) {
                if horizontal { s.x += cur - r.min.x; cur += r.width() + gap; } else { s.y += cur - r.min.y; cur += r.height() + gap; }
            }
        }
    }
    pub fn rotate_sel(&mut self, deg: f32) {
        if self.sel_shapes().is_empty() { return; }
        self.checkpoint();
        self.map_sel_shapes(|s| s.rot = (s.rot + deg).rem_euclid(360.0));
    }
    pub fn flip(&mut self, h: bool) {
        if self.sel_shapes().is_empty() { return; }
        self.checkpoint();
        self.map_sel_shapes(|s| if h { s.flip_h = !s.flip_h } else { s.flip_v = !s.flip_v });
    }
    pub fn toggle_lock(&mut self) {
        if self.sel_shapes().is_empty() { return; }
        self.checkpoint();
        let lock = !self.sel_shapes().iter().all(|s| s.locked);
        let sel = self.sel.clone();
        for s in self.page_mut().shapes.iter_mut().filter(|s| sel.contains(&s.id)) { s.locked = lock; }
    }
    pub fn nudge(&mut self, d: Vec2) {
        if self.sel.is_empty() { return; }
        self.checkpoint_key("nudge");
        self.map_sel_shapes(|s| { s.x += d.x; s.y += d.y; });
        let sel = self.sel.clone();
        for c in self.page_mut().conns.iter_mut().filter(|c| sel.contains(&c.id)) {
            for e in [&mut c.from, &mut c.to] { if let End::Free { x, y } = e { *x += d.x; *y += d.y; } }
            c.translate_bends(d.x, d.y);
        }
    }

    // ------------------------------------------------------------- style
    pub fn apply_style(&mut self, key: &str, f: impl Fn(&mut Style)) {
        if self.sel.is_empty() { return; }
        self.checkpoint_key(key);
        let sel = self.sel.clone();
        let p = self.page_mut();
        for s in p.shapes.iter_mut().filter(|s| sel.contains(&s.id)) { f(&mut s.style); }
        for c in p.conns.iter_mut().filter(|c| sel.contains(&c.id)) { f(&mut c.style); }
    }

    pub fn apply_diagram_theme(&mut self, i: usize) {
        self.checkpoint();
        let p = &mut self.doc.pages[self.pi];
        if i == usize::MAX {
            for s in p.shapes.iter_mut() { if !library::is_textlike(&s.kind) && s.kind != "image" { s.style.fill = library::default_style(&s.kind).fill; s.style.stroke = None; s.style.text = None; } }
            for c in p.conns.iter_mut() { c.style.stroke = None; c.style.text = None; }
            return;
        }
        let (_, f, st, t, cn) = theme::DIAGRAM_THEMES[i];
        for s in p.shapes.iter_mut() {
            if library::is_textlike(&s.kind) || s.kind == "image" { s.style.text = Some(Col::hex(t)); continue; }
            let none_fill = s.style.fill.map_or(false, |c| c.is_none());
            if !none_fill { s.style.fill = Some(if library::is_container(&s.kind) { let mut c = Col::hex(st); c.0[3] = 14; c } else { Col::hex(f) }); }
            s.style.stroke = Some(Col::hex(st));
            s.style.text = Some(Col::hex(t));
        }
        for c in p.conns.iter_mut() { c.style.stroke = Some(Col::hex(cn)); c.style.text = Some(Col::hex(t)); }
    }

    /// Back to automatic routing for the selected connectors (all on the page when none selected).
    pub fn reset_routes(&mut self) {
        let sel = self.sel.clone();
        let any = self.page().conns.iter().any(|c| sel.contains(&c.id));
        if !self.page().conns.iter().any(|c| c.bends.is_some() && (!any || sel.contains(&c.id))) { return; }
        self.checkpoint();
        for c in self.page_mut().conns.iter_mut() { if !any || sel.contains(&c.id) { c.bends = None; } }
    }

    pub fn set_routing_all(&mut self, r: Routing) {
        self.set.routing = r;
        let sel = self.sel_conns().len();
        self.checkpoint();
        let selv = self.sel.clone();
        for c in self.page_mut().conns.iter_mut() { if sel == 0 || selv.contains(&c.id) { c.routing = r; c.bends = None; } }
    }

    // ------------------------------------------------------------- pages
    pub fn add_page(&mut self) {
        self.checkpoint();
        let id = self.doc.new_id();
        let n = self.doc.pages.len() + 1;
        let (w, h) = (self.page().width, self.page().height);
        self.doc.pages.push(Page { id, name: format!("Page-{n}"), width: w, height: h, ..Default::default() });
        self.go_page(self.doc.pages.len() - 1);
    }
    pub fn dup_page(&mut self) {
        self.checkpoint();
        let mut p = self.page().clone();
        p.id = self.doc.new_id();
        p.name = format!("{} (copy)", p.name);
        // ids are page-scoped in practice; give fresh ids so cross-page clipboard stays unambiguous
        let mut map = HashMap::new();
        for s in p.shapes.iter_mut() { let n = self.doc.new_id(); map.insert(s.id, n); s.id = n; }
        for c in p.conns.iter_mut() {
            c.id = self.doc.new_id();
            for e in [&mut c.from, &mut c.to] { if let End::Glued { shape, .. } = e { if let Some(n) = map.get(shape) { *shape = *n; } } }
        }
        self.doc.pages.insert(self.pi + 1, p);
        self.go_page(self.pi + 1);
    }
    pub fn del_page(&mut self) {
        if self.doc.pages.len() < 2 { self.notify("A diagram needs at least one page"); return; }
        self.checkpoint();
        let gone = self.page().id;
        self.unlink_page(gone);
        self.doc.pages.remove(self.pi);
        self.go_page(self.pi.min(self.doc.pages.len() - 1));
    }
    pub fn go_page(&mut self, i: usize) {
        self.pi = i;
        self.ctx_menu = None;
        self.sel.clear();
        self.edit = None;
        self.fit_pending = true;
    }

    pub fn fit_page_to_content(&mut self) {
        let mut r: Option<Rect> = None;
        for s in &self.page().shapes { r = geom::rect_union(r, geom::bbox(s)); }
        let Some(r) = r else { return };
        self.checkpoint();
        let m = 40.0;
        let dx = m - r.min.x;
        let dy = m - r.min.y;
        let p = self.page_mut();
        for s in p.shapes.iter_mut() { s.x += dx; s.y += dy; }
        for c in p.conns.iter_mut() { for e in [&mut c.from, &mut c.to] { if let End::Free { x, y } = e { *x += dx; *y += dy; } } c.translate_bends(dx, dy); }
        p.width = (r.width() + m * 2.0).ceil();
        p.height = (r.height() + m * 2.0).ceil();
        self.fit_pending = true;
    }

    /// Auto arrange the page, or the selected shapes when two or more are selected.
    pub fn auto_arrange(&mut self, mode: crate::layout::Mode) {
        let before = self.doc.clone();
        let sel = self.sel.clone();
        match crate::layout::auto_arrange(self.page_mut(), &sel, mode) {
            Ok(msg) => { self.push_before(before); self.set.layout = mode_name(mode).into(); self.notify(msg); self.fit_pending = true; }
            Err(e) => { self.doc = before; self.notify(e); }
        }
    }

    /// Back to automatic connector routes (selected connectors, or all of them).
    pub fn tidy_connectors(&mut self) {
        let before = self.doc.clone();
        let sel = self.sel.clone();
        let n = crate::layout::tidy_connectors(self.page_mut(), &sel);
        self.push_before(before);
        self.notify(if n == 0 { "No connectors to tidy".to_string() } else { format!("Tidied {n} connector(s)") });
    }

    /// The layout used last ("Auto Arrange" repeats it).
    pub fn last_layout(&self) -> crate::layout::Mode {
        match self.set.layout.as_str() { "lr" => crate::layout::Mode::LeftRight, "grid" => crate::layout::Mode::Grid, _ => crate::layout::Mode::TopDown }
    }

    // ------------------------------------------------------------- files
    pub fn title(&self) -> String {
        let name = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).or_else(|| self.imported_name.clone()).unwrap_or_else(|| "Untitled".into());
        name
    }
    pub fn add_recent(&mut self, p: &std::path::Path) {
        let s = p.to_string_lossy().to_string();
        self.set.recent.retain(|r| r != &s);
        self.set.recent.insert(0, s);
        self.set.recent.truncate(8);
    }
    pub fn load_path(&mut self, path: PathBuf) {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string());
        let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        self.load_bytes(&name, Some(path), bytes);
    }

    /// Open a file's contents in this window. `path` is where it lives on disk (desktop only).
    pub fn load_bytes(&mut self, name: &str, path: Option<PathBuf>, bytes: Result<Vec<u8>, String>) {
        // a file arriving (Finder, drag and drop) answers the "what do you want to open?" question
        if matches!(self.modal, Some(Modal::NewDiagram)) { self.modal = None; }
        self.offer_gallery = false;
        let fname = std::path::Path::new(name);
        match bytes.and_then(|b| read_bytes(name, b)) {
            Ok((mut d, notes, native)) => {
                if d.pages.is_empty() { d = Document::default(); }
                d.fix_ids();
                self.doc = d;
                self.pi = 0;
                self.sel.clear();
                self.undo.clear();
                self.redo.clear();
                self.images.clear();
                if let Some(p) = &path { self.add_recent(p); }
                self.fit_pending = true;
                self.nav.clear();
                if native {
                    self.dirty = false;
                    // in the browser there is no path: the file name stays as the title
                    self.imported_name = if path.is_none() { fname.file_stem().map(|s| s.to_string_lossy().to_string()) } else { None };
                    self.path = path;
                } else {
                    // imported: saving goes to a new .nqs (or Save As to Visio/draw.io)
                    self.dirty = true;
                    self.imported_name = fname.file_stem().map(|s| s.to_string_lossy().to_string());
                    self.path = None;
                    let ext = fname.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
                    let mut msg = format!("Imported {ext} — {} page(s)", self.doc.pages.len());
                    for n in notes { msg.push_str(" · "); msg.push_str(&n); }
                    self.notify(msg);
                }
            }
            Err(e) => self.notify(format!("Couldn't open file: {e}")),
        }
    }
    pub fn open_dialog(&mut self) {
        self.pick_file(crate::files::Purpose::Open, "Open", crate::files::OPEN_FILTERS);
    }

    /// Use the files picked since the last frame.
    pub fn handle_picked(&mut self) {
        use crate::files::Purpose;
        for f in self.take_picked() {
            match f.purpose {
                Purpose::Open => match f.path {
                    Some(p) => self.open_or_spawn(p),
                    None => self.open_bytes(&f.name, f.bytes),
                },
                Purpose::Script => self.insert_script(&f.name, f.path.as_deref(), f.bytes),
                Purpose::Pipeline => self.insert_pipeline(&f.name, f.bytes),
                Purpose::Image => self.insert_image_bytes(f.bytes),
            }
        }
    }

    /// Open a file that has no path (browser): here if this window has nothing to lose,
    /// otherwise in a new tab.
    pub fn open_bytes(&mut self, name: &str, bytes: Result<Vec<u8>, String>) {
        if self.is_pristine() || !self.dirty || !crate::files::WEB { self.load_bytes(name, None, bytes); return; }
        match bytes.and_then(|b| read_bytes(name, b)) {
            Ok((doc, _, _)) => {
                let title = std::path::Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                self.spawn.push(Spawn::Doc(doc, title));
            }
            Err(e) => self.notify(format!("Couldn't open file: {e}")),
        }
    }

    /// Nothing to lose: an untitled, unchanged, empty document.
    pub fn is_pristine(&self) -> bool {
        self.path.is_none() && self.imported_name.is_none() && !self.dirty && self.doc.pages.iter().all(|p| p.shapes.is_empty() && p.conns.is_empty())
    }

    /// Open a file here if this window is empty, otherwise in a new window.
    pub fn open_or_spawn(&mut self, p: PathBuf) {
        // already open in this window: just say so
        if self.path.as_ref() == Some(&p) { self.notify("That diagram is already open in this window"); return; }
        if self.is_pristine() { self.load_path(p); } else { self.spawn.push(Spawn::Path(p)); }
    }

    /// Start a document from a template: here if this window is empty, otherwise in a new window.
    pub fn new_from(&mut self, doc: Document, name: &str) {
        if self.is_pristine() { self.adopt(doc, name); } else { self.spawn.push(Spawn::Doc(doc, name.to_string())); }
    }

    /// Take over a fresh (unsaved) document.
    pub fn adopt(&mut self, mut doc: Document, name: &str) {
        if doc.pages.is_empty() { doc = Document::default(); }
        doc.fix_ids();
        self.doc = doc;
        self.pi = 0;
        self.sel.clear();
        self.undo.clear();
        self.redo.clear();
        self.nav.clear();
        self.path = None;
        self.imported_name = if name.is_empty() || name == "Blank Diagram" { None } else { Some(name.to_string()) };
        self.dirty = false;
        self.recovery = None;
        self.fit_pending = true;
    }

    /// Save the current diagram into the user's templates folder.
    pub fn save_as_template(&mut self) {
        if crate::files::WEB { self.notify("Saving templates needs the desktop app"); return; }
        let Some(dir) = crate::templates::user_dir() else { self.notify("No templates folder available"); return };
        let _ = std::fs::create_dir_all(&dir);
        #[cfg(target_arch = "wasm32")]
        let p = dir.join(format!("{}.nqs", self.title()));
        #[cfg(not(target_arch = "wasm32"))]
        let Some(p) = rfd::FileDialog::new().set_title("Save as Template").set_directory(&dir).add_filter("Naqsha template", &["nqs"]).set_file_name(&format!("{}.nqs", self.title())).save_file() else { return };
        let p = if p.extension().is_none() { p.with_extension("nqs") } else { p };
        match write_foreign(&self.doc, &p) {
            Ok(_) => { self.gallery.entries = None; self.notify(format!("Saved template “{}” — find it under New › My Templates", p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())); }
            Err(e) => self.notify(format!("Couldn't save the template: {e}")),
        }
    }
    /// Returns true if saved. Save As can also write Visio (.vsdx) or draw.io files.
    pub fn save(&mut self, save_as: bool) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            // the browser saves by downloading a .nqs; Export has Visio and draw.io
            let _ = save_as;
            let name = format!("{}.nqs", self.title());
            return match serde_json::to_string_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|t| crate::files::download(&name, t.as_bytes(), "application/json")) {
                Ok(_) => { self.dirty = false; self.imported_name = Some(self.title()); self.notify(format!("Downloaded {name}")); true }
                Err(e) => { self.notify(format!("Save failed: {e}")); false }
            };
        }
        #[allow(unreachable_code)]
        let path = if save_as || self.path.is_none() {
            let name = format!("{}.nqs", self.title());
            #[cfg(not(target_arch = "wasm32"))]
            match crate::files::save_dialog("Save", &name, &[("Naqsha diagram", &["nqs"]), ("Microsoft Visio drawing", &["vsdx"]), ("draw.io diagram", &["drawio"])])
            { Some(p) => p, None => return false }
            #[cfg(target_arch = "wasm32")]
            PathBuf::from(name)
        } else { self.path.clone().unwrap() };
        let path = if path.extension().is_none() { path.with_extension("nqs") } else { path };
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "vsdx" || ext == "drawio" {
            // foreign formats are written as exports; the working document stays as it is
            return match write_foreign(&self.doc, &path) {
                Ok(_) => { self.notify(format!("Saved a copy as {}", path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())); true }
                Err(e) => { self.notify(format!("Save failed: {e}")); false }
            };
        }
        match serde_json::to_string_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|t| std::fs::write(&path, t).map_err(|e| e.to_string())) {
            Ok(_) => { self.dirty = false; self.add_recent(&path); self.path = Some(path); self.imported_name = None; self.notify("Saved"); true }
            Err(e) => { self.notify(format!("Save failed: {e}")); false }
        }
    }

    /// Ask to save if dirty, then perform the pending action.
    pub fn guard(&mut self, p: Pending) {
        // only closing can lose work: New and Open use a new window when this one has a document
        if self.dirty && matches!(p, Pending::Close) { self.modal = Some(Modal::Unsaved(p)); } else { self.run_pending(p); }
    }
    pub fn run_pending(&mut self, p: Pending) {
        match p {
            Pending::Close => self.allow_close = true,
            Pending::OpenPath => { if let Some(p) = self.open_path_pending.take() { self.open_or_spawn(p); } }
        }
    }

    /// Insert a shell script (or any text file) as a script block showing its code.
    pub fn insert_script_file(&mut self) {
        self.pick_file(crate::files::Purpose::Script, "Insert Script File", &[]);
    }
    fn insert_script(&mut self, name: &str, path: Option<&std::path::Path>, bytes: Result<Vec<u8>, String>) {
        let Ok(bytes) = bytes else { self.notify("Couldn't read the file"); return };
        let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
        if text.contains('\0') { self.notify("That doesn't look like a text file"); return; }
        let c = self.view_center_world();
        self.checkpoint();
        let mut s = self.make_shape("p_script", c);
        let n = text.lines().count().clamp(1, 30) as f32;
        let widest = text.lines().take(30).map(|l| l.chars().count()).max().unwrap_or(20).clamp(24, 90) as f32;
        s.w = (widest * 7.0 + 30.0).round();
        s.h = (n * 14.4 + 42.0).round();
        s.x = c.x - s.w / 2.0;
        s.y = c.y - s.h / 2.0;
        s.text = text.trim_end().to_string();
        s.data = vec![("file".into(), name.to_string())];
        if let Some(p) = path { s.data.push(("path".into(), p.to_string_lossy().to_string())); }
        let id = s.id;
        self.page_mut().shapes.push(s);
        self.sel = vec![id];
    }

    /// Read a CI/CD pipeline definition and add its diagram pages to this document.
    pub fn insert_pipeline_file(&mut self) {
        self.pick_file(crate::files::Purpose::Pipeline, "Insert Pipeline (GitHub Actions, GitLab CI, Azure Pipelines, Bitbucket, Jenkinsfile)", &[]);
    }
    fn insert_pipeline(&mut self, name: &str, bytes: Result<Vec<u8>, String>) {
        let text = match bytes { Ok(b) => String::from_utf8_lossy(&b).to_string(), Err(e) => { self.notify(format!("Couldn't read the file: {e}")); return } };
        let name = if std::path::Path::new(name).extension().is_none() { "Jenkinsfile".to_string() } else { name.to_string() };
        match crate::formats::pipeline::import(&name, &text) {
            Ok(imp) => {
                self.checkpoint();
                let at = self.doc.absorb(imp.doc);
                self.go_page(at);
                if let Some(n) = imp.notes.first() { self.notify(n.clone()); }
            }
            Err(e) => self.notify(format!("Couldn't read the pipeline: {e}")),
        }
    }

    pub fn insert_image(&mut self) {
        self.pick_file(crate::files::Purpose::Image, "Insert Picture", &[("Image", &["png", "jpg", "jpeg"])]);
    }
    fn insert_image_bytes(&mut self, bytes: Result<Vec<u8>, String>) {
        let Ok(bytes) = bytes else { self.notify("Couldn't read image"); return };
        let Ok(img) = image::load_from_memory(&bytes) else { self.notify("Unsupported image"); return };
        use base64::Engine;
        let mut png = vec![];
        if img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_err() { return; }
        let (w, h) = (img.width() as f32, img.height() as f32);
        let scale = (360.0 / w.max(h)).min(1.0);
        let c = self.view_center_world();
        self.checkpoint();
        let mut s = self.make_shape("image", c);
        s.w = (w * scale).round();
        s.h = (h * scale).round();
        s.x = c.x - s.w / 2.0;
        s.y = c.y - s.h / 2.0;
        s.text.clear();
        s.image = Some(base64::engine::general_purpose::STANDARD.encode(&png));
        let id = s.id;
        self.page_mut().shapes.push(s);
        self.sel = vec![id];
    }

    pub fn zoom_at(&mut self, factor: f32, screen: Pos2) {
        let nz = (self.zoom * factor).clamp(0.1, 8.0);
        let local = screen - self.canvas_rect.min;
        self.pan = local - (local - self.pan) * (nz / self.zoom);
        self.zoom = nz;
    }
    pub fn zoom_center(&mut self, factor: f32) { let c = self.canvas_rect.center(); self.zoom_at(factor, c); }
    pub fn set_zoom(&mut self, z: f32) { let f = z / self.zoom; self.zoom_center(f); }
    pub fn fit(&mut self) {
        let r = self.canvas_rect;
        if !r.is_finite() || r.width() < 50.0 { return; }
        let p = self.page();
        let mut b = Rect::from_min_size(Pos2::ZERO, vec2(p.width, p.height));
        for s in &p.shapes { b = b.union(geom::bbox(s)); }
        let m = 36.0;
        let z = ((r.width() - m * 2.0) / b.width()).min((r.height() - m * 2.0) / b.height()).clamp(0.1, 4.0);
        self.zoom = z;
        self.pan = (r.size() - b.size() * z) / 2.0 - b.min.to_vec2() * z;
        self.fit_pending = false;
    }
}

/// Read any supported file. Returns (document, notes, is_native) — native means .nqs (or the older .schx).
pub fn read_any(path: &std::path::Path) -> Result<(Document, Vec<String>, bool), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    read_bytes(&path.to_string_lossy(), bytes)
}

/// Read a file's contents; `name` (a file name or path) tells the format.
pub fn read_bytes(name: &str, bytes: Vec<u8>) -> Result<(Document, Vec<String>, bool), String> {
    let path = std::path::Path::new(name);
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Page-1".into());
    match ext.as_str() {
        "vsdx" | "vstx" | "vssx" => crate::formats::vsdx::import(bytes).map(|i| (i.doc, i.notes, false)),
        "vsd" | "vss" | "vst" => Err("legacy binary Visio files (.vsd) aren't supported — open it in Visio and save as .vsdx".into()),
        "svg" => crate::formats::svgin::import(&bytes, &stem).map(|i| (i.doc, i.notes, false)),
        "drawio" | "xml" | "dio" => crate::formats::drawio::import(&String::from_utf8_lossy(&bytes)).map(|i| (i.doc, i.notes, false)),
        "yml" | "yaml" | "groovy" | "jenkinsfile" => {
            let name = path.to_string_lossy();
            crate::formats::pipeline::import(&name, &String::from_utf8_lossy(&bytes)).map(|i| (i.doc, i.notes, false))
        }
        _ if path.file_name().map_or(false, |n| n.to_string_lossy().to_lowercase().starts_with("jenkinsfile")) => {
            crate::formats::pipeline::import("Jenkinsfile", &String::from_utf8_lossy(&bytes)).map(|i| (i.doc, i.notes, false))
        }
        _ => {
            let t = String::from_utf8_lossy(&bytes);
            if t.trim_start().starts_with('<') {
                if t.contains("<mxfile") || t.contains("<mxGraphModel") { return crate::formats::drawio::import(&t).map(|i| (i.doc, i.notes, false)); }
                if t.contains("<svg") { return crate::formats::svgin::import(&bytes, &stem).map(|i| (i.doc, i.notes, false)); }
            }
            serde_json::from_str::<Document>(&t).map(|d| (d, vec![], true)).map_err(|e| e.to_string())
        }
    }
}

pub fn write_foreign(doc: &Document, path: &std::path::Path) -> Result<(), String> {
    std::fs::write(path, foreign_bytes(doc, path)?).map_err(|e| e.to_string())
}

/// A document in the format its file name asks for (.vsdx, .drawio, otherwise .nqs).
pub fn foreign_bytes(doc: &Document, path: &std::path::Path) -> Result<Vec<u8>, String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    Ok(match ext.as_str() {
        "vsdx" => crate::formats::vsdx::export(doc)?,
        "drawio" | "xml" => crate::formats::drawio::export(doc).into_bytes(),
        _ => serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?.into_bytes(),
    })
}

pub fn mode_name(m: crate::layout::Mode) -> &'static str {
    match m { crate::layout::Mode::TopDown => "tb", crate::layout::Mode::LeftRight => "lr", crate::layout::Mode::Grid => "grid" }
}
