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

/// Shortcut text as the current OS writes it: "⇧⌘Z" stays as is on macOS and becomes
/// "Ctrl+Shift+Z" on Windows and Linux. Works on whole sentences ("Save (⌘S)").
pub fn keys(s: &str) -> String {
    if cfg!(target_os = "macos") { return s.to_string(); }
    keys_pc(s)
}

pub fn keys_pc(s: &str) -> String {
    let is_mod = |c: char| matches!(c, '⌘' | '⇧' | '⌥' | '⌃');
    let mut out = String::with_capacity(s.len() + 8);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if is_mod(c) {
            let mut mods = vec![c];
            while let Some(&n) = it.peek() { if is_mod(n) { mods.push(n); it.next(); } else { break; } }
            let mut parts: Vec<&str> = vec![];
            if mods.contains(&'⌘') || mods.contains(&'⌃') { parts.push("Ctrl"); }
            if mods.contains(&'⌥') { parts.push("Alt"); }
            if mods.contains(&'⇧') { parts.push("Shift"); }
            out.push_str(&parts.join("+"));
            // the key that follows, if any ("⌘ scroll" and "⇧ while…" have none)
            match it.peek().copied() {
                Some(k) if (!k.is_whitespace() && k != ')' && k != '-') || k_is_minus_key(&it) => {
                    let k = it.next().unwrap();
                    out.push('+');
                    out.push_str(&key_name(k));
                }
                _ => {}
            }
        } else {
            match c { '⌫' => out.push_str("Backspace"), '↩' => out.push_str("Enter"), _ => out.push(c) }
        }
    }
    out
}

/// "⌘-" is Ctrl+Minus, but "⌥-drag" is Alt and a drag: a '-' is a key only at the end of a word.
fn k_is_minus_key(it: &std::iter::Peekable<std::str::Chars>) -> bool {
    let mut look = it.clone();
    look.next() == Some('-') && look.peek().map_or(true, |c| c.is_whitespace() || *c == ')')
}

fn key_name(k: char) -> String {
    match k {
        '↓' => "Down".into(), '↑' => "Up".into(), '←' => "Left".into(), '→' => "Right".into(),
        '↩' => "Enter".into(), '⌫' => "Backspace".into(),
        c => c.to_ascii_uppercase().to_string(),
    }
}

/// Where Naqsha keeps its own files (templates…): ~/Library/Application Support/Naqsha,
/// %APPDATA%\Naqsha, or $XDG_DATA_HOME/naqsha (~/.local/share/naqsha).
pub fn data_dir(legacy: bool) -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let (mac, other) = if legacy { ("Schemix", "schemix") } else { ("Naqsha", "naqsha") };
    if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support").join(mac))
    } else if cfg!(windows) {
        env("APPDATA").or_else(|| env("USERPROFILE").map(|h| h.join("AppData\\Roaming"))).map(|d| d.join(mac))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))).map(|d| d.join(other))
    }
}

/// Where crash logs go: ~/Library/Logs/Naqsha, %LOCALAPPDATA%\Naqsha\Logs, or ~/.local/state/naqsha.
pub fn log_dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Logs/Naqsha"))
    } else if cfg!(windows) {
        env("LOCALAPPDATA").map(|d| d.join("Naqsha").join("Logs"))
    } else {
        env("XDG_STATE_HOME").or_else(|| env("HOME").map(|h| h.join(".local/state"))).map(|d| d.join("naqsha"))
    }
}

/// A release build on Windows is a GUI program with no console; when it is run from a terminal
/// for a command-line job (--export…), print to that terminal.
pub fn attach_console() {
    #[cfg(windows)]
    unsafe {
        #[link(name = "kernel32")]
        extern "system" { fn AttachConsole(pid: u32) -> i32; }
        AttachConsole(u32::MAX); // ATTACH_PARENT_PROCESS
    }
}

#[cfg(test)]
mod tests {
    use super::keys_pc;
    #[test]
    fn shortcuts_read_as_ctrl_on_windows_and_linux() {
        assert_eq!(keys_pc("Save (⌘S)"), "Save (Ctrl+S)");
        assert_eq!(keys_pc("Redo (⇧⌘Z)"), "Redo (Ctrl+Shift+Z)");
        assert_eq!(keys_pc("⌘ scroll, pinch, ⌘= ⌘-"), "Ctrl scroll, pinch, Ctrl+= Ctrl+-");
        assert_eq!(keys_pc("⌘D or ⌥-drag"), "Ctrl+D or Alt-drag");
        assert_eq!(keys_pc("Open  ⌘↓"), "Open  Ctrl+Down");
        assert_eq!(keys_pc("Arrows (⇧ ×10)"), "Arrows (Shift ×10)");
        assert_eq!(keys_pc("Esc or ⌘↩"), "Esc or Ctrl+Enter");
        assert_eq!(keys_pc("⌫"), "Backspace");
        assert_eq!(keys_pc("Bring to front (⇧⌘])"), "Bring to front (Ctrl+Shift+])");
    }
}
