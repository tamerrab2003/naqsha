//! Several document windows in one app: the first is eframe's root window, the others are
//! native windows (egui viewports). Each window is an independent `App` (its own document,
//! undo history and selection); settings are shared.
use crate::app::{App, Pending, Settings, Spawn};
use crate::{model, platform};
use egui::{Color32, ViewportBuilder, ViewportId};

pub struct Shell {
    root: App,
    others: Vec<(ViewportId, App)>,
    set: Settings,
    next: u64,
    /// Index of the window that had focus last (0 = root, i+1 = others[i]).
    focused: usize,
    pub native_menu: Option<Box<dyn std::any::Any>>,
    /// Where each extra window first appears (cascaded from the window in front).
    placed: std::collections::HashMap<ViewportId, egui::Pos2>,
}

pub fn builder(title: &str) -> ViewportBuilder {
    let mut vp = ViewportBuilder::default()
        .with_title(title)
        .with_app_id("com.nixpx.naqsha")
        .with_inner_size([1360.0, 860.0])
        .with_min_inner_size([980.0, 620.0])
        .with_icon(crate::icon());
    if cfg!(target_os = "macos") {
        vp = vp.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    vp
}

impl Shell {
    pub fn new(root: App, set: Settings) -> Shell {
        Shell { root, others: vec![], set, next: 1, focused: 0, native_menu: None, placed: Default::default() }
    }

    fn new_window(&mut self) -> &mut App {
        let id = ViewportId::from_hash_of(("naqsha-window", self.next));
        self.next += 1;
        let app = App::new(self.set.clone(), None);
        // cascade: 28px down and right of the window in front
        if let Some(ctx) = platform::CTX.get() {
            let front = if self.focused == 0 || self.focused > self.others.len() { ViewportId::ROOT } else { self.others[self.focused - 1].0 };
            let at = ctx.input_for(front, |i| i.viewport().outer_rect.map(|r| r.min))
                .or_else(|| self.placed.get(&front).copied());
            if let Some(p) = at { self.placed.insert(id, p + egui::vec2(28.0, 28.0)); }
        }
        self.others.push((id, app));
        // a new window takes the focus for menu commands until the OS says otherwise
        self.focused = self.others.len();
        &mut self.others.last_mut().unwrap().1
    }

    /// Open an unsaved document recovered from last session in a window of its own.
    pub fn recover(&mut self, doc: model::Document) {
        if doc.pages.iter().all(|p| p.shapes.is_empty() && p.conns.is_empty()) { return; }
        let w = self.new_window();
        w.adopt(doc, "");
        w.dirty = true;
        w.notify("Recovered an unsaved diagram from last session");
    }

    fn window(&mut self, i: usize) -> &mut App { if i == 0 { &mut self.root } else { &mut self.others[i - 1].1 } }
    fn count(&self) -> usize { 1 + self.others.len() }

    fn spawn(&mut self, reqs: Vec<Spawn>) {
        for r in reqs {
            match r {
                // a browser tab holds one diagram: another one opens in a new tab
                #[cfg(target_arch = "wasm32")]
                Spawn::Doc(doc, name) => {
                    // embedded in a document there is only this one diagram: the new one replaces it
                    if crate::embed::active() { self.root.adopt(doc, &name); continue; }
                    if !crate::files::web::open_in_new_tab(&doc, &name) {
                        let w = self.window(self.focused.min(self.count() - 1));
                        if w.dirty { w.notify("Your browser blocked the new tab — allow pop-ups for this site, or Save (download) this diagram first"); }
                        else { w.adopt(doc, &name); }
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                Spawn::Doc(doc, name) => { self.new_window().adopt(doc, &name); }
                Spawn::Path(p) => {
                    // already open somewhere? bring that window forward instead
                    if let Some(i) = (0..self.count()).find(|i| { let w = if *i == 0 { &self.root } else { &self.others[*i - 1].1 }; w.path.as_ref() == Some(&p) }) {
                        let vid = if i == 0 { ViewportId::ROOT } else { self.others[i - 1].0 };
                        if let Some(ctx) = platform::CTX.get() { ctx.send_viewport_cmd_to(vid, egui::ViewportCommand::Focus); }
                        continue;
                    }
                    self.new_window().load_path(p);
                }
            }
        }
    }

    fn sync_settings(set: &mut Settings, app: &mut App, before: &Settings) {
        // whatever this window changed becomes the shared settings
        if app.set.stencil_dirs != before.stencil_dirs { app.user_lib_loaded = false; }
        *set = app.set.clone();
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // ---- OS integration, routed to the window in front
        let any_dirty = self.root.dirty || self.others.iter().any(|(_, a)| a.dirty);
        platform::set_dirty(any_dirty);
        let target = self.focused.min(self.count() - 1);
        for p in platform::take_open() {
            let w = self.window(target);
            w.recovery = None;
            w.open_or_spawn(p);
        }
        if platform::take_quit() {
            // every window asks about its own unsaved changes; the app ends when all have closed
            self.root.guard(Pending::Close);
            for (_, a) in self.others.iter_mut() { a.guard(Pending::Close); }
        }
        let menu = platform::take_menu_events();
        if !menu.is_empty() { self.window(target).menu_queue.extend(menu); }

        // ---- the root window
        let before = self.set.clone();
        self.root.set = self.set.clone();
        self.root.frame(ui);
        Shell::sync_settings(&mut self.set, &mut self.root, &before);

        // ---- the other windows
        let set = &mut self.set;
        let placed = &self.placed;
        for (vid, app) in self.others.iter_mut() {
            let title = app.last_title.clone();
            let mut b = builder(if title.is_empty() { "Naqsha" } else { &title });
            if let Some(p) = placed.get(vid) { b = b.with_position(*p); }
            ctx.show_viewport_immediate(*vid, b, |ui, _class| {
                let before = set.clone();
                app.set = set.clone();
                app.frame(ui);
                Shell::sync_settings(set, app, &before);
            });
        }

        // ---- focus, new windows, closed windows
        if self.root.focused { self.focused = 0; }
        for (i, (_, a)) in self.others.iter().enumerate() { if a.focused { self.focused = i + 1; } }
        let mut reqs = std::mem::take(&mut self.root.spawn);
        for (_, a) in self.others.iter_mut() { reqs.extend(std::mem::take(&mut a.spawn)); }
        self.spawn(reqs);

        let before = self.others.len();
        self.others.retain(|(_, a)| !a.want_close);
        let alive: Vec<ViewportId> = self.others.iter().map(|(v, _)| *v).collect();
        self.placed.retain(|v, _| alive.contains(v));
        if self.others.len() != before { self.focused = 0; }
        if self.root.want_close {
            if self.others.is_empty() {
                // last window: quit (unless the OS is already closing it)
                if !ctx.input(|i| i.viewport().close_requested()) { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            } else {
                // keep the app running: the next window's document moves into the root window
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                let (vid, mut next) = self.others.remove(0);
                // ...and the root window takes that window's place, so it looks like the one you
                // closed went away and the other stayed
                let (pos, size) = ctx.input_for(vid, |i| (i.viewport().outer_rect.map(|r| r.min), i.viewport().inner_rect.map(|r| r.size())));
                if let Some(p) = pos { ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(p)); }
                if let Some(s) = size { ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(s)); }
                next.last_title.clear();
                next.fit_pending = true;
                self.root = next;
                self.focused = 0;
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.set);
        // an embedded editor's diagram lives in the host document: leave the app's own recovery alone
        if crate::embed::active() { return; }
        let mut rec: Vec<model::Document> = vec![];
        for a in std::iter::once(&self.root).chain(self.others.iter().map(|(_, a)| a)) {
            if a.dirty && a.path.is_none() { rec.push(a.doc.clone()); }
        }
        eframe::set_value(storage, "recovery_all", &rec);
        eframe::set_value::<Option<model::Document>>(storage, "recovery", &None);
    }

    fn auto_save_interval(&self) -> std::time::Duration { std::time::Duration::from_secs(20) }

    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] { Color32::from(self.root.pal.bg).to_normalized_gamma_f32() }
}
