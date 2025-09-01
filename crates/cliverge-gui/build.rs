// Build script for Windows resource file
#[cfg(windows)]
fn main() {
    use std::env;
    use std::path::Path;
    
    // Only build resources on Windows
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        // Set basic application information
        res.set_icon("assets/app_icon.ico")
           .set("ProductName", "CLIverge")
           .set("FileDescription", "AI CLI Tool Manager")
           .set("CompanyName", "CLIverge Team")
           .set("LegalCopyright", "Copyright (C) 2024 CLIverge Team")
           .set("FileVersion", env!("CARGO_PKG_VERSION"))
           .set("ProductVersion", env!("CARGO_PKG_VERSION"));
        
        // Check if icon file exists, if not, create a basic resource without icon
        if !Path::new("assets/app_icon.ico").exists() {
            println!("cargo:warning=Icon file not found, skipping icon resource");
            res = winres::WindowsResource::new();
            res.set("ProductName", "CLIverge")
               .set("FileDescription", "AI CLI Tool Manager")
               .set("CompanyName", "CLIverge Team")
               .set("LegalCopyright", "Copyright (C) 2024 CLIverge Team")
               .set("FileVersion", env!("CARGO_PKG_VERSION"))
               .set("ProductVersion", env!("CARGO_PKG_VERSION"));
        }
        
        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to compile resource: {}", e);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    // Do nothing on non-Windows platforms
}
