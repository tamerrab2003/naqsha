//! Naqsha — a native diagramming app (Visio-style) written in Rust.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// the browser build leaves out the desktop-only parts (command line, menus, files on disk)
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports, unused_variables, unused_mut))]

mod anim;
mod app;
mod canvas;
mod embed;
mod export;
mod files;
mod formats;
mod geom;
mod icons;
mod layout;
mod library;
#[cfg(target_os = "macos")]
mod macos;
mod platform;
// Naqsha Pro: the closed-source module in pro/ (official builds), or a stand-in that switches it off
#[cfg(feature = "pro")]
#[path = "../pro/src/mod.rs"]
mod pro;
#[cfg(not(feature = "pro"))]
#[path = "pro_stub.rs"]
mod pro;
mod model;
mod render;
mod route;
mod shell;
mod subpage;
mod templates;
mod theme;
mod userlib;
mod ui;

use app::{App, Mode, Pending, Settings};
use egui::{ColorImage, TextureOptions};

impl App {
    fn sync_theme(&mut self, ctx: &egui::Context) {
        let dark = match self.set.mode {
            Mode::Dark => true,
            Mode::Light => false,
            Mode::System => ctx.system_theme().map_or(false, |t| t == egui::Theme::Dark),
        };
        if self.pal.dark != dark || self.last_title.is_empty() {
            self.pal = if dark { theme::DARK } else { theme::LIGHT };
            ctx.set_visuals(theme::visuals(&self.pal));
        }
    }

    fn sync_images(&mut self, ctx: &egui::Context) {
        use base64::Engine;
        let mut wanted = vec![];
        for p in &self.doc.pages {
            for s in &p.shapes {
                if let Some(data) = &s.image {
                    let stale = self.images.get(&s.id).map_or(true, |(len, _)| *len != data.len());
                    if stale { wanted.push((s.id, data.clone())); }
                }
            }
        }
        for (id, data) in wanted {
            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data.as_bytes()) else { continue };
            let Ok(img) = image::load_from_memory(&bytes) else { continue };
            let rgba = img.to_rgba8();
            let ci = ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
            let tex = ctx.load_texture(format!("img-{id}"), ci, TextureOptions::LINEAR);
            self.images.insert(id, (data.len(), tex));
        }
    }
}

impl App {
    /// One frame of a document window (root or extra).
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.now = ctx.input(|i| i.time);
        self.focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        self.sync_theme(&ctx);
        self.sync_images(&ctx);

        // window title + close guard
        let title = format!("{}{} — Naqsha", self.title(), if self.dirty { " •" } else { "" });
        if title != self.last_title {
            #[cfg(target_arch = "wasm32")]
            files::web::set_title(&title);
            #[cfg(not(target_arch = "wasm32"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.dirty && !self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.modal = Some(app::Modal::Unsaved(Pending::Close));
            } else {
                self.want_close = true;
            }
        }
        if self.allow_close && self.modal.is_none() { self.want_close = true; }
        // files dropped onto the window: diagrams, Visio/draw.io/SVG files and pipeline definitions
        // in the browser a dropped file is read asynchronously, and has no path
        #[cfg(target_arch = "wasm32")]
        for f in ctx.input(|i| i.raw.dropped_files.clone()) { files::read_dropped(f); }
        #[cfg(not(target_arch = "wasm32"))]
        for p in ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect::<Vec<_>>()) {
            let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let known = matches!(ext.as_str(), "nqs" | "schx" | "json" | "vsdx" | "drawio" | "xml" | "svg") || name.starts_with("jenkinsfile")
                || (matches!(ext.as_str(), "yml" | "yaml" | "groovy") && std::fs::read_to_string(&p).map_or(false, |t| formats::pipeline::detect(&name, &t)));
            if known { self.open_or_spawn(p); }
            else if matches!(ext.as_str(), "png" | "jpg" | "jpeg") { self.notify("Use Insert › Picture to add images"); }
        }
        self.handle_picked();
        pro::frame(self, &ctx);
        #[cfg(target_arch = "wasm32")]
        crate::embed::frame(self, ui);
        for id in std::mem::take(&mut self.menu_queue) { ui::menu_command(self, ui, &id); }

        ui::shortcuts(self, &ctx);
        ui::top(self, ui);
        ui::bottom(self, ui);
        ui::left(self, ui);
        ui::right(self, ui);
        egui::CentralPanel::no_frame().show(ui, |ui| canvas::canvas(self, ui));
        ui::stencil_ghost(self, &ctx);
        if ctx.input(|i| i.pointer.any_released()) && !matches!(self.stencil_drag, None) && !self.canvas_rect.contains(ctx.pointer_latest_pos().unwrap_or_default()) {
            self.stencil_drag = None;
        }
        ui::context_menu(self, &ctx);
        ui::modals(self, &ctx);
        ui::toast(self, &ctx);
        if self.now < 0.6 || self.path.is_some() || self.imported_name.is_some() {
            // give launch-time "open file" events a moment before offering recovered work
        } else if let Some(doc) = self.recovery.take() {
            if !doc.pages.iter().all(|p| p.shapes.is_empty() && p.conns.is_empty()) {
                self.doc = doc;
                self.doc.fix_ids();
                self.dirty = true;
                self.fit_pending = true;
                self.notify("Recovered your unsaved diagram from last session");
            }
        }
        // launched without a file (and nothing recovered): start by choosing a template
        if self.offer_gallery && self.now < 0.6 {
            // idle frames don't come by themselves: wake up once the launch has settled
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
        if self.offer_gallery && self.now >= 0.6 {
            self.offer_gallery = false;
            if self.is_pristine() && self.modal.is_none() && self.recovery.is_none() {
                self.gallery.cat = 0;
                self.gallery.search.clear();
                self.modal = Some(app::Modal::NewDiagram);
                ctx.request_repaint();
            }
        }
    }
}

fn icon() -> egui::IconData {
    // the same artwork as the Dock / Finder icon (assets/icon.svg, rendered by examples/make_icon)
    let img = image::load_from_memory(include_bytes!("../assets/icon-512.png")).map(|i| i.to_rgba8());
    match img {
        Ok(i) => egui::IconData { width: i.width(), height: i.height(), rgba: i.into_raw() },
        Err(_) => egui::IconData::default(),
    }
}

/// `naqsha --export diagram.nqs out.png|svg|pdf|gif [page-number] [--dark]`
fn cli_export(args: &[String]) -> Result<(), String> {
    let input = args.get(2).ok_or("missing input file")?;
    let output = args.get(3).ok_or("missing output file")?;
    let page_no: usize = args.iter().skip(4).find_map(|a| a.parse().ok()).unwrap_or(1);
    let dark = args.iter().any(|a| a == "--dark");
    let (doc, notes, _) = app::read_any(std::path::Path::new(input))?;
    for n in notes { eprintln!("note: {n}"); }
    let page = doc.pages.get(page_no.saturating_sub(1)).ok_or("no such page")?;
    let ctx = egui::Context::default();
    ctx.set_fonts(theme::fonts());
    let pal = if dark { theme::DARK } else { theme::LIGHT };
    let ext = output.rsplit('.').next().unwrap_or("");
    let moves = anim::page_moves(page);
    let mut svg = String::new();
    let mut gif = Ok(vec![]);
    for pass in 0..2 {
        // first pass loads fonts, second lays out text
        let _ = ctx.run_ui(Default::default(), |ui| {
            svg = if ext == "svg" && moves { export::page_svg_with(Some(&doc), page, &pal, ui.painter(), export::content_bounds(page, 24.0), export::Motion::Css).0 }
                else { export::page_svg(Some(&doc), page, &pal, ui.painter(), 24.0).0 };
            if ext == "gif" && pass == 1 { gif = export::page_gif(Some(&doc), page, &pal, ui.painter(), 24.0, 1.0); }
        });
    }
    let bytes = match ext {
        "svg" => svg.into_bytes(),
        "gif" => gif?,
        "png" => export::svg_to_png(&svg, 2.0)?,
        "pdf" => export::svg_to_pdf(&svg)?,
        "vsdx" | "drawio" | "nqs" | "schx" => return app::write_foreign(&doc, std::path::Path::new(output)),
        e => return Err(format!("unknown format: {e}")),
    };
    std::fs::write(output, bytes).map_err(|e| e.to_string())
}

/// The app was called Schemix before: carry its saved settings (recent files, stencil folders,
/// theme, unsaved-diagram recovery) over to the new name once.
#[cfg(not(target_arch = "wasm32"))]
fn migrate_old_settings() {
    let (Some(new), Some(old)) = (eframe::storage_dir("com.nixpx.naqsha"), eframe::storage_dir("com.nixpx.schemix")) else { return };
    let (nf, of) = (new.join("app.ron"), old.join("app.ron"));
    if nf.exists() || !of.exists() { return; }
    if std::fs::create_dir_all(&new).is_ok() {
        if let Ok(t) = std::fs::read_to_string(&of) {
            // stored file paths and names keep working; only the app's own name changes
            let _ = std::fs::write(&nf, t.replace("com.nixpx.schemix", "com.nixpx.naqsha"));
        }
    }
}

/// Write panic messages to a log file: macOS crash reports don't include them.
fn install_crash_log() {
    let dir = platform::log_dir();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = &dir {
            let _ = std::fs::create_dir_all(dir);
            let bt = std::backtrace::Backtrace::force_capture();
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let msg = format!("--- Naqsha {} crash at unix time {secs}\n{info}\n{bt}\n", env!("CARGO_PKG_VERSION"));
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("crash.log")) { let _ = f.write_all(msg.as_bytes()); }
        }
        default(info);
    }));
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    install_crash_log();
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map_or(false, |a| a.starts_with("--")) { platform::attach_console(); }
    if args.get(1).map(|a| a == "--arrange").unwrap_or(false) {
        // naqsha --arrange tb|lr|grid in.(nqs|vsdx|drawio) out.(nqs|vsdx|drawio|png|svg|pdf) [page]
        let run = || -> Result<(), String> {
            let mode = match args.get(2).map(|s| s.as_str()) { Some("lr") => layout::Mode::LeftRight, Some("grid") => layout::Mode::Grid, _ => layout::Mode::TopDown };
            let input = args.get(3).ok_or("missing input file")?;
            let output = args.get(4).ok_or("missing output file")?;
            let page_no: usize = args.get(5).and_then(|a| a.parse().ok()).unwrap_or(1);
            let (mut doc, _, _) = app::read_any(std::path::Path::new(input))?;
            let page = doc.pages.get_mut(page_no.saturating_sub(1)).ok_or("no such page")?;
            eprintln!("{}", layout::auto_arrange(page, &[], mode)?);
            let tmp = std::env::temp_dir().join(format!("naqsha-arrange-{}.nqs", std::process::id()));
            app::write_foreign(&doc, &tmp)?;
            let ext = output.rsplit('.').next().unwrap_or("");
            let r = if matches!(ext, "png" | "svg" | "pdf") {
                cli_export(&["".into(), "--export".into(), tmp.to_string_lossy().into(), output.clone(), page_no.to_string()])
            } else { app::write_foreign(&doc, std::path::Path::new(output)) };
            let _ = std::fs::remove_file(&tmp);
            r
        };
        if let Err(e) = run() { eprintln!("naqsha: {e}"); std::process::exit(1); }
        return Ok(());
    }
    if args.get(1).map(|a| a == "--templates").unwrap_or(false) {
        // naqsha --templates out-dir : write every built-in template as a .nqs (for review)
        let dir = std::path::PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".into()));
        let _ = std::fs::create_dir_all(&dir);
        for e in templates::builtin() { let _ = app::write_foreign(&e.doc, &dir.join(format!("{}.nqs", e.id))); println!("{}", e.id); }
        return Ok(());
    }
    if args.get(1).map(|a| a == "--export").unwrap_or(false) {
        if let Err(e) = cli_export(&args) { eprintln!("naqsha: {e}"); std::process::exit(1); }
        return Ok(());
    }
    let start_file = std::env::args().skip(1).find(|a| !a.starts_with("-psn")).map(std::path::PathBuf::from).filter(|p| p.exists());
    let mut vp = egui::ViewportBuilder::default()
        .with_title("Naqsha")
        .with_app_id("com.nixpx.naqsha")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([980.0, 620.0])
        .with_icon(icon());
    if cfg!(target_os = "macos") {
        vp = vp.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    migrate_old_settings();
    // Own the event loop so OS hooks (macOS app delegate) are in place before it starts running.
    let event_loop = winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build().map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    platform::before_event_loop();
    let mut winit_app = eframe::create_native(
        "Naqsha",
        options,
        Box::new(move |cc| Ok(Box::new(setup(cc, start_file.map(Start::File).unwrap_or(Start::Nothing))))),
        &event_loop,
    );
    event_loop.run_app(&mut winit_app).map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    Ok(())
}

/// What a window starts with.
#[allow(dead_code)]
enum Start {
    Nothing,
    /// A file on disk (desktop: command line, Finder).
    File(std::path::PathBuf),
    /// A document handed over by another browser tab, or a template from the page URL.
    Doc(model::Document, String),
}

/// Fonts, style, settings and the first window — shared by the desktop and browser builds.
fn setup(cc: &eframe::CreationContext<'_>, start: Start) -> shell::Shell {
    let _ = platform::CTX.set(cc.egui_ctx.clone());
    cc.egui_ctx.set_fonts(theme::fonts());
    icons::init(&cc.egui_ctx);
    cc.egui_ctx.all_styles_mut(|s| {
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.interact_size.y = 24.0;
        s.interaction.tooltip_delay = 0.4;
    });
    let set: Settings = cc.storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default();
    // unsaved untitled diagrams from last session (one per window)
    let mut recovered: Vec<model::Document> = if matches!(start, Start::Nothing) {
        cc.storage.and_then(|s| eframe::get_value::<Vec<model::Document>>(s, "recovery_all"))
            .or_else(|| cc.storage.and_then(|s| eframe::get_value::<Option<model::Document>>(s, "recovery")).flatten().map(|d| vec![d]))
            .unwrap_or_default()
    } else { vec![] };
    // a browser tab is one window: it recovers one diagram
    if files::WEB { recovered.truncate(1); }
    if embed::active() { recovered.clear(); }
    let first = if recovered.is_empty() { None } else { Some(recovered.remove(0)) };
    let recent = set.recent.clone();
    let mut app = App::new(set.clone(), first);
    app.offer_gallery = matches!(start, Start::Nothing);
    match start {
        Start::File(p) => app.load_path(p),
        Start::Doc(d, name) => app.adopt(d, &name),
        Start::Nothing => {}
    }
    let mut sh = shell::Shell::new(app, set);
    sh.native_menu = platform::install_menu(&recent);
    for d in recovered { sh.recover(d); }
    sh
}

/// The browser version: draws into the page's canvas.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    eframe::WebLogger::init(log::LevelFilter::Error).ok();
    // embedded in another app, the host document keeps the diagram
    if !embed::active() { files::web::install_unload_guard(); }
    embed::install();
    let start = match files::web::start_document() { Some((d, name)) => Start::Doc(d, name), None => Start::Nothing };
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
        let Some(canvas) = document.get_element_by_id("naqsha_canvas").and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
        let r = eframe::WebRunner::new().start(canvas, eframe::WebOptions::default(), Box::new(move |cc| Ok(Box::new(setup(cc, start))))).await;
        if let Some(el) = document.get_element_by_id("loading") {
            match r {
                Ok(_) => el.remove(),
                Err(e) => el.set_inner_html(&format!("<p>Naqsha couldn't start in this browser: {e:?}</p><p>It needs WebGL. Try a current Chrome, Edge, Firefox or Safari.</p>")),
            }
        }
    });
}

#[cfg(test)]
mod model_tests {
    use crate::app::{App, Settings};
    use crate::model::*;
    use egui::{pos2, vec2};

    #[test]
    fn sample_roundtrips() {
        let t = std::fs::read_to_string("examples/sample.nqs").unwrap();
        let d: Document = serde_json::from_str(&t).unwrap();
        assert_eq!(d.pages.len(), 3);
        let again: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, again);
    }

    #[test]
    fn edit_undo_redo_autoconnect() {
        let mut a = App::new(Settings::default(), None);
        let id = a.add_shape("fc_process", pos2(200.0, 200.0));
        a.auto_connect(id, vec2(1.0, 0.0));
        assert_eq!(a.page().shapes.len(), 2);
        assert_eq!(a.page().conns.len(), 1);
        let c = &a.page().conns[0];
        let route = crate::geom::route(a.page(), c);
        assert!(route.len() >= 2);
        // glued right port of source to left port of target
        assert!((route[0].x - (a.page().shapes[0].x + a.page().shapes[0].w)).abs() < 0.5);
        a.select_all();
        a.delete_sel();
        assert!(a.page().shapes.is_empty() && a.page().conns.is_empty());
        a.do_undo();
        assert_eq!(a.page().shapes.len(), 2);
        a.do_redo();
        assert!(a.page().shapes.is_empty());
    }

    #[test]
    fn copy_paste_remaps_connector_ends() {
        let mut a = App::new(Settings::default(), None);
        let id = a.add_shape("rect", pos2(100.0, 100.0));
        a.auto_connect(id, vec2(0.0, 1.0));
        a.select_all();
        let clip = a.clip_json().unwrap();
        a.paste_str(&clip);
        assert_eq!(a.page().shapes.len(), 4);
        assert_eq!(a.page().conns.len(), 2);
        let ids: Vec<Id> = a.page().shapes.iter().map(|s| s.id).collect();
        for c in &a.page().conns {
            for e in [c.from, c.to] { if let End::Glued { shape, .. } = e { assert!(ids.contains(&shape)); } }
        }
        let pasted = &a.page().conns[1];
        assert!(matches!(pasted.from, End::Glued { shape, .. } if shape != id));
    }

    fn sample() -> Document {
        let t = std::fs::read_to_string("examples/sample.nqs").unwrap();
        let mut d: Document = serde_json::from_str(&t).unwrap();
        let lid = d.new_id();
        d.pages[0].layers.push(Layer { id: lid, name: "Notes".into(), ..Default::default() });
        d.pages[0].shapes[0].layer = Some(lid);
        d
    }

    #[test]
    fn files_and_clipboard_from_before_the_rename_still_work() {
        // a diagram saved as .schx opens as a native document
        let t = std::fs::read_to_string("examples/sample.nqs").unwrap();
        let p = std::env::temp_dir().join(format!("naqsha-old-{}.schx", std::process::id()));
        std::fs::write(&p, &t).unwrap();
        let (d, _, native) = crate::app::read_any(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert!(native && d.pages.len() == 3);
        // shapes copied in the old app paste into the new one
        let mut a = App::new(Settings::default(), None);
        a.add_shape("rect", pos2(100.0, 100.0));
        a.select_all();
        let clip = a.clip_json().unwrap();
        assert!(clip.starts_with(crate::app::CLIP_TAG));
        let old = clip.replacen(crate::app::CLIP_TAG, crate::app::OLD_CLIP_TAG, 1);
        a.paste_str(&old);
        assert_eq!(a.page().shapes.len(), 2);
        // new documents say who wrote them
        assert_eq!(crate::model::Document::default().app, "Naqsha");
    }

    #[test]
    fn subdiagram_from_selection_and_navigation() {
        let mut a = App::new(Settings::default(), None);
        let s1 = a.add_shape("fc_terminator", pos2(100.0, 300.0));
        let s2 = a.add_shape("fc_process", pos2(300.0, 300.0));
        let s3 = a.add_shape("fc_process", pos2(500.0, 300.0));
        let s4 = a.add_shape("fc_terminator", pos2(700.0, 300.0));
        let mut ids = vec![];
        for (f, t) in [(s1, s2), (s2, s3), (s3, s4)] {
            let id = a.doc.new_id();
            ids.push(id);
            a.page_mut().conns.push(Conn { id, from: End::Glued { shape: f, port: -1 }, to: End::Glued { shape: t, port: -1 }, ..Default::default() });
        }
        a.sel = vec![s2, s3];
        a.subpage_from_selection();
        assert_eq!(a.doc.pages.len(), 2);
        // parent: start, end and the new subprocess; outer connectors re-attached to it
        let sub = a.sel[0];
        let p0 = a.page();
        assert_eq!(p0.shapes.len(), 3);
        assert!(p0.shape(sub).unwrap().link == Some(a.doc.pages[1].id));
        assert_eq!(p0.conns.len(), 2);
        assert!(p0.conns.iter().all(|c| matches!(c.from, End::Glued { shape, .. } if shape == sub) || matches!(c.to, End::Glued { shape, .. } if shape == sub)));
        // child: the two processes and the connector between them
        let p1 = &a.doc.pages[1];
        assert_eq!(p1.shapes.len(), 2);
        assert_eq!(p1.conns.len(), 1);
        assert_eq!(p1.conns[0].id, ids[1]);
        // drill in and back out
        assert!(a.open_link(sub));
        assert_eq!(a.pi, 1);
        assert_eq!(a.breadcrumbs().len(), 1);
        assert!(a.go_back());
        assert_eq!(a.pi, 0);
        assert_eq!(a.sel, vec![sub]);
        // back works without history too (page reached from the tabs)
        a.nav.clear();
        a.go_page(1);
        assert_eq!(a.parent_link().map(|x| x.0), Some(0));
        // a nested sub-diagram gives a two-level trail
        let inner = a.page().shapes[0].id;
        a.create_subpage(inner);
        assert_eq!(a.pi, 2);
        assert_eq!(a.breadcrumbs().iter().map(|b| b.0).collect::<Vec<_>>(), vec![0, 1]);
        // deleting a page clears links to it
        a.del_page();
        assert!(a.doc.pages.iter().all(|p| p.shapes.iter().all(|s| s.link.map_or(true, |l| a.doc.page_by_id(l).is_some()))));
    }

    #[test]
    fn subdiagram_links_survive_visio_and_drawio() {
        let mut a = App::new(Settings::default(), None);
        let s = a.add_shape("fc_predefined", pos2(200.0, 200.0));
        a.page_mut().shape_mut(s).unwrap().text = "Payment".into();
        a.create_subpage(s);
        a.add_shape("rect", pos2(100.0, 100.0));
        let check = |d: &Document| {
            assert_eq!(d.pages.len(), 2);
            let sh = d.pages[0].shapes.iter().find(|x| x.text.contains("Payment")).expect("linked shape");
            assert_eq!(sh.link, Some(d.pages[1].id), "link lost");
        };
        check(&crate::formats::vsdx::import(crate::formats::vsdx::export(&a.doc).unwrap()).unwrap().doc);
        check(&crate::formats::drawio::import(&crate::formats::drawio::export(&a.doc)).unwrap().doc);
        // SVG export draws the preview inside the shape
        let ctx = egui::Context::default();
        let mut svg = String::new();
        for _ in 0..2 { let _ = ctx.run_ui(Default::default(), |ui| { svg = crate::export::page_svg(Some(&a.doc), &a.doc.pages[0], &crate::theme::LIGHT, ui.painter(), 24.0).0; }); }
        assert!(svg.contains(") scale("), "no preview group in SVG");
    }

    #[test]
    fn vsdx_roundtrip_keeps_structure() {
        let d = sample();
        let bytes = crate::formats::vsdx::export(&d).unwrap();
        let back = crate::formats::vsdx::import(bytes).unwrap().doc;
        assert_eq!(back.pages.len(), d.pages.len());
        for (a, b) in d.pages.iter().zip(&back.pages) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.shapes.len(), b.shapes.len(), "shape count on {}", a.name);
            assert_eq!(a.conns.len(), b.conns.len(), "connector count on {}", a.name);
            let glued = |p: &Page| p.conns.iter().filter(|c| matches!((c.from, c.to), (End::Glued { .. }, End::Glued { .. }))).count();
            assert_eq!(glued(a), glued(b), "glue on {}", a.name);
            let mut ta: Vec<&str> = a.shapes.iter().map(|s| s.text.as_str()).filter(|t| !t.contains("--")).collect();
            let mut tb: Vec<&str> = b.shapes.iter().map(|s| s.text.as_str()).filter(|t| !t.contains('\n') || !t.is_empty()).collect();
            ta.sort(); tb.sort();
            for t in ta { assert!(tb.contains(&t), "text {t:?} lost"); }
            // geometry within a pixel
            for (sa, sb) in a.shapes.iter().zip(&b.shapes) {
                assert!((sa.x - sb.x).abs() < 1.0 && (sa.y - sb.y).abs() < 1.0 && (sa.w - sb.w).abs() < 1.0, "{} moved", sa.text);
            }
        }
        assert_eq!(back.pages[0].layers.len(), 1);
        assert_eq!(back.pages[0].layers[0].name, "Notes");
        assert!(back.pages[0].shapes[0].layer.is_some());
    }

    #[test]
    fn drawio_roundtrip_keeps_structure() {
        let d = sample();
        let xml = crate::formats::drawio::export(&d);
        let back = crate::formats::drawio::import(&xml).unwrap().doc;
        assert_eq!(back.pages.len(), d.pages.len());
        for (a, b) in d.pages.iter().zip(&back.pages) {
            assert_eq!(a.shapes.len(), b.shapes.len());
            assert_eq!(a.conns.len(), b.conns.len());
        }
    }

    #[test]
    fn compressed_drawio_is_inflated() {
        use base64::Engine;
        use std::io::Write;
        let inner = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="a" value="Hello" style="ellipse;" vertex="1" parent="1"><mxGeometry x="10" y="20" width="80" height="40" as="geometry"/></mxCell></root></mxGraphModel>"#;
        let enc: String = inner.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(enc.as_bytes()).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(z.finish().unwrap());
        let file = format!(r#"<mxfile><diagram name="P">{b64}</diagram></mxfile>"#);
        let d = crate::formats::drawio::import(&file).unwrap().doc;
        assert_eq!(d.pages[0].shapes.len(), 1);
        assert_eq!(d.pages[0].shapes[0].kind, "ellipse");
        assert_eq!(d.pages[0].shapes[0].text, "Hello");
    }

    #[test]
    fn routes_avoid_obstacles_and_jump() {
        let t = std::fs::read_to_string("examples/sample.nqs").unwrap();
        let _ = t;
        let mut a = App::new(Settings::default(), None);
        let s1 = a.add_shape("rect", pos2(100.0, 300.0));
        let s2 = a.add_shape("rect", pos2(700.0, 300.0));
        let ob = a.add_shape("rect", pos2(400.0, 300.0));
        if let Some(s) = a.page_mut().shape_mut(ob) { s.w = 160.0; s.h = 200.0; s.x = 320.0; s.y = 200.0; }
        let cid = a.doc.new_id();
        a.page_mut().conns.push(Conn { id: cid, from: End::Glued { shape: s1, port: 1 }, to: End::Glued { shape: s2, port: 3 }, ..Default::default() });
        let route = crate::geom::route(a.page(), &a.page().conns[0]);
        let obr = egui::Rect::from_min_size(pos2(320.0, 200.0), vec2(160.0, 200.0));
        for w in route.windows(2) {
            for k in 0..=20 { let p = w[0] + (w[1] - w[0]) * (k as f32 / 20.0); assert!(!obr.shrink(1.0).contains(p), "route crosses obstacle at {p:?}"); }
        }
        // a vertical line through the horizontal run gets a hop
        let routes = vec![route.clone(), vec![pos2(250.0, 0.0), pos2(250.0, 600.0)]];
        let jumped = crate::route::with_jumps(&routes, 0, 5.0);
        assert!(jumped.len() > route.len() + 5);
    }

    #[test]
    fn manual_route_survives_moves_and_formats() {
        let mut a = App::new(Settings::default(), None);
        let s1 = a.add_shape("rect", pos2(100.0, 100.0));
        let s2 = a.add_shape("rect", pos2(500.0, 400.0));
        let cid = a.doc.new_id();
        a.page_mut().conns.push(Conn { id: cid, from: End::Glued { shape: s1, port: 1 }, to: End::Glued { shape: s2, port: 3 }, bends: Some(vec![[260.0, 100.0], [260.0, 400.0]]), ..Default::default() });
        let check = |p: &Page| {
            let r = crate::geom::route(p, &p.conns[0]);
            for w in r.windows(2) { assert!((w[0].x - w[1].x).abs() < 0.6 || (w[0].y - w[1].y).abs() < 0.6, "not right-angled: {r:?}"); }
            assert!(r.iter().any(|q| (q.x - 260.0).abs() < 0.6), "manual segment lost: {r:?}");
            r
        };
        check(a.page());
        // move the source shape: route stays right-angled and keeps the manual segment
        a.page_mut().shape_mut(s1).unwrap().y += 60.0;
        check(a.page());
        // draw.io and Visio keep the manual route
        let d = crate::formats::drawio::import(&crate::formats::drawio::export(&a.doc)).unwrap().doc;
        assert!(d.pages[0].conns[0].bends.is_some());
        check(&d.pages[0]);
        let v = crate::formats::vsdx::import(crate::formats::vsdx::export(&a.doc).unwrap()).unwrap().doc;
        assert!(v.pages[0].conns[0].bends.is_some());
        check(&v.pages[0]);
        // reset goes back to automatic routing
        a.select_all();
        a.reset_routes();
        assert!(a.page().conns[0].bends.is_none());
    }

    #[test]
    fn vsdx_keeps_pictures() {
        use base64::Engine;
        let mut png = vec![];
        image::RgbaImage::from_pixel(8, 8, image::Rgba([108, 77, 255, 255])).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let mut a = App::new(Settings::default(), None);
        let id = a.add_shape("image", pos2(200.0, 200.0));
        if let Some(s) = a.page_mut().shape_mut(id) { s.image = Some(base64::engine::general_purpose::STANDARD.encode(&png)); s.text = "Icon".into(); }
        let back = crate::formats::vsdx::import(crate::formats::vsdx::export(&a.doc).unwrap()).unwrap().doc;
        let s = back.pages[0].shapes.iter().find(|s| s.kind == "image").expect("picture lost");
        assert_eq!(s.text, "Icon");
        let bytes = base64::engine::general_purpose::STANDARD.decode(s.image.as_ref().unwrap()).unwrap();
        assert_eq!(bytes, png);
    }

    #[test]
    fn new_stencils_have_geometry() {
        for d in crate::library::DEFS.iter().filter(|d| d.cat >= 5) {
            let parts = crate::library::geometry(d.id, d.w, d.h);
            assert!(parts.iter().any(|p| p.kind != crate::library::PK::Hit && p.pts.len() >= 2), "{} draws nothing", d.id);
            for p in &parts { for q in &p.pts { assert!(q.x.is_finite() && q.y.is_finite(), "{} has bad points", d.id); } }
        }
    }

    fn overlaps(p: &Page, ids: &[Id]) -> Option<(Id, Id)> {
        let bs: Vec<(Id, egui::Rect)> = p.shapes.iter().filter(|s| ids.contains(&s.id)).map(|s| (s.id, crate::geom::bbox(s).shrink(0.5))).collect();
        for i in 0..bs.len() { for j in i + 1..bs.len() { if bs[i].1.intersects(bs[j].1) { return Some((bs[i].0, bs[j].0)); } } }
        None
    }

    #[test]
    fn auto_arrange_flowchart_no_overlaps_and_flows_down() {
        let mut a = App::new(Settings::default(), None);
        let ids: Vec<Id> = (0..8).map(|k| a.add_shape("fc_process", pos2(((k * 137) % 500) as f32, ((k * 211) % 400) as f32))).collect();
        let loose = a.add_shape("note", pos2(50.0, 50.0));
        for (x, y) in [(0, 1), (1, 2), (2, 3), (3, 4), (2, 5), (5, 6), (6, 2), (4, 7)] {
            let id = a.doc.new_id();
            a.page_mut().conns.push(Conn { id, from: End::Glued { shape: ids[x], port: 1 }, to: End::Glued { shape: ids[y], port: 3 }, bends: Some(vec![[1.0, 1.0]]), ..Default::default() });
        }
        let before = a.doc.clone();
        a.sel.clear();
        a.auto_arrange(crate::layout::Mode::TopDown);
        let p = a.page();
        let mut all = ids.clone();
        all.push(loose);
        assert_eq!(overlaps(p, &all), None);
        let y = |id: Id| p.shape(id).unwrap().y;
        assert!(y(ids[0]) < y(ids[1]) && y(ids[1]) < y(ids[2]) && y(ids[2]) < y(ids[3]), "main chain should flow downwards");
        // connectors are reset to automatic routing with dynamic glue
        assert!(p.conns.iter().all(|c| c.bends.is_none() && matches!(c.from, End::Glued { port: -1, .. })));
        // one undo step restores everything
        a.do_undo();
        assert_eq!(a.doc, before);
    }

    #[test]
    fn auto_arrange_keeps_swimlanes_and_containers() {
        let d: Document = serde_json::from_str(&std::fs::read_to_string("examples/sample.nqs").unwrap()).unwrap();
        for (pi, mode) in [(0usize, crate::layout::Mode::LeftRight), (1, crate::layout::Mode::TopDown), (1, crate::layout::Mode::Grid)] {
            let mut page = d.pages[pi].clone();
            // remember which container/lane each shape started in
            let conts: Vec<(Id, egui::Rect)> = page.shapes.iter().filter(|s| crate::library::is_container(&s.kind)).map(|s| (s.id, crate::geom::bbox(s))).collect();
            let home: Vec<(Id, Id)> = page.shapes.iter().filter(|s| !crate::library::is_container(&s.kind)).filter_map(|s| {
                let b = crate::geom::bbox(s);
                conts.iter().filter(|(_, r)| r.contains_rect(b)).min_by(|x, y| x.1.area().total_cmp(&y.1.area())).map(|(c, _)| (s.id, *c))
            }).collect();
            crate::layout::auto_arrange(&mut page, &[], mode).unwrap();
            for (sid, cid) in &home {
                let sb = crate::geom::bbox(page.shape(*sid).unwrap());
                let cb = crate::geom::bbox(page.shape(*cid).unwrap());
                assert!(cb.expand(0.5).contains_rect(sb), "page {pi} {mode:?}: shape {sid} left its container/lane");
            }
            let inner: Vec<Id> = page.shapes.iter().filter(|s| !crate::library::is_container(&s.kind)).map(|s| s.id).collect();
            assert_eq!(overlaps(&page, &inner), None, "page {pi} {mode:?}: shapes overlap");
        }
    }

    #[test]
    fn nearly_aligned_ports_route_straight() {
        let mut a = App::new(Settings::default(), None);
        let vm = a.add_shape("c_vm", pos2(132.0, 92.0));
        let dw = a.add_shape("c_warehouse", pos2(138.0, 452.0)); // 6 px out of line
        let id = a.doc.new_id();
        a.page_mut().conns.push(Conn { id, from: End::Glued { shape: vm, port: -1 }, to: End::Glued { shape: dw, port: -1 }, ..Default::default() });
        let r = crate::geom::route(a.page(), &a.page().conns[0]);
        assert_eq!(r.len(), 2, "expected one straight segment, got {r:?}");
        assert!((r[0].x - r[1].x).abs() < 0.01);
    }

    #[test]
    fn stale_manual_route_falls_back_to_automatic() {
        let mut a = App::new(Settings::default(), None);
        let vm = a.add_shape("c_vm", pos2(492.0, 92.0));
        let dw = a.add_shape("c_warehouse", pos2(732.0, 312.0));
        let id = a.doc.new_id();
        // a route made before Data Warehouse moved: it now arrives at the top port from below
        a.page_mut().conns.push(Conn { id, from: End::Glued { shape: vm, port: -1 }, to: End::Glued { shape: dw, port: 0 }, bends: Some(vec![[492.0, 175.0], [400.0, 175.0], [400.0, 400.0], [732.0, 400.0]]), ..Default::default() });
        let p = a.page();
        assert!(crate::geom::manual_route(p, &p.conns[0]).is_none(), "route entering through the shape must be rejected");
        let r = crate::geom::route(p, &p.conns[0]);
        let last = r[r.len() - 1] - r[r.len() - 2];
        assert!(last.y > 0.0, "must arrive at the top port from above: {r:?}");
        let body = crate::geom::bbox(p.shape(dw).unwrap()).shrink(2.0);
        for w in r.windows(2) { for k in 0..=20 { assert!(!body.contains(w[0] + (w[1] - w[0]) * (k as f32 / 20.0)), "route crosses Data Warehouse: {r:?}"); } }
    }
}
