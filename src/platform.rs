//! Cross-platform glue for OS integrations (menu bar, files opened from Finder, quit requests).
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Files the OS asked us to open (Finder double-click, Dock drop, `open -a`).
pub static OPEN_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// The OS asked the app to quit (⌘Q from the Dock, logout…) while there were unsaved changes.
pub static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Mirrors App::dirty so the native layer can decide whether quitting needs a prompt.
pub static DIRTY: AtomicBool = AtomicBool::new(false);
/// Lets native callbacks wake the UI.
pub static CTX: OnceLock<egui::Context> = OnceLock::new();

#[allow(dead_code)]
pub fn push_open(p: PathBuf) {
    if let Ok(mut q) = OPEN_QUEUE.lock() { q.push(p); }
    if let Some(c) = CTX.get() { c.request_repaint(); }
}

pub fn take_open() -> Vec<PathBuf> { OPEN_QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default() }

pub fn take_quit() -> bool { QUIT_REQUESTED.swap(false, Ordering::SeqCst) }

pub fn set_dirty(d: bool) { DIRTY.store(d, Ordering::SeqCst); }

/// Menu-bar commands chosen since the last frame (ids as defined in `macos::build_menu`).
pub fn take_menu_events() -> Vec<String> {
    #[cfg(target_os = "macos")]
    { crate::macos::take_menu_events() }
    #[cfg(not(target_os = "macos"))]
    { Vec::new() }
}

/// Install OS hooks that must exist before the event loop starts.
pub fn before_event_loop() {
    #[cfg(target_os = "macos")]
    crate::macos::install_app_delegate();
}

/// Build the native menu bar (macOS). Returned value must be kept alive.
pub fn install_menu(recent: &[String]) -> Option<Box<dyn std::any::Any>> {
    #[cfg(target_os = "macos")]
    { crate::macos::install_menu(recent) }
    #[cfg(not(target_os = "macos"))]
    { let _ = recent; None }
}
