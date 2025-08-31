// Set Windows subsystem to "windows" to prevent console window from appearing
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;

use app::CLIvergeApp;
use eframe::egui;

fn setup_fonts(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::new(22.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
        (egui::TextStyle::Button, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Small, egui::FontId::new(13.0, egui::FontFamily::Proportional)),
    ]
    .iter()
    .cloned()
    .collect();
    
    ctx.set_style(style);
}

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        use tracing_subscriber::fmt::writer::MakeWriterExt;
        
        if let Ok(log_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("cliverge-gui.log")
        {
            tracing_subscriber::fmt()
                .with_writer(log_file.with_max_level(tracing::Level::INFO))
                .init();
        } else {
            tracing_subscriber::fmt::init();
        }
    }
    
    #[cfg(not(windows))]
    {
        tracing_subscriber::fmt::init();
    }
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("CLIverge - AI CLI Tool Manager"),
        ..Default::default()
    };
    
    eframe::run_native(
        "CLIverge - AI CLI Tool Manager",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Box::new(CLIvergeApp::new())
        }),
    )
}
