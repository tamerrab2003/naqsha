//! Regenerates assets/Naqsha.icns (and a 512px PNG) from assets/icon.svg.
//! Run: cargo run --release --example make_icon
fn main() {
    let svg = std::fs::read_to_string("assets/icon.svg").expect("assets/icon.svg");
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).expect("parse svg");
    let render = |size: u32| -> Vec<u8> {
        let mut pm = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
        let s = size as f32 / tree.size().width();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(s, s), &mut pm.as_mut());
        pm.encode_png().unwrap()
    };
    // PNG-backed icns entries understood by macOS 10.7+
    let types: [(&[u8; 4], u32); 10] = [(b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64), (b"ic13", 256)];
    let mut body = Vec::new();
    for (t, size) in types {
        let png = render(size);
        body.extend_from_slice(t);
        body.extend_from_slice(&((png.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut icns = b"icns".to_vec();
    icns.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    icns.extend_from_slice(&body);
    std::fs::write("assets/Naqsha.icns", icns).unwrap();
    std::fs::write("assets/icon-512.png", render(512)).unwrap();
    println!("wrote assets/Naqsha.icns and assets/icon-512.png");
}
