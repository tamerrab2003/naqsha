//! Opening and saving files. The desktop app uses native file dialogs and the disk; the browser
//! version uses the browser's file picker, downloads and the page URL.
use crate::app::App;
use std::path::PathBuf;

/// What a picked file is for.
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(not(feature = "pro"), allow(dead_code))]
pub enum Purpose { Open, Script, Pipeline, Image, /** a file for a Pro feature, by name */ Ext(&'static str) }

/// A file the user chose, read into memory.
pub struct Picked {
    pub purpose: Purpose,
    pub name: String,
    /// Where it lives on disk (desktop only).
    pub path: Option<PathBuf>,
    pub bytes: Result<Vec<u8>, String>,
}

pub type Filters = &'static [(&'static str, &'static [&'static str])];

pub const OPEN_FILTERS: Filters = &[
    ("All supported", &["nqs", "schx", "vsdx", "drawio", "xml", "svg", "json", "yml", "yaml", "groovy", "jenkinsfile"]),
    ("Naqsha diagram", &["nqs", "schx"]),
    ("Microsoft Visio", &["vsdx"]),
    ("draw.io / diagrams.net", &["drawio", "xml"]),
    ("SVG", &["svg"]),
    ("CI/CD pipeline (GitHub, GitLab, Azure, Bitbucket, Jenkins)", &["yml", "yaml", "groovy", "jenkinsfile"]),
];

/// True in the browser build.
pub const WEB: bool = cfg!(target_arch = "wasm32");
/// What holds one diagram: a window on the desktop, a tab in the browser.
pub const WINDOW: &str = if WEB { "tab" } else { "window" };

#[cfg(target_arch = "wasm32")]
static PICKED: std::sync::Mutex<Vec<Picked>> = std::sync::Mutex::new(Vec::new());

impl App {
    /// Ask for a file. The result arrives through `take_picked` (at once on the desktop, when the
    /// browser hands it over on the web).
    pub fn pick_file(&mut self, purpose: Purpose, title: &str, filters: Filters) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut d = rfd::FileDialog::new().set_title(title);
            for (n, e) in filters { d = d.add_filter(*n, e); }
            if let Some(p) = d.pick_file() {
                let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let bytes = std::fs::read(&p).map_err(|e| e.to_string());
                self.picked.push(Picked { purpose, name, path: Some(p), bytes });
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut d = rfd::AsyncFileDialog::new().set_title(title);
            for (n, e) in filters { d = d.add_filter(*n, e); }
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(h) = d.pick_file().await {
                    let bytes = h.read().await;
                    if let Ok(mut q) = PICKED.lock() { q.push(Picked { purpose, name: h.file_name(), path: None, bytes: Ok(bytes) }); }
                    if let Some(c) = crate::platform::CTX.get() { c.request_repaint(); }
                }
            });
        }
    }

    pub fn take_picked(&mut self) -> Vec<Picked> {
        #[allow(unused_mut)]
        let mut v = std::mem::take(&mut self.picked);
        #[cfg(target_arch = "wasm32")]
        if let Ok(mut q) = PICKED.lock() { v.extend(q.drain(..)); }
        v
    }
}

/// A file dropped onto the page: read it, then open it like a picked file.
#[cfg(target_arch = "wasm32")]
pub fn read_dropped(f: egui::DroppedFileHandle) {
    wasm_bindgen_futures::spawn_local(async move {
        let name = f.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let lower = name.to_lowercase();
        let purpose = if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") { Purpose::Image } else { Purpose::Open };
        let bytes = f.bytes_async().await;
        if let Ok(mut q) = PICKED.lock() { q.push(Picked { purpose, name, path: None, bytes }); }
        if let Some(c) = crate::platform::CTX.get() { c.request_repaint(); }
    });
}

/// Native save dialog. Returns the chosen path (with an extension added when the name has none).
#[cfg(not(target_arch = "wasm32"))]
pub fn save_dialog(title: &str, name: &str, filters: Filters) -> Option<PathBuf> {
    let mut d = rfd::FileDialog::new().set_title(title).set_file_name(name);
    for (n, e) in filters { d = d.add_filter(*n, e); }
    let p = d.save_file()?;
    Some(if p.extension().is_none() { p.with_extension(filters.first().and_then(|f| f.1.first()).copied().unwrap_or("nqs")) } else { p })
}

/// In the browser a save is a download: the name is all there is to choose.
#[cfg(target_arch = "wasm32")]
pub fn save_dialog(_title: &str, name: &str, _filters: Filters) -> Option<PathBuf> { Some(PathBuf::from(name)) }

/// Hand a file to the browser as a download.
#[cfg(target_arch = "wasm32")]
pub fn download(name: &str, bytes: &[u8], mime: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let err = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let win = web_sys::window().ok_or("no window")?;
    let doc = win.document().ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = doc.create_element("a").map_err(err)?.dyn_into().map_err(|_| "no anchor".to_string())?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // let the download start before the object URL goes away
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || { let _ = web_sys::Url::revoke_object_url(&url); });
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000);
    Ok(())
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "vsdx" => "application/vnd.ms-visio.drawing.main+xml",
        "drawio" => "application/vnd.jgraph.mxfile",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "csv" => "text/csv",
        "dxf" => "image/vnd.dxf",
        _ => "application/json",
    }
}

/// Write a finished file: to `path` on the desktop, as a download (named after `path`) on the web.
pub fn deliver(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    { std::fs::write(path, bytes).map_err(|e| e.to_string()) }
    #[cfg(target_arch = "wasm32")]
    {
        let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "diagram".into());
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        download(&name, bytes, mime_for(&ext))
    }
}

// ---------------------------------------------------------------- browser page integration

/// The browser tab: title, "leave page?" warning, and handing a document to a new tab.
#[cfg(target_arch = "wasm32")]
pub mod web {
    use wasm_bindgen::JsCast;

    pub fn set_title(t: &str) {
        if let Some(d) = web_sys::window().and_then(|w| w.document()) { d.set_title(t); }
    }

    /// Warn before closing or reloading the tab while there are unsaved changes.
    pub fn install_unload_guard() {
        let Some(win) = web_sys::window() else { return };
        let f = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(|e: web_sys::BeforeUnloadEvent| {
            if crate::platform::DIRTY.load(std::sync::atomic::Ordering::SeqCst) {
                e.prevent_default();
                e.set_return_value("You have unsaved changes");
            }
        });
        win.set_onbeforeunload(Some(f.as_ref().unchecked_ref()));
        f.forget();
    }

    fn storage() -> Option<web_sys::Storage> { web_sys::window()?.local_storage().ok()? }

    fn param(name: &str) -> Option<String> {
        let search = web_sys::window()?.location().search().ok()?;
        web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(name)
    }

    /// Open a document in a new browser tab. Returns false if the browser blocked the new tab.
    pub fn open_in_new_tab(doc: &crate::model::Document, name: &str) -> bool {
        let Some(win) = web_sys::window() else { return false };
        let key = format!("naqsha-handoff-{}", (js_sys::Math::random() * 1e12) as u64);
        let Ok(json) = serde_json::to_string(&(doc, name)) else { return false };
        let Some(st) = storage() else { return false };
        if st.set_item(&key, &json).is_err() { return false; }
        let path = win.location().pathname().unwrap_or_default();
        match win.open_with_url_and_target(&format!("{path}?doc={key}"), "_blank") {
            Ok(Some(_)) => true,
            _ => { let _ = st.remove_item(&key); false }
        }
    }

    /// What this tab was opened with: a handed-over document (`?doc=`) or a template (`?template=`).
    pub fn start_document() -> Option<(crate::model::Document, String)> {
        if let Some(key) = param("doc") {
            let st = storage()?;
            let json = st.get_item(&key).ok()??;
            let _ = st.remove_item(&key);
            return serde_json::from_str(&json).ok();
        }
        if let Some(id) = param("template") {
            let e = crate::templates::builtin().into_iter().find(|e| e.id == id)?;
            return Some((e.doc, e.name.to_string()));
        }
        None
    }

    /// Running in a browser on a Mac (shortcuts use ⌘ there)?
    pub fn is_mac() -> bool {
        web_sys::window().and_then(|w| w.navigator().user_agent().ok()).map_or(false, |ua| ua.contains("Mac"))
    }
}
