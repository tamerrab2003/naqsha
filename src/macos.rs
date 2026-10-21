//! macOS integration: native menu bar, plus open-document / quit hooks added to winit's app delegate.
#![cfg(target_os = "macos")]

use crate::platform;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2::{sel, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

// ------------------------------------------------------------------ app delegate hooks
//
// winit 0.30 installs its own NSApplicationDelegate ("WinitApplicationDelegate") and, on every
// event, asserts that NSApp's delegate is still that object — replacing it aborts the app.
// So instead of a delegate of our own, we add the selectors winit doesn't implement
// (open documents, should-terminate) to winit's delegate class at runtime.

type Id = *mut AnyObject;

unsafe extern "C-unwind" fn open_urls(_this: Id, _cmd: Sel, _app: Id, urls: *const NSArray<NSURL>) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(urls) = (unsafe { urls.as_ref() }) else { return };
        for url in urls.iter() {
            if let Some(path) = url.path() { platform::push_open(PathBuf::from(path.to_string())); }
        }
    }));
}

unsafe extern "C-unwind" fn open_file(_this: Id, _cmd: Sel, _app: Id, name: *const NSString) -> Bool {
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(name) = (unsafe { name.as_ref() }) else { return false };
        platform::push_open(PathBuf::from(name.to_string()));
        true
    }));
    Bool::new(ok.unwrap_or(false))
}

unsafe extern "C-unwind" fn should_terminate(_this: Id, _cmd: Sel, _app: Id) -> NSApplicationTerminateReply {
    if platform::DIRTY.load(Ordering::SeqCst) {
        platform::QUIT_REQUESTED.store(true, Ordering::SeqCst);
        if let Some(c) = platform::CTX.get() { c.request_repaint(); }
        NSApplicationTerminateReply::TerminateCancel
    } else {
        NSApplicationTerminateReply::TerminateNow
    }
}

#[cfg(target_arch = "aarch64")]
const BOOL_ENC: &str = "B";
#[cfg(not(target_arch = "aarch64"))]
const BOOL_ENC: &str = "c";

/// Must run after the winit EventLoop is created (that is when winit sets its delegate)
/// and before it runs (launch-time "open file" events arrive during startup).
pub fn install_app_delegate() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let Some(delegate) = app.delegate() else { return };
    let obj: &AnyObject = delegate.as_ref();
    let cls = obj.class() as *const AnyClass as *mut AnyClass;
    let add = |sel: Sel, imp: Imp, types: String| {
        let types = std::ffi::CString::new(types).unwrap();
        // returns NO (and changes nothing) if the class already implements the selector
        unsafe { objc2::ffi::class_addMethod(cls, sel, imp, types.as_ptr()) };
    };
    unsafe {
        add(sel!(application:openURLs:), std::mem::transmute::<unsafe extern "C-unwind" fn(Id, Sel, Id, *const NSArray<NSURL>), Imp>(open_urls), "v@:@@".into());
        add(sel!(application:openFile:), std::mem::transmute::<unsafe extern "C-unwind" fn(Id, Sel, Id, *const NSString) -> Bool, Imp>(open_file), format!("{BOOL_ENC}@:@@"));
        add(sel!(applicationShouldTerminate:), std::mem::transmute::<unsafe extern "C-unwind" fn(Id, Sel, Id) -> NSApplicationTerminateReply, Imp>(should_terminate), "Q@:@".into());
    }
    // AppKit caches which optional delegate methods exist when the delegate is assigned,
    // so assign the *same* object again to make it notice the new ones.
    app.setDelegate(None);
    app.setDelegate(Some(&*delegate));
}

// ------------------------------------------------------------------ menu bar

fn cmd(k: Code) -> Option<Accelerator> { Some(Accelerator::new(Modifiers::META, k)) }
fn cmd_shift(k: Code) -> Option<Accelerator> { Some(Accelerator::new(Modifiers::META | Modifiers::SHIFT, k)) }
fn item(id: &str, text: &str, acc: Option<Accelerator>) -> MenuItem { MenuItem::with_id(id, text, true, acc) }

pub fn install_menu(recent: &[String]) -> Option<Box<dyn std::any::Any>> {
    let sep = || PredefinedMenuItem::separator();
    let about = AboutMetadata { name: Some("Naqsha".into()), version: Some(env!("CARGO_PKG_VERSION").into()), copyright: Some("© 2026 Nixpx LLC".into()), comments: Some("Native diagramming for macOS".into()), ..Default::default() };

    let app_menu = Submenu::with_items("Naqsha", true, &[
        &PredefinedMenuItem::about(Some("About Naqsha"), Some(about)),
        &sep(),
        &item("shortcuts", "Keyboard Shortcuts…", cmd(Code::Slash)),
        &sep(),
        &PredefinedMenuItem::services(None),
        &sep(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::hide_others(None),
        &PredefinedMenuItem::show_all(None),
        &sep(),
        &item("quit", "Quit Naqsha", cmd(Code::KeyQ)),
    ]).ok()?;

    let recent_menu = Submenu::new("Open Recent", !recent.is_empty());
    for (i, r) in recent.iter().take(10).enumerate() {
        let name = std::path::Path::new(r).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| r.clone());
        let _ = recent_menu.append(&item(&format!("recent:{i}"), &name, None));
    }
    let export_menu = Submenu::with_items("Export", true, &[
        &item("export:png", "PNG Image…", cmd_shift(Code::KeyE)),
        &item("export:svg", "SVG…", None),
        &item("export:pdf", "PDF…", None),
        &sep(),
        &item("export:vsdx", "Microsoft Visio (.vsdx)…", None),
        &item("export:drawio", "draw.io (.drawio)…", None),
    ]).ok()?;
    let file_menu = Submenu::with_items("File", true, &[
        &item("new", "New", cmd(Code::KeyN)),
        &item("open", "Open…", cmd(Code::KeyO)),
        &recent_menu,
        &sep(),
        &PredefinedMenuItem::close_window(Some("Close Window")),
        &item("save", "Save", cmd(Code::KeyS)),
        &item("save_as", "Save As…", cmd_shift(Code::KeyS)),
        &item("save_template", "Save as Template…", None),
        &sep(),
        &export_menu,
    ]).ok()?;

    // Cut/Copy/Paste are forwarded to egui, so they work in text fields and on the canvas alike.
    let edit_menu = Submenu::with_items("Edit", true, &[
        &item("undo", "Undo", cmd(Code::KeyZ)),
        &item("redo", "Redo", cmd_shift(Code::KeyZ)),
        &sep(),
        &item("cut", "Cut", cmd(Code::KeyX)),
        &item("copy", "Copy", cmd(Code::KeyC)),
        &item("paste", "Paste", cmd(Code::KeyV)),
        &item("duplicate", "Duplicate", cmd(Code::KeyD)),
        &item("delete", "Delete", None),
        &sep(),
        &item("select_all", "Select All", None),
        &item("painter", "Format Painter", None),
    ]).ok()?;

    let view_menu = Submenu::with_items("View", true, &[
        &item("zoom_in", "Zoom In", cmd(Code::Equal)),
        &item("zoom_out", "Zoom Out", cmd(Code::Minus)),
        &item("zoom_100", "Actual Size", cmd(Code::Digit0)),
        &item("fit", "Fit Page", cmd(Code::Digit1)),
        &sep(),
        &item("toggle:rulers", "Rulers", None),
        &item("toggle:grid", "Grid", None),
        &item("toggle:guides", "Dynamic Guides", None),
        &item("toggle:snap", "Snap to Grid", None),
        &sep(),
        &item("toggle:left", "Shapes Panel", None),
        &item("toggle:right", "Format Pane", None),
        &sep(),
        &item("mode:toggle", "Toggle Dark Mode", cmd_shift(Code::KeyD)),
        &sep(),
        &PredefinedMenuItem::fullscreen(None),
    ]).ok()?;

    let insert_menu = Submenu::with_items("Insert", true, &[
        &item("page:new", "New Page", None),
        &item("page:dup", "Duplicate Page", None),
        &item("page:del", "Delete Page", None),
        &sep(),
        &item("insert:container", "Container", None),
        &item("insert:lane_h", "Swimlane", None),
        &item("insert:text", "Text Box", None),
        &item("insert:image", "Picture…", None),
        &item("insert:pipeline", "Pipeline from File…", None),
        &item("insert:flows", "Node-RED Flows from File…", None),
        &item("insert:script", "Script File…", None),
        &sep(),
        &item("layer:new", "New Layer", None),
        &sep(),
        &item("sub:open", "Open / Create Sub-diagram  (⌘↓)", None),
        &item("sub:selection", "Move Selection to Sub-diagram", None),
        &item("sub:back", "Back to Parent Page  (⌘↑)", None),
        &sep(),
        &item("stencil:add", "Add Stencil Folder…", None),
    ]).ok()?;

    let arrange_menu = Submenu::with_items("Arrange", true, &[
        &item("align:0", "Align Left", None),
        &item("align:1", "Align Center", None),
        &item("align:2", "Align Right", None),
        &item("align:3", "Align Top", None),
        &item("align:4", "Align Middle", None),
        &item("align:5", "Align Bottom", None),
        &sep(),
        &item("dist:h", "Distribute Horizontally", None),
        &item("dist:v", "Distribute Vertically", None),
        &sep(),
        &item("front", "Bring to Front", cmd_shift(Code::BracketRight)),
        &item("forward", "Bring Forward", cmd(Code::BracketRight)),
        &item("backward", "Send Backward", cmd(Code::BracketLeft)),
        &item("back", "Send to Back", cmd_shift(Code::BracketLeft)),
        &sep(),
        &item("group", "Group", cmd(Code::KeyG)),
        &item("ungroup", "Ungroup", cmd_shift(Code::KeyG)),
        &item("rotate", "Rotate 90° Right", None),
        &item("rotate_l", "Rotate 90° Left", None),
        &item("flip_h", "Flip Horizontal", None),
        &item("flip_v", "Flip Vertical", None),
        &item("lock", "Lock / Unlock", cmd(Code::KeyL)),
        &sep(),
        &item("layout:auto", "Auto Arrange", cmd_shift(Code::KeyL)),
        &item("layout:tb", "Auto Arrange: Top to Bottom", None),
        &item("layout:lr", "Auto Arrange: Left to Right", None),
        &item("layout:grid", "Auto Arrange: Grid", None),
        &item("layout:tidy", "Tidy Connectors", None),
        &item("reset_route", "Reset Connector Route", None),
        &item("toggle:jumps", "Line Jumps", None),
        &item("toggle:avoid", "Route Around Shapes", None),
    ]).ok()?;

    let window_menu = Submenu::with_items("Window", true, &[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::maximize(None),
        &sep(),
        &PredefinedMenuItem::bring_all_to_front(None),
    ]).ok()?;
    let help_menu = Submenu::with_items("Help", true, &[&item("shortcuts", "Naqsha Keyboard Shortcuts", None)]).ok()?;

    let menu = Menu::with_items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &insert_menu, &arrange_menu, &window_menu, &help_menu]).ok()?;
    menu.init_for_nsapp();
    window_menu.set_as_windows_menu_for_nsapp();
    help_menu.set_as_help_menu_for_nsapp();
    Some(Box::new((menu, window_menu, help_menu)))
}

pub fn take_menu_events() -> Vec<String> {
    let mut out = vec![];
    while let Ok(ev) = MenuEvent::receiver().try_recv() { out.push(ev.id.0.clone()); }
    out
}
