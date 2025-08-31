use cliverge_core::{
    ConfigManager, ToolManager, ToolInfo, ToolStatus, VersionCheckStrategy,
};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub enum AppView {
    Main,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub struct UiState {
    selected_tool: Option<String>,
    search_query: String,
    show_only_installed: bool,
    settings_window_open: bool,
    notifications: Vec<Notification>,
    current_view: AppView,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_tool: None,
            search_query: String::new(),
            show_only_installed: false,
            settings_window_open: false,
            notifications: Vec::new(),
            current_view: AppView::Main,
        }
    }
}

pub struct CLIvergeApp {
    config_manager: Arc<Mutex<ConfigManager>>,
    tool_manager: ToolManager,
    ui_state: UiState,
    runtime: Arc<tokio::runtime::Runtime>,
    background_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    tools_cache: Arc<Mutex<Vec<ToolInfo>>>,
    ctx: Option<egui::Context>,
}

impl CLIvergeApp {
    fn create_minimal_config_manager() -> ConfigManager {
        use cliverge_core::{AppSettings, AppearanceSettings, BehaviorSettings, PathSettings};
        
        let app_settings = AppSettings {
            appearance: AppearanceSettings {
                theme: "dark".to_string(),
                font_size: 14.0,
                window_size: [1200.0, 800.0],
            },
            behavior: BehaviorSettings {
                auto_check_updates: false,
                check_interval_minutes: 30,
                show_notifications: true,
            },
            paths: PathSettings {
                tools_config_path: "tools.json".to_string(),
                data_directory: "~/.cliverge".to_string(),
            },
        };
        
        // Create a ConfigManager and try to load the embedded tools config
        let mut config_manager = ConfigManager::new_with_settings(app_settings);
        
        // Try to manually load embedded tools config
        if let Ok(tools_config) = Self::load_embedded_tools_config_directly() {
            config_manager.set_tools_config(tools_config);
        }
        
        config_manager
    }
    
    fn load_embedded_tools_config_directly() -> Result<cliverge_core::ToolsConfig, Box<dyn std::error::Error>> {
        use cliverge_core::ToolsConfig;
        
        let default_config_paths = [
            "./configs/tools.json",
            "../configs/tools.json", 
            "../../configs/tools.json",
        ];
        
        for path in &default_config_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<ToolsConfig>(&content) {
                    return Ok(config);
                }
            }
        }
        
        Err("No embedded tools config found".into())
    }
    
    pub fn new() -> Self {
        let runtime = Arc::new(
            tokio::runtime::Runtime::new()
                .expect("Failed to create async runtime")
        );

        // Create configuration manager
        let config_manager = runtime.block_on(async {
            match ConfigManager::load().await {
                Ok(cm) => cm,
                Err(_) => Self::create_minimal_config_manager(),
            }
        });
        
        let config_manager = Arc::new(Mutex::new(config_manager));
        let tool_manager = ToolManager::new(Arc::clone(&config_manager));

        let app = Self {
            config_manager,
            tool_manager: tool_manager.clone(),
            ui_state: UiState::default(),
            runtime: runtime.clone(),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            tools_cache: Arc::new(Mutex::new(Vec::new())),
            ctx: None,
        };

        // Load tool configs immediately without status checking (non-blocking)
        let tool_configs = tool_manager.get_all_tools_configs().unwrap_or_default();
        if let Ok(mut cache) = app.tools_cache.lock() {
            *cache = tool_configs;
        }

        app
    }

    /// Start background status checking for all tools in parallel
    fn start_background_status_checking(&self) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();

        // Get list of tool IDs to check
        let tool_ids: Vec<String> = if let Ok(cache) = self.tools_cache.lock() {
            cache.iter().map(|tool| tool.config.id.clone()).collect()
        } else {
            Vec::new()
        };

        // Start a background task for each tool to check its status in parallel
        for tool_id in tool_ids {
            let tool_manager_clone = tool_manager.clone();
            let tools_cache_clone = Arc::clone(&tools_cache);
            let tool_id_clone = tool_id.clone();
            let ctx_clone = ctx.clone();

            let handle = runtime.spawn(async move {
                match tool_manager_clone.check_tool_status(&tool_id_clone).await {
                    Ok(status) => {
                        // Update the specific tool in the cache
                        if let Ok(mut cache) = tools_cache_clone.lock() {
                            for tool in cache.iter_mut() {
                                if tool.config.id == tool_id_clone {
                                    tool.status = status;
                                    break;
                                }
                            }
                        }
                        
                        // Request UI repaint after status update
                        if let Some(context) = &ctx_clone {
                            context.request_repaint();
                        }
                        
                        tracing::debug!("Background status check completed for tool: {}", tool_id_clone);
                    }
                    Err(e) => {
                        tracing::error!("Background status check failed for {}: {}", tool_id_clone, e);
                        // Update with error status
                        if let Ok(mut cache) = tools_cache_clone.lock() {
                            for tool in cache.iter_mut() {
                                if tool.config.id == tool_id_clone {
                                    tool.status = ToolStatus::Error(format!("Status check failed: {}", e));
                                    break;
                                }
                            }
                        }
                        
                        // Request UI repaint after status update
                        if let Some(context) = &ctx_clone {
                            context.request_repaint();
                        }
                    }
                }
            });

            if let Ok(mut tasks) = self.background_tasks.lock() {
                tasks.push(handle);
            }
        }
    }

    pub fn refresh_tools(&self) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);
        let runtime = Arc::clone(&self.runtime);

        let handle = runtime.spawn(async move {
            match tool_manager.get_all_tools().await {
                Ok(tools) => {
                    if let Ok(mut cache) = tools_cache.lock() {
                        *cache = tools;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to refresh tools: {}", e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    pub fn install_tool(&self, tool_id: String) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);

        let handle = self.runtime.spawn(async move {
            tracing::info!("Installing tool: {}", tool_id);
            match tool_manager.install_tool(&tool_id).await {
                Ok(_) => {
                    tracing::info!("Tool {} installed successfully", tool_id);
                    // Refresh tools after installation
                    if let Ok(tools) = tool_manager.get_all_tools().await {
                        if let Ok(mut cache) = tools_cache.lock() {
                            *cache = tools;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to install tool {}: {}", tool_id, e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    pub fn uninstall_tool(&self, tool_id: String) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);

        let handle = self.runtime.spawn(async move {
            tracing::info!("Uninstalling tool: {}", tool_id);
            match tool_manager.uninstall_tool(&tool_id).await {
                Ok(_) => {
                    tracing::info!("Tool {} uninstalled successfully", tool_id);
                    // Refresh tools after uninstallation
                    if let Ok(tools) = tool_manager.get_all_tools().await {
                        if let Ok(mut cache) = tools_cache.lock() {
                            *cache = tools;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to uninstall tool {}: {}", tool_id, e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    pub fn check_version_updates(&self, tool_id: String) {
        let tool_manager = self.tool_manager.clone();

        let handle = self.runtime.spawn(async move {
            match tool_manager.check_version_updates(&tool_id, VersionCheckStrategy::Auto).await {
                Ok(version_info) => {
                    tracing::info!("Version info for {}: {:?}", tool_id, version_info);
                }
                Err(e) => {
                    tracing::error!("Failed to check version updates for {}: {}", tool_id, e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    fn render_tool_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("🤖 AI CLI Tools");
        ui.separator();

        // Search and filter controls
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.ui_state.search_query);
            ui.checkbox(&mut self.ui_state.show_only_installed, "Show only installed");
        });

        ui.separator();

        // Clone necessary data to avoid borrow conflicts
        let tools_data = if let Ok(tools) = self.tools_cache.lock() {
            tools.clone()
        } else {
            Vec::new()
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            if !tools_data.is_empty() {
                let filtered_tools: Vec<_> = tools_data.iter()
                    .filter(|tool| {
                        let matches_search = if self.ui_state.search_query.is_empty() {
                            true
                        } else {
                            tool.config.name.to_lowercase().contains(&self.ui_state.search_query.to_lowercase()) ||
                            tool.config.id.to_lowercase().contains(&self.ui_state.search_query.to_lowercase())
                        };

                        let matches_filter = if self.ui_state.show_only_installed {
                            matches!(tool.status, ToolStatus::Installed { .. })
                        } else {
                            true
                        };

                        matches_search && matches_filter
                    })
                    .collect();

                for tool in filtered_tools {
                    self.render_tool_item(ui, tool);
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.label("Loading tools...");
                });
            }
        });
    }

    fn render_tool_item(&mut self, ui: &mut egui::Ui, tool: &ToolInfo) {
        let (status_icon, status_color) = Self::get_status_icon_and_color(&tool.status);

        ui.horizontal(|ui| {
            ui.colored_label(status_color, status_icon);

            let is_selected = self.ui_state.selected_tool.as_ref() == Some(&tool.config.id);
            if ui.selectable_label(is_selected, &tool.config.name).clicked() {
                self.ui_state.selected_tool = Some(tool.config.id.clone());
            }
        });

        ui.small(&tool.config.description);
        ui.separator();
    }

    fn render_tool_details(&mut self, ui: &mut egui::Ui) {
        if let Some(selected_id) = &self.ui_state.selected_tool.clone() {
            let tool_data = if let Ok(tools) = self.tools_cache.lock() {
                tools.iter().find(|t| &t.config.id == selected_id).cloned()
            } else {
                None
            };

            if let Some(tool) = tool_data {
                ui.heading(&tool.config.name);
                ui.label(&tool.config.description);

                ui.horizontal(|ui| {
                    ui.label("🌐 Website:");
                    if ui.hyperlink(&tool.config.website).clicked() {
                        let _ = webbrowser::open(&tool.config.website);
                    }
                });

                ui.separator();

                // Status display
                ui.horizontal(|ui| {
                    let (status_icon, status_color) = Self::get_status_icon_and_color(&tool.status);
                    ui.colored_label(status_color, status_icon);
                    ui.label(format!("Status: {}", Self::get_status_text(&tool.status)));
                });

                ui.separator();

                // Action buttons
                self.render_tool_actions(ui, &tool);

                return;
            }
        }

        // Default view when no tool is selected
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Welcome to CLIverge");
            ui.label("Select a tool from the list to see details and actions.");
            ui.add_space(20.0);

            if ui.button("🔄 Refresh Tool List").clicked() {
                self.refresh_tools();
            }
        });
    }

    fn render_tool_actions(&mut self, ui: &mut egui::Ui, tool: &ToolInfo) {
        ui.horizontal(|ui| {
            match &tool.status {
                ToolStatus::Unknown => {
                    ui.spinner();
                    ui.label("Checking status...");
                }
                ToolStatus::NotInstalled => {
                    if ui.button("📥 Install").clicked() {
                        self.install_tool(tool.config.id.clone());
                    }
                }
                ToolStatus::Installed { version } => {
                    ui.label(format!("Version: {}", version));
                    
                    if ui.button("🗑 Uninstall").clicked() {
                        self.uninstall_tool(tool.config.id.clone());
                    }
                    
                    if ui.button("🔄 Check Updates").clicked() {
                        self.check_version_updates(tool.config.id.clone());
                    }
                }
                ToolStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", msg));
                }
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙ Settings");
        ui.separator();

        if let Ok(config_manager) = self.config_manager.lock() {
            let settings = config_manager.get_app_settings();
            
            ui.label(format!("Theme: {}", settings.appearance.theme));
            ui.label(format!("Font Size: {}", settings.appearance.font_size));
            
            if settings.behavior.auto_check_updates {
                ui.label(format!("Auto update check: Every {} minutes", 
                    settings.behavior.check_interval_minutes));
            } else {
                ui.label("Auto update check: Disabled");
            }
        }

        ui.separator();

        if ui.button("◀ Back").clicked() {
            self.ui_state.current_view = AppView::Main;
        }
    }

    fn render_about(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("ℹ About CLIverge");
            ui.separator();

            ui.label(format!("Version: {}", VERSION));
            ui.add_space(10.0);

            ui.label("CLIverge is a lightweight AI CLI tool manager that simplifies");
            ui.label("the installation and management of various AI development tools.");
            ui.add_space(20.0);

            ui.group(|ui| {
                ui.label("📋 Key Features:");
                ui.label("• One-click installation of AI development tools");
                ui.label("• Real-time status monitoring");
                ui.label("• Version checking and updates");
                ui.label("• Clean and intuitive user interface");
            });

            ui.add_space(20.0);

            if ui.button("OK").clicked() {
                self.ui_state.current_view = AppView::Main;
            }
        });
    }

    fn get_status_icon_and_color(status: &ToolStatus) -> (&str, egui::Color32) {
        match status {
            ToolStatus::Unknown => ("⏳", egui::Color32::GRAY),
            ToolStatus::Installed { .. } => ("✅", egui::Color32::GREEN),
            ToolStatus::NotInstalled => ("❌", egui::Color32::RED),
            ToolStatus::Error(_) => ("⚠️", egui::Color32::YELLOW),
        }
    }

    fn get_status_text(status: &ToolStatus) -> String {
        match status {
            ToolStatus::Unknown => "Checking...".to_string(),
            ToolStatus::Installed { version } => format!("Installed (v{})", version),
            ToolStatus::NotInstalled => "Not Installed".to_string(),
            ToolStatus::Error(msg) => format!("Error: {}", msg),
        }
    }
}

impl eframe::App for CLIvergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Store context for background tasks to request repaints
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
            // Start background status checking now that we have the context
            self.start_background_status_checking();
        }
        
        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 CLIverge - AI CLI Tool Manager");
                
                ui.separator();
                
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_tools();
                }
                
                if ui.button("⚙ Settings").clicked() {
                    self.ui_state.current_view = AppView::Settings;
                }
                
                if ui.button("ℹ About").clicked() {
                    self.ui_state.current_view = AppView::About;
                }
            });
        });

        // Main content based on current view
        match self.ui_state.current_view {
            AppView::Main => {
                // Left panel - Tools list
                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .default_width(300.0)
                    .show(ctx, |ui| {
                        self.render_tool_list(ui);
                    });

                // Central panel - Tool details
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_tool_details(ui);
                });
            }
            AppView::Settings => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_settings(ui);
                });
            }
            AppView::About => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_about(ui);
                });
            }
        }
    }
}

impl Default for CLIvergeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CLIvergeApp {
    fn drop(&mut self) {
        // Clean up background tasks
        if let Ok(mut tasks) = self.background_tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}
