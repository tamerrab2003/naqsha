//! Naqsha inside another app: `…/naqsha/app/?embed=<host>` (the ONLYOFFICE plugin shows the
//! browser version in a frame, the way Visio drawings sit inside Word).
//!
//! The host talks to the editor with `postMessage`:
//! - editor → host `{naqsha: "ready", version}` once it is running;
//! - host → editor `{naqsha: "load", doc}`: the diagram stored in the document (`.nqs` JSON),
//!   or `null`/`""` for a new one (the template gallery opens as usual);
//! - host → editor `{naqsha: "export"}` when the user presses Insert;
//! - editor → host `{naqsha: "exported", doc, png, width, height}`: the whole diagram to store,
//!   and a picture of the current page (a PNG data URL at 2× `width` × `height` CSS pixels) to
//!   show in the document; or `{naqsha: "error", message}`.
//! - host → editor `{naqsha: "values", values: {tag: value, …}}`: live values (see `live.rs`);
//!   `{naqsha: "clear"}` removes them.
//! - host → editor `{naqsha: "theme", type: "dark" | "light"}`: follow the host's theme (on load
//!   and whenever it changes). `&theme=dark|light` in the address sets it from the first frame.
//!
//! With `&view=1` the editor is a live view: the drawing only, no editing (the Node-RED add-on).
//!
//! While embedded the editor doesn't recover unsaved work or warn before the page closes: the
//! host document is where the diagram lives.

#[cfg(target_arch = "wasm32")]
mod imp {
    use crate::app::App;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};
    use wasm_bindgen::JsCast;

    enum Msg { Load(Option<String>), Export, Values(serde_json::Map<String, serde_json::Value>), Clear, Theme(bool) }

    static ON: OnceLock<bool> = OnceLock::new();
    static INBOX: Mutex<Vec<Msg>> = Mutex::new(Vec::new());
    static HOST: Mutex<Option<String>> = Mutex::new(None);
    static GREETED: AtomicBool = AtomicBool::new(false);

    pub fn active() -> bool { *ON.get_or_init(|| crate::files::web::param("embed").is_some()) }

    /// Listen to the host page. Messages from anywhere else are ignored.
    pub fn install() {
        if !active() { return; }
        let Some(win) = web_sys::window() else { return };
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MessageEvent)>::new(|e: web_sys::MessageEvent| {
            let Some(parent) = web_sys::window().and_then(|w| w.parent().ok().flatten()) else { return };
            let from_host = e.source().map_or(false, |s| js_sys::Object::is(s.as_ref(), parent.as_ref()));
            if !from_host { return; }
            let d = e.data();
            let text = d.as_string().or_else(|| js_sys::JSON::stringify(&d).ok().and_then(|s| s.as_string()));
            let Some(v) = text.and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { return };
            let msg = match v["naqsha"].as_str() {
                Some("load") => Msg::Load(v["doc"].as_str().filter(|s| !s.trim().is_empty()).map(String::from)),
                Some("export") => Msg::Export,
                Some("values") => match v["values"].as_object() { Some(o) => Msg::Values(o.clone()), None => return },
                Some("clear") => Msg::Clear,
                Some("theme") => match v["type"].as_str() { Some("dark") => Msg::Theme(true), Some("light") => Msg::Theme(false), _ => return },
                _ => return,
            };
            if let Ok(mut h) = HOST.lock() { *h = Some(e.origin()); }
            if let Ok(mut q) = INBOX.lock() { q.push(msg); }
            if let Some(c) = crate::platform::CTX.get() { c.request_repaint(); }
        });
        let _ = win.add_event_listener_with_callback("message", f.as_ref().unchecked_ref());
        f.forget();
    }

    fn post(v: &serde_json::Value) {
        let Some(parent) = web_sys::window().and_then(|w| w.parent().ok().flatten()) else { return };
        let Ok(js) = js_sys::JSON::parse(&v.to_string()) else { return };
        // answer only the page that asked; before it has (or from a file:// host) there is no origin to name
        let origin = HOST.lock().ok().and_then(|h| h.clone()).filter(|o| o.starts_with("http")).unwrap_or_else(|| "*".into());
        let _ = parent.post_message(&js, &origin);
    }

    /// Save while embedded: send the diagram to the host as if it had asked for it.
    pub fn save_to_host() {
        if let Ok(mut q) = INBOX.lock() { q.push(Msg::Export); }
        if let Some(c) = crate::platform::CTX.get() { c.request_repaint(); }
    }

    pub fn frame(app: &mut App, ui: &egui::Ui) {
        if !active() { return; }
        if !GREETED.load(Ordering::SeqCst) {
            app.viewer = crate::files::web::param("view").map_or(false, |v| v != "0");
            match crate::files::web::param("theme").as_deref() {
                Some("dark") => app.set.mode = crate::app::Mode::Dark,
                Some("light") => app.set.mode = crate::app::Mode::Light,
                _ => {}
            }
        }
        if !GREETED.swap(true, Ordering::SeqCst) {
            post(&serde_json::json!({ "naqsha": "ready", "version": env!("CARGO_PKG_VERSION") }));
        }
        let msgs: Vec<Msg> = INBOX.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default();
        for m in msgs {
            match m {
                Msg::Load(Some(t)) => match serde_json::from_str::<crate::model::Document>(&t) {
                    Ok(d) => {
                        app.adopt(d, "");
                        app.offer_gallery = false;
                        if matches!(app.modal, Some(crate::app::Modal::NewDiagram)) { app.modal = None; }
                    }
                    Err(e) => app.notify(crate::i18n::trf("Couldn't read this diagram: {error}", &[("error", &e.to_string())])),
                },
                Msg::Load(None) => {}
                Msg::Values(o) => {
                    for (k, v) in o { app.live.insert(k, crate::live::parse(&v)); }
                }
                Msg::Clear => app.live.clear(),
                Msg::Theme(dark) => {
                    // the host's theme, for this session only (an embedded editor saves no settings)
                    app.set.mode = if dark { crate::app::Mode::Dark } else { crate::app::Mode::Light };
                    ui.ctx().request_repaint();
                }
                Msg::Export => export(app, ui),
            }
        }
    }

    fn export(app: &mut App, ui: &egui::Ui) {
        use base64::Engine;
        if app.doc.pages.iter().all(|p| p.shapes.is_empty() && p.conns.is_empty()) {
            app.notify("Draw something (or pick a template from File › New) before inserting");
            post(&serde_json::json!({ "naqsha": "error", "message": "empty" }));
            return;
        }
        // documents are printed on white: always the light look, whatever the editor shows
        let (svg, w, h) = crate::export::page_svg(Some(&app.doc), app.page(), &crate::theme::LIGHT, ui.painter(), 16.0);
        let reply = crate::export::svg_to_png(&svg, 2.0).and_then(|png| {
            let doc = serde_json::to_string(&app.doc).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "naqsha": "exported", "doc": doc, "width": w, "height": h,
                "png": format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)),
            }))
        });
        match reply {
            Ok(v) => { post(&v); app.dirty = false; }
            Err(e) => { app.notify(crate::i18n::trf("Couldn't insert the diagram: {error}", &[("error", &e.to_string())])); post(&serde_json::json!({ "naqsha": "error", "message": e })); }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::{active, frame, install, save_to_host};

/// The desktop app is never embedded.
#[cfg(not(target_arch = "wasm32"))]
pub fn active() -> bool { false }
#[cfg(not(target_arch = "wasm32"))]
pub fn save_to_host() {}
