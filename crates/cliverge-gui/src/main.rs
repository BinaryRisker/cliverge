// Set Windows subsystem to "windows" to prevent console window from appearing
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;

use app::CLIvergeApp;

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
        Box::new(|_cc| {
            Box::new(CLIvergeApp::new())
        }),
    )
}
