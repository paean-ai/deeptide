//! `deeptide-gui` — native desktop GUI for Deeptide.
//!
//! A single Rust binary that embeds `deeptide-core`, so it shares the CLI's
//! config, session history, and the full tool set with no duplication. Each
//! conversation runs the synchronous agent loop on its own worker thread and
//! streams events to the egui UI (see `conversation.rs`).

mod app;
mod conversation;
mod events;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Deeptide")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([480.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Deeptide",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
