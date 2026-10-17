//! Naqsha Pro, as seen by a build without it.
//!
//! Pro (sheets and title blocks, tags and schedules, engineering symbol libraries) is closed
//! source. It lives in a private repository mounted at `pro/` and is compiled into the official
//! builds with `--features pro`. Builds from this public repository use this file instead: the
//! same functions, with Pro switched off. Diagrams made with Pro still open and display here.
use crate::app::App;
use crate::theme::Pal;
use egui::{RichText, Ui};

/// Is the Pro module part of this build?
#[allow(dead_code)]
pub const INCLUDED: bool = false;

const PAGE: &str = "https://nixpx.com/naqsha#pro";

/// Pro features are unlocked (licence or trial).
pub fn active(_app: &App) -> bool { false }

/// One line about the licence, for About.
pub fn status_line(_app: &App) -> String { "Free edition".into() }

/// Per-frame housekeeping (licence checks finishing in the background, …).
pub fn frame(_app: &mut App, _ctx: &egui::Context) {}

/// A file the user picked for a Pro feature (none without Pro).
pub fn picked(_app: &mut App, _name: &str, _f: crate::files::Picked) {}

/// The Pro tab of the ribbon.
pub fn ribbon(_app: &mut App, ui: &mut Ui, pal: &Pal) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Naqsha Pro").strong().color(pal.accent));
        ui.label(RichText::new("Sheets and title blocks, equipment tags and schedules, P&ID and electrical libraries.").color(pal.muted));
        ui.horizontal(|ui| {
            ui.label(RichText::new("Pro is part of the official download; this build is the open-source edition.").color(pal.faint));
            if ui.link("Learn more").clicked() { ui.ctx().open_url(egui::OpenUrl::new_tab(PAGE)); }
        });
    });
}

/// The Naqsha Pro dialog. Returns false when it should close.
pub fn dialog(_app: &mut App, ui: &mut Ui, pal: &Pal) -> bool {
    let mut keep = true;
    ui.set_width(380.0);
    ui.label(RichText::new("Naqsha Pro").size(18.0).strong().color(pal.accent));
    ui.add_space(6.0);
    ui.label("This is the open-source edition of Naqsha. Pro features are part of the official builds from nixpx.com.");
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Learn more").clicked() { ui.ctx().open_url(egui::OpenUrl::new_tab(PAGE)); }
        if ui.button("Close").clicked() { keep = false; }
    });
    keep
}

/// An engineering symbol by stencil id (Pro stencils only).
pub fn symbol(_kind: &str) -> Option<crate::model::Shape> { None }

/// The tag a new copy of `proto` gets (Pro numbers equipment tags).
pub fn next_tag(_app: &App, proto: &crate::model::Shape) -> String { proto.text.clone() }

/// Extra shape groups for the Shapes panel.
pub fn stencils(_app: &mut App, _ui: &mut Ui, _query: &str) {}

/// Extra templates for the New Diagram gallery.
pub fn templates() -> Vec<crate::templates::Entry> { vec![] }

/// A dialog of the Pro module (Modal::Ext). Returns false when it should close.
pub fn ext_dialog(_app: &mut App, _ui: &mut Ui, _pal: &Pal, _name: &str) -> bool { false }
