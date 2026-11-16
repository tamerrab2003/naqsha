//! Story videos: a page's story played into a video, the way the presenter shows it (the view
//! gliding to each step, new items coming on, the caption along the bottom), in a shape that suits
//! where it is going: tall for TikTok, Reels and Shorts, wide for YouTube and LinkedIn, or square.
//! Each frame is drawn as an SVG and rendered with resvg; on the desktop the frames are encoded as
//! H.264 in an MP4 file.
use crate::export;
use crate::model::*;
use crate::present::{ease, target_rect, MOVE};
use crate::render::{Ctx, View};
use crate::story;
use crate::theme::Pal;
use egui::*;
use std::collections::HashMap;
use std::fmt::Write as _;

pub const FPS: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format { Tall, Wide, Square }

impl Format {
    pub fn size(self) -> (u32, u32) {
        match self { Format::Tall => (1080, 1920), Format::Wide => (1920, 1080), Format::Square => (1080, 1080) }
    }
}

#[derive(Clone, Debug)]
pub struct Opts {
    pub format: Format,
    /// Shown across the top for the whole video (empty: none).
    pub title: String,
    /// A small "Made with Naqsha" in the corner.
    pub mark: bool,
    /// Seconds a step stays on screen at least (its own "next step after" wins when set).
    pub step_secs: f32,
}

impl Default for Opts {
    fn default() -> Self { Opts { format: Format::Tall, title: String::new(), mark: true, step_secs: 3.5 } }
}

/// Where the view is: the page point in the middle of the picture, and the zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cam { c: Pos2, z: f32 }

struct Seg { k: Option<usize>, start: f32, dur: f32, from: Cam, to: Cam }

/// The whole video worked out ahead: how long each step stays and where the view is for it.
pub struct Plan {
    pub w: u32,
    pub h: u32,
    pub total: f32,
    segs: Vec<Seg>,
    /// The part of the picture the diagram goes in (the title above it, the caption below).
    area: Rect,
    /// Base text size, in pixels.
    fs: f32,
    margin: f32,
    caption_band: f32,
    opts: Opts,
}

/// Seconds to read a caption out loud, roughly.
fn read_secs(s: &str) -> f32 {
    let words = s.split_whitespace().count();
    let chars = s.chars().filter(|c| !c.is_whitespace()).count();
    // Chinese and Japanese have no spaces between words: count characters instead
    let words = if words * 8 < chars { chars / 2 } else { words };
    if words == 0 { 0.0 } else { 1.2 + words as f32 * 0.32 }
}

impl Plan {
    pub fn new(page: &Page, opts: &Opts) -> Plan {
        let (w, h) = opts.format.size();
        let (wf, hf) = (w as f32, h as f32);
        let short = wf.min(hf);
        let margin = short * 0.05;
        let fs = short * 0.04;
        let top = margin + if opts.title.trim().is_empty() { 0.0 } else { fs * 1.3 * 1.25 * 2.0 + margin * 0.4 };
        let captions = page.story.iter().any(|s| !s.caption.trim().is_empty());
        let caption_band = if !captions { margin } else { match opts.format { Format::Tall => hf * 0.19, Format::Square => hf * 0.21, Format::Wide => hf * 0.18 } };
        let area = Rect::from_min_max(pos2(margin, top), pos2(wf - margin, hf - caption_band));
        let all = export::content_bounds(page, 16.0);
        let cam = |k: Option<usize>| {
            let focused = k.and_then(|k| page.story.get(k)).map_or(false, |s| !s.focus.is_empty());
            let r = if focused { target_rect(page, k) } else { all };
            let z = (area.width() / r.width().max(1.0)).min(area.height() / r.height().max(1.0)).min(short / 280.0);
            Cam { c: r.center(), z }
        };
        let mut segs: Vec<Seg> = vec![];
        let mut start = 0.0;
        if page.story.is_empty() {
            let c = cam(None);
            segs.push(Seg { k: None, start, dur: opts.step_secs + 1.5, from: c, to: c });
        }
        for (k, st) in page.story.iter().enumerate() {
            let to = cam(Some(k));
            let from = segs.last().map_or(to, |s| s.to);
            let dur = if st.auto > 0.0 { st.auto } else { opts.step_secs.max(story::motion_secs(page, k) + 1.0).max(read_secs(&st.caption)).max(MOVE + 0.6) };
            segs.push(Seg { k: Some(k), start, dur, from, to });
            start += dur;
        }
        // hold the end a moment
        if let Some(l) = segs.last_mut() { l.dur += 1.0; }
        let total = segs.last().map_or(0.0, |s| s.start + s.dur);
        Plan { w, h, total, segs, area, fs, margin, caption_band, opts: opts.clone() }
    }

    pub fn frames(&self) -> usize { (self.total * FPS as f32).ceil() as usize }

    /// The step, the time into it and the view at frame `i`.
    fn at(&self, i: usize) -> (&Seg, f32, Cam) {
        let tg = i as f32 / FPS as f32;
        let seg = self.segs.iter().rev().find(|s| s.start <= tg).unwrap_or(&self.segs[0]);
        let t = tg - seg.start;
        let f = ease(t / MOVE);
        let (a, b) = (seg.from, seg.to);
        let cam = if a == b || f >= 1.0 { b } else { Cam { c: a.c + (b.c - a.c) * f, z: (a.z.ln() + (b.z.ln() - a.z.ln()) * f).exp() } };
        (seg, t, cam)
    }

    /// Frame `i` as an SVG of the video's size.
    pub fn frame_svg(&self, doc: &Document, page: &Page, pal: &Pal, painter: &Painter, i: usize) -> String {
        let (seg, t, cam) = self.at(i);
        let tg = i as f64 / FPS as f64;
        let accent = { let a = pal.accent; Col([a.r(), a.g(), a.b(), 255]) };
        let shown = story::apply(page, seg.k, story::Mode::Play { t }, accent);
        let fx = story::fx(page, seg.k, t);
        let bg = page.bg.map(crate::theme::to_c32).unwrap_or(pal.page);
        let (w, h) = (self.w as f32, self.h as f32);
        let mut o = String::with_capacity(64 * 1024);
        let _ = write!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">");
        let _ = write!(o, "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", export::col(bg));
        o.push_str(export::SHADOW_DEFS);
        // the diagram, through the moving view
        let ac = self.area.center();
        let _ = write!(o, "<g transform=\"translate({:.2} {:.2}) scale({:.5}) translate({:.2} {:.2})\">", ac.x, ac.y, cam.z, -cam.c.x, -cam.c.y);
        let empty = HashMap::new();
        let cx = Ctx { painter, view: View { origin: Pos2::ZERO, pan: Vec2::ZERO, zoom: 1.0 }, pal, images: &empty, doc: Some(doc), depth: 0, time: Some(tg) };
        export::write_items(&mut o, &cx, &shown, bg, None, Some(&fx));
        o.push_str("</g>");
        // the title across the top, on a band of the background that the diagram slides under
        let title = self.opts.title.trim();
        if !title.is_empty() {
            let edge = self.area.min.y - self.margin * 0.35;
            let fade = self.margin * 0.7;
            let c = export::col(bg);
            let _ = write!(o, "<defs><linearGradient id=\"tfade\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"{c}\"/><stop offset=\"1\" stop-color=\"{c}\" stop-opacity=\"0\"/></linearGradient></defs>");
            let _ = write!(o, "<rect width=\"{w}\" height=\"{edge:.2}\" fill=\"{c}\"/><rect y=\"{edge:.2}\" width=\"{w}\" height=\"{fade:.2}\" fill=\"url(#tfade)\"/>");
            let g = centred(painter, title, FontId::new(self.fs * 1.3, FontFamily::Name("bold".into())), pal.text, w - self.margin * 2.0);
            export::eshape_svg(&mut o, &egui::Shape::galley(pos2(w / 2.0, self.margin) - vec2(g.rect.center().x, 0.0), g, pal.text));
        }
        // the step's caption: a dark band, centred in the space below the diagram
        let caption = seg.k.and_then(|k| page.story.get(k)).map(|s| s.caption.trim().to_string()).unwrap_or_default();
        if !caption.is_empty() {
            let a = (t / 0.35).clamp(0.0, 1.0);
            let pad = vec2(self.fs * 0.8, self.fs * 0.5);
            let white = Color32::from_white_alpha((255.0 * a) as u8);
            let g = centred(painter, &caption, FontId::proportional(self.fs), white, w * 0.86 - pad.x * 2.0);
            let size = g.rect.size() + pad * 2.0;
            let cy = (h - self.caption_band / 2.0).min(h - self.margin - size.y / 2.0);
            let r = Rect::from_center_size(pos2(w / 2.0, cy), size);
            let _ = write!(o, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"rgba(16,12,36,{:.3})\"/>", r.min.x, r.min.y, r.width(), r.height(), self.fs * 0.6, 0.88 * a);
            export::eshape_svg(&mut o, &egui::Shape::galley(pos2(w / 2.0, r.min.y + pad.y) - vec2(g.rect.center().x, 0.0), g, white));
        }
        if self.opts.mark {
            let c = crate::render::with_alpha(pal.muted, 0.8);
            let g = painter.layout_no_wrap("Made with Naqsha".into(), FontId::proportional(self.fs * 0.5), c);
            let pos = pos2(w - self.margin * 0.6 - g.size().x, h - self.margin * 0.6 - g.size().y);
            export::eshape_svg(&mut o, &egui::Shape::galley(pos, g, c));
        }
        o.push_str("</svg>");
        o
    }
}

/// Text wrapped to `width` with each line centred (right-to-left text keeps its own layout).
fn centred(p: &Painter, s: &str, font: FontId, color: Color32, width: f32) -> std::sync::Arc<Galley> {
    if crate::rtl::has_rtl(s) { return crate::i18n::wrapped(p, s, font, color, width); }
    let mut job = text::LayoutJob::simple(s.to_string(), font, color, width);
    job.halign = Align::Center;
    p.layout_job(job)
}

/// Renders a frame's SVG to RGBA pixels.
pub fn raster(svg: &str, opt: &resvg::usvg::Options, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let t = resvg::usvg::Tree::from_str(svg, opt).map_err(|e| e.to_string())?;
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h).ok_or("video too large")?;
    resvg::render(&t, resvg::tiny_skia::Transform::identity(), &mut pm.as_mut());
    Ok(pm.take())
}

/// An MP4 file being written, one frame at a time.
#[cfg(not(target_arch = "wasm32"))]
pub struct Mp4Out {
    enc: openh264::encoder::Encoder,
    out: Option<std::io::BufWriter<std::fs::File>>,
    mp4: Option<mp4::Mp4Writer<std::io::BufWriter<std::fs::File>>>,
    w: u32,
    h: u32,
    n: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl Mp4Out {
    const SCALE: u32 = 90_000;

    pub fn create(path: &std::path::Path, w: u32, h: u32) -> Result<Self, String> {
        use openh264::encoder::*;
        let bps = (w as f64 * h as f64 * FPS as f64 * 0.12) as u32;
        let cfg = EncoderConfig::new()
            .bitrate(BitRate::from_bps(bps))
            .max_frame_rate(FrameRate::from_hz(FPS as f32))
            .usage_type(UsageType::ScreenContentRealTime)
            .rate_control_mode(RateControlMode::Bitrate)
            .skip_frames(false)
            .adaptive_quantization(false)
            .background_detection(false)
            .intra_frame_period(IntraFramePeriod::from_num_frames(FPS * 2));
        let enc = Encoder::with_api_config(openh264::OpenH264API::from_source(), cfg).map_err(|e| e.to_string())?;
        let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
        Ok(Mp4Out { enc, out: Some(std::io::BufWriter::new(f)), mp4: None, w, h, n: 0 })
    }

    pub fn push(&mut self, rgba: &[u8]) -> Result<(), String> {
        use openh264::formats::{RgbaSliceU8, YUVBuffer};
        let yuv = YUVBuffer::from_rgba8_source(RgbaSliceU8::new(rgba, (self.w as usize, self.h as usize)));
        let bs = self.enc.encode(&yuv).map_err(|e| e.to_string())?;
        let (mut sps, mut pps, mut sample, mut sync) = (None, None, Vec::new(), false);
        for l in 0..bs.num_layers() {
            let Some(layer) = bs.layer(l) else { continue };
            for i in 0..layer.nal_count() {
                let Some(nal) = layer.nal_unit(i) else { continue };
                // drop the 00 00 (00) 01 start code: MP4 stores each unit after its length
                let nal = nal.strip_prefix(&[0, 0, 0, 1]).or_else(|| nal.strip_prefix(&[0, 0, 1])).unwrap_or(nal);
                let Some(&head) = nal.first() else { continue };
                match head & 0x1f {
                    7 => sps = Some(nal.to_vec()),
                    8 => pps = Some(nal.to_vec()),
                    t => {
                        if t == 5 { sync = true; }
                        sample.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                        sample.extend_from_slice(nal);
                    }
                }
            }
        }
        if self.mp4.is_none() {
            let (Some(sps), Some(pps)) = (sps, pps) else { return Err("the video encoder did not start with a key frame".into()) };
            let out = self.out.take().ok_or("video already finished")?;
            let cfg = mp4::Mp4Config { major_brand: "isom".parse().unwrap(), minor_version: 512, compatible_brands: ["isom", "iso2", "avc1", "mp41"].iter().map(|b| b.parse().unwrap()).collect(), timescale: 1000 };
            let mut m = mp4::Mp4Writer::write_start(out, &cfg).map_err(|e| e.to_string())?;
            m.add_track(&mp4::TrackConfig { track_type: mp4::TrackType::Video, timescale: Self::SCALE, language: "und".into(), media_conf: mp4::MediaConfig::AvcConfig(mp4::AvcConfig { width: self.w as u16, height: self.h as u16, seq_param_set: sps, pic_param_set: pps }) }).map_err(|e| e.to_string())?;
            self.mp4 = Some(m);
        }
        let per = Self::SCALE / FPS;
        let m = self.mp4.as_mut().unwrap();
        m.write_sample(1, &mp4::Mp4Sample { start_time: self.n * per as u64, duration: per, rendering_offset: 0, is_sync: sync, bytes: bytes::Bytes::from(sample) }).map_err(|e| e.to_string())?;
        self.n += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), String> {
        let mut m = self.mp4.take().ok_or("no frames")?;
        m.write_end().map_err(|e| e.to_string())?;
        use std::io::Write;
        m.into_writer().flush().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_last_long_enough() {
        let mut p = Page::default();
        p.shapes.push(crate::model::Shape { id: 1, ..Default::default() });
        p.story = vec![
            Step { caption: "Short".into(), ..Default::default() },
            Step { caption: "A much longer caption that takes a while to read out loud to the people watching this".into(), ..Default::default() },
            Step { auto: 1.5, ..Default::default() },
        ];
        let plan = Plan::new(&p, &Opts::default());
        assert_eq!(plan.segs.len(), 3);
        assert!((plan.segs[0].dur - 3.5).abs() < 1e-3);
        assert!(plan.segs[1].dur > 5.0, "{}", plan.segs[1].dur);
        assert!((plan.segs[2].dur - 2.5).abs() < 1e-3, "its own time, plus the hold at the end");
        assert_eq!(plan.frames(), (plan.total * 30.0).ceil() as usize);
        // the view stays put when nothing is framed
        assert_eq!(plan.segs[1].from, plan.segs[1].to);
    }
}

/// A video being made in the background of the app: a few frames each time the window draws, so
/// it stays responsive and shows how far along it is.
#[cfg(not(target_arch = "wasm32"))]
pub struct Job {
    pub plan: Plan,
    doc: Document,
    pi: usize,
    pal: Pal,
    fonts: resvg::usvg::Options<'static>,
    out: Option<Mp4Out>,
    pub i: usize,
    pub path: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl Job {
    pub fn start(doc: &Document, pi: usize, pal: Pal, opts: &Opts, path: std::path::PathBuf) -> Result<Job, String> {
        let page = doc.pages.get(pi).ok_or("no such page")?;
        let plan = Plan::new(page, opts);
        let out = Mp4Out::create(&path, plan.w, plan.h)?;
        Ok(Job { plan, doc: doc.clone(), pi, pal, fonts: export::svg_options(), out: Some(out), i: 0, path })
    }

    /// Makes frames for about `budget`; true once the file is finished.
    pub fn work(&mut self, painter: &Painter, budget: std::time::Duration) -> Result<bool, String> {
        let t0 = std::time::Instant::now();
        let n = self.plan.frames();
        let page = &self.doc.pages[self.pi];
        while self.i < n && t0.elapsed() < budget {
            let svg = self.plan.frame_svg(&self.doc, page, &self.pal, painter, self.i);
            let px = raster(&svg, &self.fonts, self.plan.w, self.plan.h)?;
            self.out.as_mut().ok_or("video finished")?.push(&px)?;
            self.i += 1;
        }
        if self.i < n { return Ok(false); }
        if let Some(out) = self.out.take() { out.finish()?; }
        Ok(true)
    }

    pub fn progress(&self) -> f32 { self.i as f32 / self.plan.frames().max(1) as f32 }

    /// Stops and removes the unfinished file.
    pub fn cancel(mut self) {
        drop(self.out.take());
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The browser can't encode video (yet): there is no job.
#[cfg(target_arch = "wasm32")]
pub struct Job;
