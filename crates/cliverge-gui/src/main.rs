// Set Windows subsystem to "windows" to prevent console window from appearing
#![cfg_attr(windows, windows_subsystem = "windows")]

mod cache;
mod settings;
mod terminal;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use eframe::egui;
use cliverge_tools::get_builtin_tools;
use cliverge_sdk::{ToolStatus, InstallConfig};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use cache::{ToolCache, CachedToolInfo, version_checker};
use chrono::Utc;
use settings::{AppSettings, Theme};
use terminal::Terminal;
use tokio::task::JoinHandle;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub enum AppView {
    Main,
    Settings,
    Terminal,
    About,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub description: String,
    pub website: String,
    pub installing: bool,
    pub uninstalling: bool,
    pub updating: bool,
    pub latest_version: Option<String>,
    pub has_update: bool,
}

pub struct CLIvergeApp {
    tools: Arc<Mutex<Vec<ToolInfo>>>,
    selected_tool: Option<String>,
    output_log: Arc<Mutex<Vec<String>>>,
    runtime: Arc<tokio::runtime::Runtime>,
    search_query: String,
    show_only_installed: bool,
    status_sender: UnboundedSender<(String, String, ToolStatus, Option<String>)>,
    status_receiver: UnboundedReceiver<(String, String, ToolStatus, Option<String>)>,
    cache: Arc<Mutex<ToolCache>>,
    settings: AppSettings,
    terminal: Terminal,
    current_view: AppView,
    shutdown_flag: Arc<AtomicBool>,
    background_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    temp_settings: AppSettings,
}

impl Default for CLIvergeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CLIvergeApp {
    fn drop(&mut self) {
        // Signal shutdown to all background tasks
        self.shutdown_flag.store(true, Ordering::Relaxed);
        
        // Cancel all background tasks
        if let Ok(mut tasks) = self.background_tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}

impl CLIvergeApp {
    pub fn new() -> Self {
        // Set environment variable to indicate we're running in GUI mode
        std::env::set_var("CLIVERGE_GUI_MODE", "1");
        
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let (status_sender, status_receiver) = unbounded_channel();
        let cache = Arc::new(Mutex::new(ToolCache::default()));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        
        // Load settings
        let settings = runtime.block_on(async {
            AppSettings::load().await
        });
        
        let temp_settings = settings.clone();
        
        let app = Self {
            tools: Arc::new(Mutex::new(Vec::new())),
            selected_tool: None,
            output_log: Arc::new(Mutex::new(Vec::new())),
            runtime: runtime.clone(),
            search_query: String::new(),
            show_only_installed: false,
            status_sender,
            status_receiver,
            cache: cache.clone(),
            settings,
            terminal: Terminal::new(),
            current_view: AppView::Main,
            shutdown_flag: shutdown_flag.clone(),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            temp_settings,
        };
        
        // Initialize tools data
        app.refresh_tools();
        
        // Start periodic update checker if enabled
        if app.settings.auto_check_updates {
            app.start_periodic_checker();
        }
        
        app
    }
    
    fn start_periodic_checker(&self) {
        let sender = self.status_sender.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let interval_minutes = self.settings.update_check_interval_minutes as u64;
        
        let handle = self.runtime.spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_minutes * 60)
            );
            interval.tick().await; // Skip first immediate tick
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if shutdown_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        
                        // Check all tools for updates
                        let builtin_tools = get_builtin_tools();
                        for tool in builtin_tools {
                            if shutdown_flag.load(Ordering::Relaxed) {
                                break;
                            }
                            
                            let tool_id = tool.id().to_string();
                            let tool_name = tool.name().to_string();
                            let sender = sender.clone();
                            let shutdown_flag_clone = shutdown_flag.clone();
                            
                            tokio::spawn(async move {
                                if shutdown_flag_clone.load(Ordering::Relaxed) {
                                    return;
                                }
                                
                                let status = tool.status().await.unwrap_or(ToolStatus::Error("Check failed".to_string()));
                                let latest_version = version_checker::get_latest_version(&tool_id).await;
                                let _ = sender.send((tool_id, tool_name, status, latest_version));
                            });
                        }
                    }
                }
            }
        });
        
        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    fn refresh_tools(&self) {
        let tools_arc = Arc::clone(&self.tools);
        let log_arc = Arc::clone(&self.output_log);
        let sender = self.status_sender.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        
        let handle = self.runtime.spawn(async move {
            if shutdown_flag.load(Ordering::Relaxed) {
                return;
            }
            
            let cache = ToolCache::load().await;
            let builtin_tools = get_builtin_tools();
            let mut tool_info_list = Vec::new();
            
            // Populate tools with cached data or Loading status
            for tool in &builtin_tools {
                let cached_info = cache.as_ref().and_then(|c| c.get_tool(tool.id()));
                
                let tool_info = ToolInfo {
                    id: tool.id().to_string(),
                    name: tool.name().to_string(),
                    status: cached_info.map(|c| c.status.clone()).unwrap_or(ToolStatus::Loading),
                    description: Self::get_tool_description(tool.id()),
                    website: Self::get_tool_website(tool.id()),
                    installing: false,
                    uninstalling: false,
                    updating: false,
                    latest_version: cached_info.and_then(|c| c.latest_version.clone()),
                    has_update: false,
                };
                tool_info_list.push(tool_info);
            }
            
            // Update the tools list with cached or loading status
            if let Ok(mut tools) = tools_arc.lock() {
                *tools = tool_info_list;
            }
            
            // Log cache status
            if let Ok(mut log) = log_arc.lock() {
                if cache.is_some() {
                    log.push("💾 Loaded tool statuses from cache".to_string());
                }
                log.push("🔍 Checking for updates...".to_string());
            }
            
            // Now spawn parallel tasks to check each tool's status and latest version
            for tool in builtin_tools {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                
                let sender = sender.clone();
                let tool_id = tool.id().to_string();
                let tool_name = tool.name().to_string();
                let shutdown_flag_clone = shutdown_flag.clone();
                
                tokio::spawn(async move {
                    if shutdown_flag_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    
                    // Check current status
                    let status = tool.status().await.unwrap_or(ToolStatus::Error("Check failed".to_string()));
                    
                    // Check latest version from network
                    let latest_version = version_checker::get_latest_version(&tool_id).await;
                    
                    // Send the result back through the channel
                    let _ = sender.send((tool_id.clone(), tool_name.clone(), status.clone(), latest_version.clone()));
                });
            }
        });
        
        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }
    
    fn get_tool_description(tool_id: &str) -> String {
        match tool_id {
            "claude-code" => "Anthropic Claude AI Code Assistant - Generate, explain and optimize code".to_string(),
            "gemini-cli" => "Google Gemini AI Assistant - Multimodal AI code generation tool".to_string(),
            "qwen-code-cli" => "Alibaba Qwen Code Assistant - Chinese-friendly AI programming tool".to_string(),
            "openai-codex" => "OpenAI CodeX - Powerful code generation and completion tool".to_string(),
            "opencode" => "OpenCode Development Platform - Open source code generation and management platform".to_string(),
            "iflow" => "iFlow Intelligent Process Management - AI-driven workflow automation tool".to_string(),
            _ => "AI Development Tool".to_string(),
        }
    }
    
    fn get_tool_website(tool_id: &str) -> String {
        match tool_id {
            "claude-code" => "https://www.anthropic.com/claude".to_string(),
            "gemini-cli" => "https://ai.google.dev/".to_string(),
            "qwen-code-cli" => "https://qwen.alibaba.com/".to_string(),
            "openai-codex" => "https://openai.com/".to_string(),
            "opencode" => "https://github.com/opencode".to_string(),
            "iflow" => "https://iflow.ai/".to_string(),
            _ => "https://github.com/".to_string(),
        }
    }
    
    fn install_tool(&self, tool_id: String) {
        let tools_arc = Arc::clone(&self.tools);
        let log_arc = Arc::clone(&self.output_log);
        
        self.runtime.spawn(async move {
            Self::install_tool_async(tool_id, tools_arc, log_arc).await;
        });
    }
    
    async fn install_tool_async(tool_id: String, tools_arc: Arc<Mutex<Vec<ToolInfo>>>, log_arc: Arc<Mutex<Vec<String>>>) {
        // Set installing state and log start
        {
            if let Ok(mut tools) = tools_arc.lock() {
                if let Some(tool) = tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.installing = true;
                }
            }
            if let Ok(mut log) = log_arc.lock() {
                log.push(format!("🛠️ Starting installation of {}", tool_id));
            }
        }
        
        let builtin_tools = get_builtin_tools();
        if let Some(tool) = builtin_tools.into_iter().find(|t| t.id() == tool_id) {
            // First check if already installed
            match tool.detect().await {
                Ok(true) => {
                    if let Ok(mut log) = log_arc.lock() {
                        log.push(format!("ℹ️ {} is already installed", tool.name()));
                    }
                },
                _ => {
                    // Proceed with installation
                    if let Ok(mut log) = log_arc.lock() {
                        log.push(format!("💾 Installing {}, this may take a few moments...", tool.name()));
                    }
                    
                    match tool.install(&InstallConfig::default()).await {
                        Ok(_) => {
                            if let Ok(mut log) = log_arc.lock() {
                                log.push(format!("✅ {} installed successfully!", tool.name()));
                            }
                            // Verify installation
                            match tool.detect().await {
                                Ok(true) => {
                                    if let Ok(mut log) = log_arc.lock() {
                                        log.push(format!("✔️ {} installation verified", tool.name()));
                                    }
                                },
                                _ => {
                                    if let Ok(mut log) = log_arc.lock() {
                                        log.push(format!("⚠️ {} installation completed but verification failed", tool.name()));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if let Ok(mut log) = log_arc.lock() {
                                log.push(format!("❌ {} installation failed: {}", tool.name(), e));
                                log.push(format!("💡 Tip: Check if you have the required package managers installed (npm, pip, brew, etc.)"));
                            }
                        }
                    }
                }
            }
            
            // Update status and clear installing flag
            if let Ok(new_status) = tool.status().await {
                if let Ok(mut tools) = tools_arc.lock() {
                    if let Some(tool_info) = tools.iter_mut().find(|t| t.id == tool_id) {
                        tool_info.status = new_status;
                        tool_info.installing = false;
                    }
                }
            } else {
                // Just clear installing flag if status check fails
                if let Ok(mut tools) = tools_arc.lock() {
                    if let Some(tool_info) = tools.iter_mut().find(|t| t.id == tool_id) {
                        tool_info.installing = false;
                    }
                }
            }
        } else {
            if let Ok(mut log) = log_arc.lock() {
                log.push(format!("❌ Tool {} not found in registry", tool_id));
            }
            // Clear installing flag
            if let Ok(mut tools) = tools_arc.lock() {
                if let Some(tool_info) = tools.iter_mut().find(|t| t.id == tool_id) {
                    tool_info.installing = false;
                }
            }
        }
    }
    
    fn execute_tool_in_terminal(&mut self, tool_id: String) {
        if let Ok(tools) = self.tools.lock() {
            if let Some(tool) = tools.iter().find(|t| t.id == tool_id) {
                if matches!(tool.status, ToolStatus::Installed { .. }) {
                    self.current_view = AppView::Terminal;
                    self.terminal.input = format!("{} ", tool_id);
                }
            }
        }
    }
    
    fn render_main_view(&mut self, ctx: &egui::Context) {
        // Top panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 CLIverge - AI CLI Tool Manager");
                ui.separator();
                
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_tools();
                }
                
                ui.separator();
                if ui.button("⚙ Settings").clicked() {
                    self.current_view = AppView::Settings;
                    self.temp_settings = self.settings.clone();
                }
                
                if ui.button("💻 Terminal").clicked() {
                    self.current_view = AppView::Terminal;
                }
                
                if ui.button("ℹ About").clicked() {
                    self.current_view = AppView::About;
                }
                
                ui.separator();
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                
                ui.separator();
                ui.checkbox(&mut self.show_only_installed, "Show only installed");
            });
        });

        // Left panel - Tools list
        egui::SidePanel::left("left_panel").resizable(true).show(ctx, |ui| {
            ui.heading("🤖 AI CLI Tools");
            ui.separator();
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Ok(tools) = self.tools.lock() {
                    let filtered_tools: Vec<_> = tools.iter()
                        .filter(|tool| {
                            let matches_search = if self.search_query.is_empty() {
                                true
                            } else {
                                tool.name.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                                tool.id.to_lowercase().contains(&self.search_query.to_lowercase())
                            };
                            
                            let matches_filter = if self.show_only_installed {
                                matches!(tool.status, ToolStatus::Installed { .. })
                            } else {
                                true
                            };
                            
                            matches_search && matches_filter
                        })
                        .collect();
                        
                    for tool in filtered_tools {
                        let (status_icon, status_color) = Self::get_status_icon_and_color(&tool.status);
                        
                        ui.horizontal(|ui| {
                            ui.colored_label(status_color, status_icon);
                            
                            // Show update indicator if available
                            if tool.has_update {
                                ui.colored_label(egui::Color32::from_rgb(251, 191, 36), "🆕");
                            }
                            
                            let is_selected = self.selected_tool.as_ref() == Some(&tool.id);
                            if ui.selectable_label(is_selected, &tool.name).clicked() {
                                self.selected_tool = Some(tool.id.clone());
                            }
                        });
                        
                        ui.small(&tool.description);
                        ui.separator();
                    }
                }
            });
        });

        // Central panel - Tool details and actions
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(selected_id) = &self.selected_tool.clone() {
                // Get tool info and clone what we need to avoid borrow issues
                let tool_info = if let Ok(tools) = self.tools.lock() {
                    tools.iter().find(|t| &t.id == selected_id).cloned()
                } else {
                    None
                };
                
                if let Some(tool) = tool_info {
                        ui.heading(&tool.name);
                        ui.label(&tool.description);
                        
                        // Official website
                        ui.horizontal(|ui| {
                            ui.label("🌐 Official Website:");
                            if ui.hyperlink(&tool.website).clicked() {
                                let _ = webbrowser::open(&tool.website);
                            }
                        });
                        
                        ui.separator();
                        
                        // Status and version information
                        ui.horizontal(|ui| {
                            let (status_icon, status_color) = Self::get_status_icon_and_color(&tool.status);
                            ui.colored_label(status_color, status_icon);
                            ui.label(format!("Status: {}", 
                                Self::get_status_text(&tool.status)));
                        });
                        
                        // Show latest version if available
                        if let Some(latest) = &tool.latest_version {
                            ui.horizontal(|ui| {
                                ui.label("Latest Version:");
                                ui.strong(format!("v{}", latest));
                                
                                if tool.has_update {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(251, 191, 36),
                                        "🆕 Update Available"
                                    );
                                }
                            });
                        }
                        
                        ui.separator();
                        
                        // Action buttons
                        ui.horizontal(|ui| {
                            match &tool.status {
                                ToolStatus::Loading => {
                                    ui.spinner();
                                    ui.label("Checking status...");
                                }
                                ToolStatus::NotInstalled => {
                                    if tool.installing {
                                        ui.spinner();
                                        ui.label("Installing...");
                                    } else if ui.button("📥 Install").clicked() {
                                        self.install_tool(tool.id.clone());
                                    }
                                }
                                ToolStatus::Installed { .. } => {
                                    if ui.button("💻 Open Terminal").clicked() {
                                        self.execute_tool_in_terminal(tool.id.clone());
                                    }
                                    
                                    if tool.uninstalling {
                                        ui.spinner();
                                        ui.label("Uninstalling...");
                                    } else if ui.button("🗑 Uninstall").clicked() {
                                        // uninstall implementation
                                    }
                                    
                                    if tool.updating {
                                        ui.spinner();
                                        ui.label("Updating...");
                                    } else {
                                        let update_button = if tool.has_update {
                                            ui.add(egui::Button::new("🔄 Update")
                                                .fill(egui::Color32::from_rgb(251, 191, 36)))
                                        } else {
                                            ui.button("🔄 Check Updates")
                                        };
                                        
                                        if update_button.clicked() {
                                            if tool.has_update {
                                                // update implementation
                                            } else {
                                                self.refresh_tools();
                                            }
                                        }
                                    }
                                }
                                ToolStatus::Error(_) => {
                                    // error handling
                                }
                            }
                        });
                        
                        ui.separator();
                        
                        // Tool help section
                        ui.collapsing("📶 Tool Help", |ui| {
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                let builtin_tools = get_builtin_tools();
                                if let Some(actual_tool) = builtin_tools.iter().find(|t| t.id() == tool.id) {
                                    ui.label(actual_tool.help());
                                }
                            });
                        });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Welcome to CLIverge");
                    ui.label("Select a tool from the left panel to see details and actions.");
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Refresh Tool List").clicked() {
                            self.refresh_tools();
                        }
                    });
                });
            }
        });

        // Bottom panel - Output log
        egui::TopBottomPanel::bottom("bottom_panel").resizable(true).show(ctx, |ui| {
            ui.heading("📋 Operation Log");
            ui.separator();
            
            egui::ScrollArea::vertical().max_height(150.0).stick_to_bottom(true).show(ui, |ui| {
                if let Ok(log) = self.output_log.lock() {
                    for entry in log.iter() {
                        ui.label(entry);
                    }
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Clear Log").clicked() {
                    if let Ok(mut log) = self.output_log.lock() {
                        log.clear();
                    }
                }
            });
        });
    }
    
    fn render_settings_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("⚙ Settings");
            ui.separator();
            
            // Theme selection
            ui.horizontal(|ui| {
                ui.label("Theme:");
                ui.radio_value(&mut self.temp_settings.theme, Theme::Light, "Light");
                ui.radio_value(&mut self.temp_settings.theme, Theme::Dark, "Dark");
            });
            
            ui.separator();
            
            // Update check settings
            ui.checkbox(&mut self.temp_settings.auto_check_updates, "Automatically check for updates");
            
            if self.temp_settings.auto_check_updates {
                ui.horizontal(|ui| {
                    ui.label("Check interval (minutes):");
                    ui.add(egui::DragValue::new(&mut self.temp_settings.update_check_interval_minutes)
                        .clamp_range(5..=1440) // 5 minutes to 24 hours
                        .speed(1.0));
                });
            }
            
            ui.separator();
            
            // Buttons
            ui.horizontal(|ui| {
                if ui.button("Save Settings").clicked() {
                    self.settings = self.temp_settings.clone();
                    
                    // Save settings to disk
                    let settings_clone = self.settings.clone();
                    self.runtime.spawn(async move {
                        let _ = settings_clone.save().await;
                    });
                    
                    self.current_view = AppView::Main;
                }
                
                if ui.button("Cancel").clicked() {
                    self.current_view = AppView::Main;
                }
            });
        });
    }
    
    fn render_terminal_view(&mut self, ctx: &egui::Context) {
        // Top toolbar
        egui::TopBottomPanel::top("terminal_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("◀ Back").clicked() {
                    self.current_view = AppView::Main;
                }
                ui.separator();
                ui.label("Terminal");
                ui.separator();
                
                // Terminal tabs (for future expansion)
                let _ = ui.selectable_label(true, "PowerShell");
                
                // Right-aligned controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text("Close Terminal").clicked() {
                        self.current_view = AppView::Main;
                    }
                    if ui.button("🗑").on_hover_text("Clear Terminal").clicked() {
                        self.terminal.clear();
                    }
                    if ui.button("⚙").on_hover_text("Terminal Settings").clicked() {
                        // Future: Terminal settings
                    }
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Create a dark background for the terminal area
            let terminal_bg = if self.settings.theme == Theme::Dark {
                egui::Color32::from_rgb(30, 30, 30)
            } else {
                egui::Color32::from_rgb(18, 18, 18)
            };
            
            // Terminal output area
            let available_height = ui.available_height() - 40.0; // Reserve space for input
            
            egui::Frame::none()
                .fill(terminal_bg)
                .inner_margin(egui::style::Margin::same(10.0))
                .rounding(egui::Rounding::same(5.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .stick_to_bottom(true)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            // Display current directory
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 200, 100),
                                format!("PS {}>", std::env::current_dir()
                                    .unwrap_or_default()
                                    .display())
                            );
                            
                            // Display command history and output
                            if let Ok(output) = self.terminal.output.lock() {
                                if output.is_empty() {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(150, 150, 150),
                                        "Type a command to get started..."
                                    );
                                } else {
                                    for line in output.iter() {
                                        // Color code different types of output
                                        let (color, text) = if line.starts_with(">") {
                                            // Command
                                            (egui::Color32::from_rgb(200, 200, 200), line.clone())
                                        } else if line.starts_with("ERROR:") || line.starts_with("Error:") {
                                            // Error
                                            (egui::Color32::from_rgb(255, 100, 100), line.clone())
                                        } else if line.starts_with("WARNING:") || line.starts_with("Warning:") {
                                            // Warning
                                            (egui::Color32::from_rgb(255, 200, 100), line.clone())
                                        } else if line.starts_with("SUCCESS:") || line.starts_with("✓") {
                                            // Success
                                            (egui::Color32::from_rgb(100, 255, 100), line.clone())
                                        } else {
                                            // Normal output
                                            (egui::Color32::from_rgb(220, 220, 220), line.clone())
                                        };
                                        
                                        ui.horizontal(|ui| {
                                            ui.add_space(5.0);
                                            ui.colored_label(color, text);
                                        });
                                    }
                                }
                            }
                            
                            // Show running indicator inline
                            if self.terminal.is_running() {
                                ui.horizontal(|ui| {
                                    ui.add_space(5.0);
                                    ui.spinner();
                                    ui.colored_label(
                                        egui::Color32::from_rgb(100, 200, 255),
                                        "Running..."
                                    );
                                });
                            }
                        });
                });
            
            // Input area at the bottom
            ui.add_space(5.0);
            
            egui::Frame::none()
                .fill(terminal_bg)
                .inner_margin(egui::style::Margin::symmetric(10.0, 5.0))
                .rounding(egui::Rounding::same(5.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Prompt
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 200, 100),
                            ">"
                        );
                        
                        // Input field with custom style
                        let input_response = ui.add(
                            egui::TextEdit::singleline(&mut self.terminal.input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(ui.available_width() - 100.0)
                                .hint_text("Enter command...")
                        );
                        
                        // Handle Enter key
                        if input_response.lost_focus() 
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)) 
                            && !self.terminal.input.is_empty() 
                            && !self.terminal.is_running() {
                            
                            // Add command to output history
                            if let Ok(mut output) = self.terminal.output.lock() {
                                output.push(format!("> {}", self.terminal.input));
                            }
                            
                            // Parse and execute command
                            let parts: Vec<&str> = self.terminal.input.split_whitespace().collect();
                            if let Some(tool_id) = parts.first() {
                                let args = parts[1..].join(" ");
                                let terminal_clone = self.terminal.clone();
                                let tool_id = tool_id.to_string();
                                
                                self.runtime.spawn(async move {
                                    terminal_clone.execute_command(&tool_id, &args).await;
                                });
                            }
                            self.terminal.input.clear();
                            input_response.request_focus();
                        }
                        
                        // Run button
                        if ui.button("▶").on_hover_text("Run Command").clicked() 
                            && !self.terminal.input.is_empty() 
                            && !self.terminal.is_running() {
                            
                            // Add command to output history
                            if let Ok(mut output) = self.terminal.output.lock() {
                                output.push(format!("> {}", self.terminal.input));
                            }
                            
                            let parts: Vec<&str> = self.terminal.input.split_whitespace().collect();
                            if let Some(tool_id) = parts.first() {
                                let args = parts[1..].join(" ");
                                let terminal_clone = self.terminal.clone();
                                let tool_id = tool_id.to_string();
                                
                                self.runtime.spawn(async move {
                                    terminal_clone.execute_command(&tool_id, &args).await;
                                });
                            }
                            self.terminal.input.clear();
                        }
                    });
                });
        });
    }
    
    fn render_about_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("ℹ About CLIverge");
                ui.separator();
                
                ui.label(format!("Version: {}", VERSION));
                ui.add_space(10.0);
                
                ui.label("CLIverge is a comprehensive AI CLI tool manager that simplifies the installation and management of various AI development tools.");
                ui.add_space(20.0);
                
                ui.group(|ui| {
                    ui.label("📋 Key Features:");
                    ui.label("• One-click installation of AI development tools");
                    ui.label("• Real-time status monitoring and updates");
                    ui.label("• Integrated terminal for direct tool usage");
                    ui.label("• Automatic update notifications");
                    ui.label("• Clean and intuitive user interface");
                });
                
                ui.add_space(20.0);
                
                ui.group(|ui| {
                    ui.label("🤖 Supported AI Tools:");
                    ui.label("• Claude Code (Anthropic)");
                    ui.label("• Gemini CLI (Google)");
                    ui.label("• Qwen Code CLI (Alibaba)");
                    ui.label("• OpenAI Codex");
                    ui.label("• And many more...");
                });
                
                ui.add_space(20.0);
                
                if ui.button("OK").clicked() {
                    self.current_view = AppView::Main;
                }
            });
        });
    }
    
    fn get_status_icon_and_color(status: &ToolStatus) -> (&str, egui::Color32) {
        match status {
            ToolStatus::Loading => ("⏳", egui::Color32::from_rgb(156, 163, 175)),
            ToolStatus::Installed { .. } => ("✅", egui::Color32::from_rgb(34, 197, 94)),
            ToolStatus::NotInstalled => ("❌", egui::Color32::from_rgb(239, 68, 68)),
            ToolStatus::Error(_) => ("⚠️", egui::Color32::from_rgb(245, 158, 11)),
        }
    }
    
    fn get_status_text(status: &ToolStatus) -> String {
        match status {
            ToolStatus::Loading => "Loading...".to_string(),
            ToolStatus::Installed { version } => format!("Installed (v{})", version),
            ToolStatus::NotInstalled => "Not Installed".to_string(),
            ToolStatus::Error(msg) => format!("Error: {}", msg),
        }
    }
}

impl eframe::App for CLIvergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        self.settings.apply_theme(ctx);
        
        // Process incoming status updates from background tasks
        let mut any_updates = false;
        while let Ok((tool_id, tool_name, status, latest_version)) = self.status_receiver.try_recv() {
            if let Ok(mut tools) = self.tools.lock() {
                if let Some(tool) = tools.iter_mut().find(|t| t.id == tool_id) {
                    // Check if there's an update available
                    let has_update = match (&status, &latest_version) {
                        (ToolStatus::Installed { version }, Some(latest)) => {
                            version != latest
                        }
                        _ => false,
                    };
                    
                    tool.status = status.clone();
                    tool.latest_version = latest_version.clone();
                    tool.has_update = has_update;
                    any_updates = true;
                    
                    // Update cache
                    let cache_arc = Arc::clone(&self.cache);
                    if let Ok(mut cache) = cache_arc.lock() {
                        let cached_info = CachedToolInfo {
                            id: tool_id.clone(),
                            name: tool_name.clone(),
                            status: status.clone(),
                            latest_version: latest_version.clone(),
                            cached_at: Utc::now(),
                            last_checked: Utc::now(),
                        };
                        cache.update_tool(cached_info);
                    }
                    
                    // Save cache asynchronously
                    let cache_data = if let Ok(cache) = self.cache.lock() {
                        cache.clone()
                    } else {
                        continue;
                    };
                    self.runtime.spawn(async move {
                        let _ = cache_data.save().await;
                    });
                }
            }
        }
        
        // Request repaint if we received updates
        if any_updates {
            ctx.request_repaint();
        }
        
        // Render based on current view
        match self.current_view {
            AppView::Main => self.render_main_view(ctx),
            AppView::Settings => self.render_settings_view(ctx),
            AppView::Terminal => self.render_terminal_view(ctx),
            AppView::About => self.render_about_view(ctx),
        }
    }
}

fn setup_fonts(ctx: &egui::Context) {
    // Note: Chinese font loading removed for now since we're using English-only interface
    // TODO: Add Chinese font support back if internationalization is added later
    
    // Configure text styles for better readability
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "CLIverge - AI CLI Tool Manager",
        options,
        Box::new(|cc| {
            // Configure fonts for Chinese support
            setup_fonts(&cc.egui_ctx);
            Box::new(CLIvergeApp::new())
        }),
    )
}
