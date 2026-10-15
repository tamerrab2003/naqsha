//! Render an SVG to PNG at a given square size (used for icon design).
//! Run: cargo run --release --example render_svg -- in.svg out.png 512
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let svg = std::fs::read_to_string(&a[1]).expect("svg");
    let size: u32 = a[3].parse().unwrap();
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).expect("parse svg");
    let mut pm = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    let s = size as f32 / tree.size().width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(s, s), &mut pm.as_mut());
    std::fs::write(&a[2], pm.encode_png().unwrap()).unwrap();
}
