// Set Windows subsystem to "windows" to prevent console window from appearing
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;

struct MinimalApp;

impl eframe::App for MinimalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CLIverge Test App");
            ui.label("This is a minimal test to check if the GUI framework works.");
            if ui.button("Test Button").clicked() {
                println!("Button clicked!");
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("CLIverge Test"),
        ..Default::default()
    };
    
    eframe::run_native(
        "CLIverge Test",
        options,
        Box::new(|_cc| {
            Box::new(MinimalApp)
        }),
    )
}
