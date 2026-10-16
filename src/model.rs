//! Document model: pages, shapes, connectors. Serialized as JSON (`.nqs`; files from before the rename use `.schx`).
use serde::{Deserialize, Serialize};

pub type Id = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Col(pub [u8; 4]);

impl Col {
    pub const NONE: Col = Col([0, 0, 0, 0]);
    pub fn hex(s: &str) -> Col {
        let s = s.trim_start_matches('#');
        let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0);
        if s.len() >= 6 { Col([p(0), p(2), p(4), 255]) } else { Col::NONE }
    }
    pub fn is_none(&self) -> bool { self.0[3] == 0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Dash { #[default] Solid, Dashed, Dotted }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAlign { Left, Center, Right }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign { Top, Middle, Bottom }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Routing { #[default] Orthogonal, Straight, Curved }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Arrow { #[default] None, Filled, Open, Diamond, Circle }

/// `None` values resolve to theme defaults at render time, so default shapes adapt to light/dark mode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Style {
    pub fill: Option<Col>,
    pub stroke: Option<Col>,
    pub text: Option<Col>,
    pub width: Option<f32>,
    pub dash: Dash,
    pub font_size: Option<f32>,
    pub bold: bool,
    pub italic: bool,
    pub align: Option<HAlign>,
    pub valign: Option<VAlign>,
    pub shadow: bool,
    pub opacity: Option<f32>,
    /// Code font (monospace), e.g. for script blocks.
    pub mono: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shape {
    pub id: Id,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rot: f32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub text: String,
    pub style: Style,
    pub data: Vec<(String, String)>,
    pub locked: bool,
    pub group: Option<Id>,
    /// Base64-encoded image bytes for image shapes.
    pub image: Option<String>,
    /// Layer membership (None = no layer, always visible).
    pub layer: Option<Id>,
    /// Free-form geometry for imported shapes (kind "custom"), normalized 0..1.
    pub path: Option<Vec<CPath>>,
    /// Explicit text block [x, y, w, h] in local px (imported Visio text blocks).
    pub tbox: Option<[f32; 4]>,
    /// Text block reads bottom-to-top (rotated 90°).
    pub tvert: bool,
    /// Page this shape opens (a sub-diagram), by page id.
    pub link: Option<Id>,
    /// Don't draw the linked page's preview inside the shape.
    pub no_preview: bool,
}

/// One sub-path of a custom shape, points normalized to the shape's box.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CPath {
    pub pts: Vec<[f32; 2]>,
    pub closed: bool,
    pub fill: bool,
    pub stroke: bool,
    /// Filled with the line colour (solid valve bodies, arrow heads…).
    pub solid: bool,
    /// Drawn dashed.
    pub dashed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layer {
    pub id: Id,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub print: bool,
    /// A background layer (e.g. a drawing sheet's frame and title block): shown and printed, but
    /// shapes on it can't be selected, connectors don't glue to them or route around them.
    pub background: bool,
}

impl Default for Layer {
    fn default() -> Self { Layer { id: 0, name: "Layer 1".into(), visible: true, locked: false, print: true, background: false } }
}

impl Default for Shape {
    fn default() -> Self {
        Shape { id: 0, kind: "rect".into(), x: 0.0, y: 0.0, w: 120.0, h: 72.0, rot: 0.0, flip_h: false, flip_v: false, text: String::new(), style: Style::default(), data: vec![], locked: false, group: None, image: None, layer: None, path: None, tbox: None, tvert: false, link: None, no_preview: false }
    }
}

impl Shape {
    pub fn center(&self) -> (f32, f32) { (self.x + self.w / 2.0, self.y + self.h / 2.0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum End {
    Free { x: f32, y: f32 },
    /// `port == -1` means dynamic glue (nearest connection point).
    Glued { shape: Id, port: i32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Conn {
    pub id: Id,
    pub from: End,
    pub to: End,
    pub routing: Routing,
    pub style: Style,
    pub arrow_start: Arrow,
    pub arrow_end: Arrow,
    pub text: String,
    pub data: Vec<(String, String)>,
    pub layer: Option<Id>,
    /// Manual route: interior points set by dragging segments (None = automatic routing).
    pub bends: Option<Vec<[f32; 2]>>,
}

impl Conn {
    pub fn has_bends(&self) -> bool { self.bends.as_ref().map_or(false, |b| !b.is_empty()) && self.routing != Routing::Curved }
    pub fn translate_bends(&mut self, dx: f32, dy: f32) {
        if let Some(b) = self.bends.as_mut() { for p in b.iter_mut() { p[0] += dx; p[1] += dy; } }
    }
}

impl Default for Conn {
    fn default() -> Self {
        Conn { id: 0, from: End::Free { x: 0.0, y: 0.0 }, to: End::Free { x: 0.0, y: 0.0 }, routing: Routing::Orthogonal, style: Style::default(), arrow_start: Arrow::None, arrow_end: Arrow::Filled, text: String::new(), data: vec![], layer: None, bends: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub id: Id,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub bg: Option<Col>,
    pub shapes: Vec<Shape>,
    pub conns: Vec<Conn>,
    pub layers: Vec<Layer>,
    /// Layer that newly created shapes join.
    pub active_layer: Option<Id>,
    /// Draw small hops where connectors cross.
    pub line_jumps: bool,
    /// Right-angle connectors route around shapes in their way.
    pub avoid_shapes: bool,
}

impl Default for Page {
    fn default() -> Self { Page { id: 0, name: "Page-1".into(), width: 1400.0, height: 990.0, bg: None, shapes: vec![], conns: vec![], layers: vec![], active_layer: None, line_jumps: true, avoid_shapes: true } }
}

impl Page {
    pub fn shape(&self, id: Id) -> Option<&Shape> { self.shapes.iter().find(|s| s.id == id) }
    pub fn shape_mut(&mut self, id: Id) -> Option<&mut Shape> { self.shapes.iter_mut().find(|s| s.id == id) }
    pub fn conn(&self, id: Id) -> Option<&Conn> { self.conns.iter().find(|c| c.id == id) }
    pub fn conn_mut(&mut self, id: Id) -> Option<&mut Conn> { self.conns.iter_mut().find(|c| c.id == id) }
    pub fn contains(&self, id: Id) -> bool { self.shape(id).is_some() || self.conn(id).is_some() }
    pub fn layer(&self, id: Option<Id>) -> Option<&Layer> { id.and_then(|i| self.layers.iter().find(|l| l.id == i)) }
    pub fn layer_visible(&self, id: Option<Id>) -> bool { self.layer(id).map_or(true, |l| l.visible) }
    pub fn layer_locked(&self, id: Option<Id>) -> bool { self.layer(id).map_or(false, |l| l.locked || l.background) }
    /// On a background layer (see `Layer::background`).
    pub fn in_background(&self, s: &Shape) -> bool { self.layer(s.layer).map_or(false, |l| l.background) }
    /// Visible for display and hit-testing.
    pub fn shape_shown(&self, s: &Shape) -> bool { self.layer_visible(s.layer) }
    /// Can be selected / edited.
    pub fn shape_editable(&self, s: &Shape) -> bool { self.layer_visible(s.layer) && !self.layer_locked(s.layer) }
    pub fn conn_shown(&self, c: &Conn) -> bool { self.layer_visible(c.layer) }
    pub fn conn_editable(&self, c: &Conn) -> bool { self.layer_visible(c.layer) && !self.layer_locked(c.layer) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    pub app: String,
    pub version: u32,
    pub pages: Vec<Page>,
    pub next_id: Id,
}

impl Default for Document {
    fn default() -> Self {
        let mut d = Document { app: "Naqsha".into(), version: 1, pages: vec![], next_id: 1 };
        let id = d.new_id();
        d.pages.push(Page { id, ..Default::default() });
        d
    }
}

impl Document {
    pub fn page_by_id(&self, id: Id) -> Option<&Page> { self.pages.iter().find(|p| p.id == id) }
    pub fn page_index(&self, id: Id) -> Option<usize> { self.pages.iter().position(|p| p.id == id) }
    /// The page a shape opens, if its link still points at a page in this document.
    pub fn linked_page(&self, s: &Shape) -> Option<&Page> { s.link.and_then(|id| self.page_by_id(id)) }
    /// Shapes (page index, shape id) that open the given page.
    pub fn parents_of(&self, page_id: Id) -> Vec<(usize, Id)> {
        let mut v = vec![];
        for (i, p) in self.pages.iter().enumerate() {
            if p.id == page_id { continue; }
            for s in &p.shapes { if s.link == Some(page_id) { v.push((i, s.id)); } }
        }
        v
    }
    /// Append another document's pages, giving everything fresh ids (links and glue follow).
    /// Returns the index of the first appended page.
    pub fn absorb(&mut self, mut other: Document) -> usize {
        let first = self.pages.len();
        let mut map: std::collections::HashMap<Id, Id> = std::collections::HashMap::new();
        let mut fresh = |doc: &mut Document, old: Id| -> Id { *map.entry(old).or_insert_with(|| doc.new_id()) };
        let mut pages = std::mem::take(&mut other.pages);
        for p in pages.iter_mut() {
            p.id = fresh(self, p.id);
            for l in p.layers.iter_mut() { l.id = fresh(self, l.id); }
            if let Some(a) = p.active_layer { p.active_layer = Some(fresh(self, a)); }
            for sh in p.shapes.iter_mut() {
                sh.id = fresh(self, sh.id);
                if let Some(g) = sh.group { sh.group = Some(fresh(self, g)); }
                if let Some(l) = sh.layer { sh.layer = Some(fresh(self, l)); }
            }
            for c in p.conns.iter_mut() {
                c.id = fresh(self, c.id);
                if let Some(l) = c.layer { c.layer = Some(fresh(self, l)); }
                for e in [&mut c.from, &mut c.to] { if let End::Glued { shape, .. } = e { *shape = fresh(self, *shape); } }
            }
        }
        // links point at pages, which all have new ids by now
        for p in pages.iter_mut() { for sh in p.shapes.iter_mut() { if let Some(l) = sh.link { sh.link = map.get(&l).copied(); } } }
        // keep page names unique
        for p in pages.iter_mut() {
            let base = p.name.clone();
            let mut n = 2;
            while self.pages.iter().any(|q| q.name == p.name) { p.name = format!("{base} ({n})"); n += 1; }
            self.pages.push(p.clone());
        }
        first
    }

    pub fn new_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    /// Make sure `next_id` is above every id in the document (after load / paste).
    pub fn fix_ids(&mut self) {
        let mut m = self.next_id;
        for p in &self.pages {
            m = m.max(p.id + 1);
            for s in &p.shapes { m = m.max(s.id + 1); if let Some(g) = s.group { m = m.max(g + 1); } }
            for l in &p.layers { m = m.max(l.id + 1); }
            for c in &p.conns { m = m.max(c.id + 1); }
        }
        self.next_id = m;
    }
}
