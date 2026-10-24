//! Sub-diagrams: shapes that open another page, previews of that page inside them, and
//! navigation between a page and the pages it drills into.
use crate::app::App;
use crate::geom;
use crate::model::*;
use egui::{pos2, Pos2, Rect};

impl App {
    /// The linked page index of a shape on the current page.
    pub fn link_target(&self, shape: Id) -> Option<usize> {
        let s = self.page().shape(shape)?;
        s.link.and_then(|id| self.doc.page_index(id))
    }

    /// A page name that isn't taken yet, based on `base`.
    fn unique_page_name(&self, base: &str) -> String {
        let base = base.lines().next().unwrap_or("").trim();
        let base = if base.is_empty() { "Sub-diagram" } else { base };
        let taken = |n: &str| self.doc.pages.iter().any(|p| p.name == n);
        if !taken(base) { return base.to_string(); }
        (2..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap()
    }

    /// Give `shape` a new, empty page of its own and open it.
    pub fn create_subpage(&mut self, shape: Id) {
        let Some(s) = self.page().shape(shape) else { return };
        if s.locked { self.notify("Unlock the shape first"); return; }
        let name = self.unique_page_name(&s.text);
        self.checkpoint();
        let id = self.doc.new_id();
        let (w, h) = (self.page().width, self.page().height);
        let parent = self.page().name.clone();
        self.doc.pages.insert(self.pi + 1, Page { id, name: name.clone(), width: w, height: h, ..Default::default() });
        if let Some(s) = self.page_mut().shape_mut(shape) { s.link = Some(id); s.no_preview = false; }
        self.open_link(shape);
        self.notify(crate::i18n::trf("“{name}” opens from “{parent}” — use Back (⌘↑) to return", &[("name", &name), ("parent", &parent)]));
    }

    /// Link a shape to an existing page (None removes the link).
    pub fn set_link(&mut self, shape: Id, page: Option<Id>) {
        if self.page().shape(shape).map_or(true, |s| s.link == page) { return; }
        self.checkpoint();
        if let Some(s) = self.page_mut().shape_mut(shape) { s.link = page; }
    }

    pub fn toggle_preview(&mut self, shape: Id) {
        self.checkpoint();
        if let Some(s) = self.page_mut().shape_mut(shape) { s.no_preview = !s.no_preview; }
    }

    /// Go to the page a shape links to, remembering where we came from.
    pub fn open_link(&mut self, shape: Id) -> bool {
        let Some(i) = self.link_target(shape) else { return false };
        if i == self.pi { return false; }
        let here = self.page().id;
        self.nav.push((here, shape));
        if self.nav.len() > 64 { self.nav.remove(0); }
        self.go_page(i);
        true
    }

    /// Where Back leads from the current page: the shape we came through, or else the first
    /// shape anywhere that links here.
    pub fn parent_link(&self) -> Option<(usize, Id)> {
        let here = self.page().id;
        if let Some(&(pid, sid)) = self.nav.last() {
            if let Some(i) = self.doc.page_index(pid) {
                if self.doc.pages[i].shape(sid).map_or(false, |s| s.link == Some(here)) { return Some((i, sid)); }
            }
        }
        self.doc.parents_of(here).into_iter().next()
    }

    /// Return to the parent page, with the shape we drilled into selected and in view.
    pub fn go_back(&mut self) -> bool {
        let Some((i, sid)) = self.parent_link() else { return false };
        if let Some(&(pid, _)) = self.nav.last() { if Some(pid) == self.doc.pages.get(i).map(|p| p.id) { self.nav.pop(); } }
        self.go_page(i);
        self.sel = vec![sid];
        self.focus_shape = Some(sid);
        true
    }

    /// The chain of parent pages above the current one, outermost first: (page index, shape id).
    pub fn breadcrumbs(&self) -> Vec<(usize, Id)> {
        let mut out = vec![];
        let mut seen = vec![self.pi];
        let mut cur = self.parent_link();
        while let Some((i, sid)) = cur {
            if seen.contains(&i) || out.len() > 8 { break; }
            seen.push(i);
            out.push((i, sid));
            let pid = self.doc.pages[i].id;
            cur = self.doc.parents_of(pid).into_iter().find(|(j, _)| !seen.contains(j));
        }
        out.reverse();
        out
    }

    /// Jump to a page from the breadcrumb bar, selecting the shape that leads onward.
    pub fn go_crumb(&mut self, i: usize, sid: Id) {
        let pid = self.doc.pages[i].id;
        while let Some(&(p, _)) = self.nav.last() { self.nav.pop(); if p == pid { break; } }
        self.go_page(i);
        self.sel = vec![sid];
        self.focus_shape = Some(sid);
    }

    /// Move the selected shapes (and the connectors between them) onto a new page, replacing them
    /// with one Subprocess shape that links to it. Connectors from the rest of the diagram are
    /// re-attached to that shape.
    pub fn subpage_from_selection(&mut self) {
        let ids: Vec<Id> = self.sel_shapes().iter().filter(|s| !s.locked).map(|s| s.id).collect();
        if ids.is_empty() { self.notify("Select the shapes to move into a sub-diagram"); return; }
        let page = self.page().clone();
        let mut bb: Option<Rect> = None;
        for s in page.shapes.iter().filter(|s| ids.contains(&s.id)) { bb = geom::rect_union(bb, geom::bbox(s)); }
        let Some(bb) = bb else { return };
        self.checkpoint();
        let inside = |e: &End| matches!(e, End::Glued { shape, .. } if ids.contains(shape));
        let sel_conn = |c: &Conn| self.sel.contains(&c.id);
        // connectors that move: both ends inside, or one end inside and the other loose
        let moves = |c: &Conn| (inside(&c.from) || matches!(c.from, End::Free { .. }) && sel_conn(c)) && (inside(&c.to) || matches!(c.to, End::Free { .. }) && sel_conn(c)) && (inside(&c.from) || inside(&c.to));
        let moved_conns: Vec<Conn> = page.conns.iter().filter(|c| moves(c)).cloned().collect();
        let moved_ids: Vec<Id> = moved_conns.iter().map(|c| c.id).collect();
        let shapes: Vec<Shape> = page.shapes.iter().filter(|s| ids.contains(&s.id)).cloned().collect();

        // the new page: content moved to a 40px margin, page sized to fit
        let name = self.unique_page_name(shapes.iter().find(|s| !s.text.trim().is_empty()).map(|s| s.text.as_str()).unwrap_or("Sub-process"));
        let pid = self.doc.new_id();
        let (dx, dy) = (40.0 - bb.min.x, 40.0 - bb.min.y);
        let mut np = Page { id: pid, name: name.clone(), width: (bb.width() + 80.0).max(page.width.min(1400.0)), height: (bb.height() + 80.0).max(page.height.min(990.0)), line_jumps: page.line_jumps, avoid_shapes: page.avoid_shapes, ..Default::default() };
        for mut s in shapes { s.x += dx; s.y += dy; s.layer = None; np.shapes.push(s); }
        for mut c in moved_conns {
            for e in [&mut c.from, &mut c.to] { if let End::Free { x, y } = e { *x += dx; *y += dy; } }
            c.translate_bends(dx, dy);
            c.layer = None;
            np.conns.push(c);
        }

        // the stand-in shape goes where the flow enters and leaves the selection (the centre of the
        // shapes with outside connections), so the remaining connectors stay straight
        let crossing: Vec<Id> = page.conns.iter().filter(|c| !moved_ids.contains(&c.id)).flat_map(|c| [c.from, c.to]).filter_map(|e| match e { End::Glued { shape, .. } if ids.contains(&shape) => Some(shape), _ => None }).collect();
        let mut anchor: Option<Rect> = None;
        for s in page.shapes.iter().filter(|s| crossing.contains(&s.id)) { anchor = geom::rect_union(anchor, geom::bbox(s)); }
        let at = anchor.map_or(bb.center(), |r| r.center());
        let mut sub = self.make_shape("fc_predefined", at);
        sub.text = name.clone();
        sub.link = Some(pid);
        // as big as the selection was (within reason), but never on top of the shapes around it
        let others: Vec<Rect> = page.shapes.iter().filter(|s| !ids.contains(&s.id)).map(|s| geom::bbox(s).expand(16.0)).collect();
        let want = (bb.width().clamp(140.0, 220.0), bb.height().clamp(90.0, 140.0));
        let sizes = [want, (want.0.min(170.0), want.1.min(110.0)), (140.0, 90.0), (120.0, 72.0)];
        let fits = |(w, h): (f32, f32)| { let r = Rect::from_center_size(at, egui::vec2(w, h)); !others.iter().any(|o| o.intersects(r)) };
        let (sw, sh) = sizes.iter().copied().find(|sz| fits(*sz)).unwrap_or((120.0, 72.0));
        sub.w = sw;
        sub.h = sh;
        sub.x = (at.x - sw / 2.0).round();
        sub.y = (at.y - sh / 2.0).round();
        let sub_id = sub.id;
        let p = self.page_mut();
        p.shapes.retain(|s| !ids.contains(&s.id));
        p.conns.retain(|c| !moved_ids.contains(&c.id));
        for c in p.conns.iter_mut() {
            for e in [&mut c.from, &mut c.to] {
                if let End::Glued { shape, .. } = e { if ids.contains(shape) { *e = End::Glued { shape: sub_id, port: -1 }; c.bends = None; } }
            }
        }
        p.shapes.push(sub);
        let at = self.pi + 1;
        self.doc.pages.insert(at, np);
        self.sel = vec![sub_id];
        self.notify(crate::i18n::trf("Moved {n} shape(s) to “{name}”", &[("n", &ids.len().to_string()), ("name", &name)]));
    }

    /// Links pointing at a page that is about to go away are cleared.
    pub fn unlink_page(&mut self, page_id: Id) {
        for p in self.doc.pages.iter_mut() { for s in p.shapes.iter_mut() { if s.link == Some(page_id) { s.link = None; } } }
        self.nav.retain(|(p, _)| *p != page_id);
    }

    /// World position of the open-link badge of a linked shape.
    pub fn link_badge_pos(s: &Shape, zoom: f32) -> Pos2 {
        let r = geom::bbox(s);
        let inset = (13.0 / zoom).min(r.width() / 3.0).min(r.height() / 3.0);
        pos2(r.max.x - inset, r.min.y + inset)
    }
}

