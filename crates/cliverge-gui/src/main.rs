// Set Windows subsystem to "windows" to prevent console window from appearing
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;

use app::CLIvergeApp;
use eframe::egui;
use egui::IconData;

/// Load application icon from embedded data
fn load_icon() -> IconData {
    // Create a simple 32x32 icon programmatically
    // Blue-purple gradient background with terminal-like appearance
    let size = 32;
    let mut rgba = Vec::with_capacity(size * size * 4);
    
    for y in 0..size {
        for x in 0..size {
            // Create circular mask
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance <= 15.0 {
                // Inside circle - blue/purple gradient
                let t = distance / 15.0;
                let r = (63.0 + t * (139.0 - 63.0)) as u8;  // 63->139 (blue to purple)
                let g = (102.0 + t * (92.0 - 102.0)) as u8; // 102->92
                let b = (241.0 + t * (246.0 - 241.0)) as u8; // 241->246
                
                // Add terminal window appearance
                if y >= 8 && y <= 24 && x >= 6 && x <= 26 {
                    // Terminal window area
                    if y <= 11 {
                        // Title bar
                        rgba.extend_from_slice(&[55, 65, 81, 255]);
                    } else {
                        // Terminal content area
                        rgba.extend_from_slice(&[30, 41, 59, 255]);
                    }
                } else {
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            } else {
                // Outside circle - transparent
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    
    IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
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
    
    // Load application icon
    let icon_data = load_icon();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("CLIverge - AI CLI Tool Manager")
            .with_icon(icon_data),
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
