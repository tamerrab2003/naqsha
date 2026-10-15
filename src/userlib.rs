//! User stencil folders: any folder of SVG icons (for example a vendor's official
//! architecture-icon package) shown as shape groups in the Shapes panel.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct UserIcon {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct UserGroup {
    pub name: String,
    /// The stencil folder (as stored in settings) this group came from.
    pub root: String,
    pub icons: Vec<UserIcon>,
}

const MAX_FILES: usize = 20_000;

/// Folder names that only describe a size or theme variant, not a category.
fn is_variant_dir(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let digits = n.chars().filter(|c| c.is_ascii_digit()).count();
    let letters: String = n.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    (digits > 0 && matches!(letters.as_str(), "" | "x" | "px" | "res" | "light" | "dark" | "reslight" | "resdark"))
        || matches!(n.as_str(), "light" | "dark" | "svg" | "svgs" | "icons" | "color" | "colour" | "mono" | "outline")
}

/// Trailing size number in a file stem (`name_48` → 48).
fn size_of(stem: &str, dir: &str) -> Option<u32> {
    let tail = stem.rsplit(['_', '-', '@']).next().unwrap_or("");
    tail.trim_end_matches("px").parse().ok().or_else(|| dir.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
}

fn clean(s: &str) -> String {
    let mut t = s.to_string();
    // drop a trailing size suffix: "_48", "-64px", "@2x"
    if let Some(i) = t.rfind(['_', '-', '@']) {
        let tail = &t[i + 1..];
        if !tail.is_empty() && tail.trim_end_matches("px").trim_end_matches('x').chars().all(|c| c.is_ascii_digit()) { t.truncate(i); }
    }
    // drop a short leading tag such as "Arch_", "Res_", "Icon-"
    if let Some(i) = t.find(['_']) {
        let head = &t[..i];
        if (2..=5).contains(&head.len()) && head.chars().all(|c| c.is_ascii_alphabetic()) && t.len() > i + 3 { t = t[i + 1..].to_string(); }
    }
    let t = t.replace(['_', '-'], " ");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 8 || out.len() >= MAX_FILES { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if name.starts_with('.') || name.starts_with("__MACOSX") { continue; }
        if p.is_dir() { walk(&p, depth + 1, out); } else if p.extension().map_or(false, |e| e.eq_ignore_ascii_case("svg")) { out.push(p); }
        if out.len() >= MAX_FILES { return; }
    }
}

/// Scan one stencil folder into groups (one per category sub-folder).
pub fn scan(root: &str) -> Vec<UserGroup> {
    let rootp = PathBuf::from(root);
    let root_name = clean(&rootp.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Stencils".into()));
    let mut files = vec![];
    walk(&rootp, 0, &mut files);
    // (group, icon name) -> (score, path)
    let mut best: HashMap<(String, String), (i64, PathBuf)> = HashMap::new();
    let mut order: Vec<(String, String)> = vec![];
    for f in files {
        // category = nearest ancestor that isn't just a size/theme variant folder
        let mut cat = None;
        let mut dir_for_size = String::new();
        let mut a = f.parent();
        while let Some(d) = a {
            if d == rootp.as_path() { break; }
            let n = d.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if dir_for_size.is_empty() { dir_for_size = n.clone(); }
            if !is_variant_dir(&n) { cat = Some(clean(&n)); break; }
            a = d.parent();
        }
        let group = cat.filter(|c| !c.is_empty()).unwrap_or_else(|| root_name.clone());
        let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let name = clean(&stem);
        if name.is_empty() { continue; }
        // prefer the largest variant up to 64 px, and light over dark
        let size = size_of(&stem, &dir_for_size).unwrap_or(48) as i64;
        let lower = f.to_string_lossy().to_ascii_lowercase();
        let score = if size <= 64 { size } else { 64 - size / 64 } - if lower.contains("dark") { 1000 } else { 0 };
        let key = (group, name);
        match best.get(&key) {
            Some((s, _)) if *s >= score => {}
            prev => { if prev.is_none() { order.push(key.clone()); } best.insert(key, (score, f)); }
        }
    }
    let mut groups: Vec<UserGroup> = vec![];
    for key in order {
        let (_, path) = best.remove(&key).unwrap();
        let (g, name) = key;
        match groups.iter_mut().find(|x| x.name == g) {
            Some(x) => x.icons.push(UserIcon { name, path }),
            None => groups.push(UserGroup { name: g, root: root.to_string(), icons: vec![UserIcon { name, path }] }),
        }
    }
    for g in groups.iter_mut() { g.icons.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())); }
    groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    groups
}

/// Rasterise an SVG so its longer side is `max_px`. Returns premultiplied RGBA + size.
pub fn rasterize(path: &Path, max_px: u32) -> Option<(Vec<u8>, u32, u32)> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let sz = tree.size();
    let scale = max_px as f32 / sz.width().max(sz.height()).max(1.0);
    let (w, h) = (((sz.width() * scale).round() as u32).max(1), ((sz.height() * scale).round() as u32).max(1));
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Some((pm.data().to_vec(), w, h))
}

/// PNG bytes for an icon, plus its aspect ratio (w / h).
pub fn to_png(path: &Path, max_px: u32) -> Option<(Vec<u8>, f32)> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let sz = tree.size();
    let scale = max_px as f32 / sz.width().max(sz.height()).max(1.0);
    let (w, h) = (((sz.width() * scale).round() as u32).max(1), ((sz.height() * scale).round() as u32).max(1));
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Some((pm.encode_png().ok()?, sz.width() / sz.height().max(0.01)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_variants() {
        assert_eq!(clean("Arch_Compute-Engine_48"), "Compute Engine");
        assert_eq!(clean("Res_Object-Store_Light@2x"), "Object Store Light");
        assert_eq!(clean("message-queue"), "message queue");
        assert!(is_variant_dir("48"));
        assert!(is_variant_dir("64x64"));
        assert!(is_variant_dir("Res_48_Light"));
        assert!(!is_variant_dir("Arch_Compute"));
    }

    #[test]
    fn scan_groups_and_dedups_sizes() {
        let dir = std::env::temp_dir().join(format!("naqsha-stencil-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48"><rect width="48" height="48" fill="#6C4DFF"/></svg>"##;
        for (sub, f) in [("Pack/Arch_Compute/16", "Arch_Server-Thing_16.svg"), ("Pack/Arch_Compute/48", "Arch_Server-Thing_48.svg"), ("Pack/Arch_Compute/64", "Arch_Server-Thing_64.svg"), ("Pack/Arch_Storage/48", "Arch_Disk-Thing_48.svg"), ("Pack", "loose.svg")] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
            std::fs::write(dir.join(sub).join(f), svg).unwrap();
        }
        let groups = scan(dir.join("Pack").to_str().unwrap());
        let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["Compute", "Pack", "Storage"]);
        let compute = &groups[0];
        assert_eq!(compute.icons.len(), 1, "size variants collapse to one icon");
        assert_eq!(compute.icons[0].name, "Server Thing");
        assert!(compute.icons[0].path.to_string_lossy().contains("_64"));
        assert!(rasterize(&compute.icons[0].path, 64).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
