use cliverge_core::{
    ConfigManager, ToolManager, ToolInfo, ToolStatus, CacheManager,
    AppSettings, AppearanceSettings, BehaviorSettings,
};
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusCheckProgress {
    pub tool_id: String,
    pub tool_name: String,
    pub status: ProgressStatus,
    pub message: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum ProgressStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

pub struct AppState {
    // UI State
    pub selected_tool: Option<String>,
    pub search_query: String,
    pub show_only_installed: bool,
    pub log_window_open: bool,
    pub notifications: Vec<Notification>,
    pub current_view: AppView,
    pub status_progress: Vec<StatusCheckProgress>,
    pub is_refreshing: bool,
    
    // Settings state (live editing)
    pub settings_theme: String,
    pub settings_font_size: f32,
    pub settings_auto_check_updates: bool,
    pub settings_check_interval: u32,
    pub settings_show_notifications: bool,
    pub settings_auto_refresh_on_startup: bool,
    pub settings_debug_mode: bool,
    pub settings_experimental_features: bool,
    
    // Auto-check timer state
    pub last_auto_check: Option<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_tool: None,
            search_query: String::new(),
            show_only_installed: false,
            log_window_open: false,
            notifications: Vec::new(),
            current_view: AppView::Main,
            status_progress: Vec::new(),
            is_refreshing: false,
            
            settings_theme: "dark".to_string(),
            settings_font_size: 14.0,
            settings_auto_check_updates: false,
            settings_check_interval: 30,
            settings_show_notifications: true,
            settings_auto_refresh_on_startup: true,
            settings_debug_mode: false,
            settings_experimental_features: false,
            last_auto_check: None,
        }
    }
}

pub struct CLIvergeApp {
    config_manager: Arc<Mutex<ConfigManager>>,
    cache_manager: Arc<Mutex<CacheManager>>,
    tool_manager: ToolManager,
    app_state: AppState,
    runtime: Arc<tokio::runtime::Runtime>,
    background_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    tools_cache: Arc<Mutex<Vec<ToolInfo>>>,
    help_cache: Arc<Mutex<HashMap<String, String>>>,
    progress_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<StatusCheckProgress>>>>,
    progress_receiver: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<StatusCheckProgress>>>>,
    ctx: Option<egui::Context>,
}

impl CLIvergeApp {
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

        // Create cache manager - use same config directory as ConfigManager
        let config_dir = Self::get_config_dir();
        let cache_path = config_dir.join("cache");
        let mut cache_manager = CacheManager::new(cache_path);
        
        // Load cache
        runtime.block_on(async {
            let _ = cache_manager.load().await;
        });

        let config_manager = Arc::new(Mutex::new(config_manager));
        let cache_manager = Arc::new(Mutex::new(cache_manager));
        let tool_manager = ToolManager::new(Arc::clone(&config_manager));

        // Create progress channel
        let (progress_sender, progress_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut app = Self {
            config_manager: Arc::clone(&config_manager),
            cache_manager,
            tool_manager: tool_manager.clone(),
            app_state: AppState::default(),
            runtime: runtime.clone(),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            tools_cache: Arc::new(Mutex::new(Vec::new())),
            help_cache: Arc::new(Mutex::new(HashMap::new())),
            progress_sender: Arc::new(Mutex::new(Some(progress_sender))),
            progress_receiver: Arc::new(Mutex::new(Some(progress_receiver))),
            ctx: None,
        };

        // Load settings into app state
        app.load_settings_into_state();
        
        // Initialize auto check timer if enabled
        if app.app_state.settings_auto_check_updates {
            app.start_auto_check_timer();
        }

        // Load initial tool configs with cached status
        app.load_tools_with_cache();

        app
    }

    fn create_minimal_config_manager() -> ConfigManager {
        use cliverge_core::PathSettings;
        
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
                auto_refresh_on_startup: true,
            },
            paths: PathSettings {
                tools_config_path: "tools.json".to_string(),
                data_directory: "~/.cliverge".to_string(),
            },
        };
        
        let mut config_manager = ConfigManager::new_with_settings(app_settings);
        
        // Try to load embedded tools config
        if let Ok(tools_config) = Self::load_embedded_tools_config() {
            config_manager.set_tools_config(tools_config);
        }
        
        config_manager
    }
    
    // Helper method to get config directory - same as ConfigManager
    fn get_config_dir() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".cliverge")
    }

    fn load_embedded_tools_config() -> Result<cliverge_core::ToolsConfig, Box<dyn std::error::Error>> {
        let default_config_paths = [
            "./configs/tools.json",
            "../configs/tools.json", 
            "../../configs/tools.json",
        ];
        
        for path in &default_config_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return Ok(config);
                }
            }
        }
        
        // Return minimal config if no file found
        Ok(cliverge_core::ToolsConfig {
            version: "1.0".to_string(),
            tools: vec![],
        })
    }

    fn load_settings_into_state(&mut self) {
        if let Ok(config) = self.config_manager.lock() {
            let settings = config.get_app_settings();
            self.app_state.settings_theme = settings.appearance.theme.clone();
            self.app_state.settings_font_size = settings.appearance.font_size;
            self.app_state.settings_auto_check_updates = settings.behavior.auto_check_updates;
            self.app_state.settings_check_interval = settings.behavior.check_interval_minutes;
            self.app_state.settings_show_notifications = settings.behavior.show_notifications;
            self.app_state.settings_auto_refresh_on_startup = settings.behavior.auto_refresh_on_startup;
        }
    }

    fn save_settings_from_state(&mut self) {
        if let Ok(mut config) = self.config_manager.lock() {
            let mut settings = config.get_app_settings().clone();
            
            settings.appearance.theme = self.app_state.settings_theme.clone();
            settings.appearance.font_size = self.app_state.settings_font_size;
            settings.behavior.auto_check_updates = self.app_state.settings_auto_check_updates;
            settings.behavior.check_interval_minutes = self.app_state.settings_check_interval;
            settings.behavior.show_notifications = self.app_state.settings_show_notifications;
            settings.behavior.auto_refresh_on_startup = self.app_state.settings_auto_refresh_on_startup;
            
            config.update_app_settings(settings);
            
            // Save asynchronously - use std::thread to avoid Send issues
            let config_manager = Arc::clone(&self.config_manager);
            std::thread::spawn(move || {
                // Create a new runtime for this thread
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(config) = config_manager.lock() {
                        if let Err(e) = config.save().await {
                            tracing::error!("Failed to save settings: {}", e);
                        } else {
                            tracing::info!("Settings saved successfully");
                        }
                    }
                });
            });
        }
    }

    fn load_tools_with_cache(&mut self) {
        let tool_infos = self.tool_manager.get_all_tools_configs().unwrap_or_default();
        
        // Load from cache first
        if let (Ok(mut tools), Ok(cache)) = (self.tools_cache.lock(), self.cache_manager.lock()) {
            *tools = tool_infos.into_iter().map(|mut tool_info| {
                let cached_status = cache.get_tool_status(&tool_info.config.id)
                    .unwrap_or(ToolStatus::Unknown);
                
                tool_info.status = cached_status;
                tool_info
            }).collect();
        }
    }

    pub fn start_background_status_checking(&mut self) {
        if self.ctx.is_none() {
            return; // No context yet, will be called later
        }

        let tools_to_check: Vec<String> = if let Ok(tools) = self.tools_cache.lock() {
            tools.iter().map(|tool| tool.config.id.clone()).collect()
        } else {
            Vec::new()
        };

        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);
        let cache_manager = Arc::clone(&self.cache_manager);
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();
        let sender = if let Ok(sender_guard) = self.progress_sender.lock() {
            sender_guard.clone()
        } else {
            None
        };

        for tool_id in tools_to_check {
            let tool_manager = tool_manager.clone();
            let tools_cache = Arc::clone(&tools_cache);
            let cache_manager = Arc::clone(&cache_manager);
            let tool_id = tool_id.clone();
            let ctx = ctx.clone();
            let sender = sender.clone();

            let handle = runtime.spawn(async move {
                // Send progress update - starting
                if let Some(sender) = &sender {
                    let _ = sender.send(StatusCheckProgress {
                        tool_id: tool_id.clone(),
                        tool_name: tool_id.clone(), // We'll get the real name from cache
                        status: ProgressStatus::InProgress,
                        message: "Checking status...".to_string(),
                        timestamp: Instant::now(),
                    });
                }

                match tool_manager.check_tool_status(&tool_id).await {
                    Ok(status) => {
                        // Update cache and save it
                        let cache_manager_clone = Arc::clone(&cache_manager);
                        let status_clone = status.clone();
                        let tool_id_for_cache = tool_id.clone();
                        
                        // Update cache in a separate thread to avoid Send issues
                        std::thread::spawn(move || {
                            // Create a new runtime for this thread
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                if let Ok(mut cache) = cache_manager_clone.lock() {
                                    cache.set_tool_status(&tool_id_for_cache, status_clone);
                                    if let Err(e) = cache.save().await {
                                        tracing::warn!("Failed to save cache: {}", e);
                                    }
                                }
                            });
                        });

                        // Update tools cache
                        if let Ok(mut tools) = tools_cache.lock() {
                            for tool in tools.iter_mut() {
                                if tool.config.id == tool_id {
                                    tool.status = status;
                                    break;
                                }
                            }
                        }

                        // Send completion progress
                        if let Some(sender) = &sender {
                            let _ = sender.send(StatusCheckProgress {
                                tool_id: tool_id.clone(),
                                tool_name: tool_id.clone(),
                                status: ProgressStatus::Completed,
                                message: "Status check completed".to_string(),
                                timestamp: Instant::now(),
                            });
                        }

                        if let Some(context) = &ctx {
                            context.request_repaint();
                        }
                    }
                    Err(e) => {
                        // Send failure progress
                        if let Some(sender) = &sender {
                            let _ = sender.send(StatusCheckProgress {
                                tool_id: tool_id.clone(),
                                tool_name: tool_id.clone(),
                                status: ProgressStatus::Failed,
                                message: format!("Failed: {}", e),
                                timestamp: Instant::now(),
                            });
                        }

                        tracing::error!("Status check failed for {}: {}", tool_id, e);
                    }
                }
            });

            if let Ok(mut tasks) = self.background_tasks.lock() {
                tasks.push(handle);
            }
        }
    }

    fn update_progress(&mut self) {
        if let Ok(mut receiver_guard) = self.progress_receiver.lock() {
            if let Some(receiver) = receiver_guard.as_mut() {
                while let Ok(progress) = receiver.try_recv() {
                    // Update tool name from cache
                    let tool_name = if let Ok(tools) = self.tools_cache.lock() {
                        tools.iter()
                            .find(|tool| tool.config.id == progress.tool_id)
                            .map(|tool| tool.config.name.clone())
                            .unwrap_or_else(|| progress.tool_id.clone())
                    } else {
                        progress.tool_id.clone()
                    };

                    let mut updated_progress = progress;
                    updated_progress.tool_name = tool_name;

                    // Update or add progress entry
                    if let Some(existing) = self.app_state.status_progress.iter_mut()
                        .find(|p| p.tool_id == updated_progress.tool_id) {
                        *existing = updated_progress;
                    } else {
                        self.app_state.status_progress.push(updated_progress);
                    }
                }
            }
        }
    }

    pub fn get_tool_help(&mut self, tool_id: String) {
        // Check cache first
        if let Ok(help_cache) = self.help_cache.lock() {
            if help_cache.contains_key(&tool_id) {
                return; // Already cached
            }
        }

        // Check persistent cache
        if let Ok(cache) = self.cache_manager.lock() {
            if let Some(cached_help) = cache.get_tool_help(&tool_id) {
                if let Ok(mut help_cache) = self.help_cache.lock() {
                    help_cache.insert(tool_id.clone(), cached_help);
                }
                return;
            }
        }

        let tool_manager = self.tool_manager.clone();
        let help_cache = Arc::clone(&self.help_cache);
        let cache_manager = Arc::clone(&self.cache_manager);
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();
        let tool_id_clone = tool_id.clone();

        let handle = runtime.spawn(async move {
            match tool_manager.get_tool_help(&tool_id_clone).await {
                Ok(help_content) => {
                    // Update memory cache
                    if let Ok(mut help_cache) = help_cache.lock() {
                        help_cache.insert(tool_id_clone.clone(), help_content.clone());
                    }

                    // Update persistent cache and save it
                    let cache_manager_clone = Arc::clone(&cache_manager);
                    let help_content_clone = help_content.clone();
                    let tool_id_for_cache = tool_id_clone.clone();
                    
                    // Update cache in a separate thread to avoid Send issues
                    std::thread::spawn(move || {
                        // Create a new runtime for this thread
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Ok(mut cache) = cache_manager_clone.lock() {
                                cache.set_tool_help(&tool_id_for_cache, help_content_clone);
                                if let Err(e) = cache.save().await {
                                    tracing::warn!("Failed to save cache: {}", e);
                                }
                            }
                        });
                    });

                    if let Some(context) = &ctx {
                        context.request_repaint();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get help for {}: {}", tool_id_clone, e);
                    // Note: Can't add notification here as we don't have access to self
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    fn get_cached_help(&self, tool_id: &str) -> Option<String> {
        if let Ok(help_cache) = self.help_cache.lock() {
            help_cache.get(tool_id).cloned()
        } else {
            None
        }
    }

    fn refresh_tools_with_progress(&mut self) {
        self.app_state.is_refreshing = true;
        self.app_state.status_progress.clear();

        // Initialize progress for all tools
        if let Ok(tools) = self.tools_cache.lock() {
            for tool in tools.iter() {
                self.app_state.status_progress.push(StatusCheckProgress {
                    tool_id: tool.config.id.clone(),
                    tool_name: tool.config.name.clone(),
                    status: ProgressStatus::Pending,
                    message: "Waiting to start...".to_string(),
                    timestamp: Instant::now(),
                });
            }
        }

        // Start background checking
        self.start_background_status_checking();

        // Set timer to reset refreshing state
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();
        runtime.spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            if let Some(context) = &ctx {
                context.request_repaint();
            }
        });
    }

    fn render_tool_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("🤖 AI CLI Tools");
        ui.separator();

        // Search and filter controls
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.app_state.search_query);
            ui.checkbox(&mut self.app_state.show_only_installed, "Show only installed");
        });

        ui.separator();

        let tools_data = if let Ok(tools) = self.tools_cache.lock() {
            tools.clone()
        } else {
            Vec::new()
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            if !tools_data.is_empty() {
                let filtered_tools: Vec<_> = tools_data.iter()
                    .filter(|tool| {
                        let matches_search = if self.app_state.search_query.is_empty() {
                            true
                        } else {
                            tool.config.name.to_lowercase().contains(&self.app_state.search_query.to_lowercase()) ||
                            tool.config.id.to_lowercase().contains(&self.app_state.search_query.to_lowercase())
                        };

                        let matches_filter = if self.app_state.show_only_installed {
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

            let is_selected = self.app_state.selected_tool.as_ref() == Some(&tool.config.id);
            if ui.selectable_label(is_selected, &tool.config.name).clicked() {
                self.app_state.selected_tool = Some(tool.config.id.clone());
                // Pre-load help when tool is selected
                if matches!(tool.status, ToolStatus::Installed { .. }) {
                    self.get_tool_help(tool.config.id.clone());
                }
            }
        });

        ui.small(&tool.config.description);
        ui.separator();
    }

    fn render_tool_details(&mut self, ui: &mut egui::Ui) {
        if let Some(selected_id) = &self.app_state.selected_tool.clone() {
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

                ui.separator();

                // Help section
                self.render_tool_help_section(ui, &tool);

                return;
            }
        }

        // Default view
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Welcome to CLIverge");
            ui.label("Select a tool from the list to see details and actions.");
            ui.add_space(20.0);

            if ui.button("🔄 Refresh Tool List").clicked() {
                self.refresh_tools_with_progress();
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
                        // TODO: Implement install functionality
                        self.add_notification("Installation started".to_string(), NotificationLevel::Info);
                    }
                }
                ToolStatus::Installed { version } => {
                    ui.label(format!("Version: {}", version));
                    
                    if ui.button("🗑 Uninstall").clicked() {
                        // TODO: Implement uninstall functionality
                        self.add_notification("Uninstallation started".to_string(), NotificationLevel::Info);
                    }
                    
                    if ui.button("🔄 Check Updates").clicked() {
                        // TODO: Implement update check
                        self.add_notification("Checking for updates...".to_string(), NotificationLevel::Info);
                    }
                }
                ToolStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", msg));
                }
            }
        });
    }

    fn render_tool_help_section(&mut self, ui: &mut egui::Ui, tool: &ToolInfo) {
        ui.collapsing("📝 Help & Documentation", |ui| {
            let tool_id = tool.config.id.clone();
            
            if let Some(help_content) = self.get_cached_help(&tool_id) {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.label("Command Line Help:");
                            ui.separator();
                            
                            let formatted_help = Self::format_help_text(&help_content);
                            ui.code(&formatted_help);
                        });
                    });
            } else {
                ui.horizontal(|ui| {
                    if matches!(tool.status, ToolStatus::Installed { .. }) {
                        if ui.button("📋 Get Help").clicked() {
                            self.get_tool_help(tool_id.clone());
                        }
                        ui.label("Click to load help information");
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "ℹ Help available after installation");
                    }
                });
            }
            
            ui.separator();
            
            // Configuration schema display
            if let Some(config_schema) = &tool.config.config_schema {
                if !config_schema.is_empty() {
                    ui.collapsing("⚙ Configuration Options", |ui| {
                        egui::Grid::new(format!("{}_config_grid", tool_id))
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Parameter");
                                ui.label("Type");
                                ui.label("Description");
                                ui.end_row();
                                
                                for (key, schema) in config_schema {
                                    ui.label(key);
                                    ui.label(&schema.field_type);
                                    ui.label(&schema.description);
                                    ui.end_row();
                                }
                            });
                    });
                }
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙ Settings");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Appearance Settings
            ui.collapsing("🎨 Appearance", |ui| {
                let mut settings_changed = false;
                
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    let old_theme = self.app_state.settings_theme.clone();
                    egui::ComboBox::from_label("")
                        .selected_text(&self.app_state.settings_theme)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.app_state.settings_theme, "dark".to_string(), "Dark");
                            ui.selectable_value(&mut self.app_state.settings_theme, "light".to_string(), "Light");
                            ui.selectable_value(&mut self.app_state.settings_theme, "auto".to_string(), "Auto");
                        });
                    if old_theme != self.app_state.settings_theme {
                        settings_changed = true;
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    if ui.add(egui::Slider::new(&mut self.app_state.settings_font_size, 10.0..=24.0).suffix("px")).changed() {
                        settings_changed = true;
                    }
                });
                
                // Auto-save when settings change
                if settings_changed {
                    self.save_settings_from_state();
                    self.add_notification("Settings saved automatically".to_string(), NotificationLevel::Info);
                }
            });
            
            ui.separator();
            
            // Behavior Settings
            ui.collapsing("⚙ Behavior", |ui| {
                let mut settings_changed = false;
                
                let old_auto_check = self.app_state.settings_auto_check_updates;
                if ui.checkbox(&mut self.app_state.settings_auto_check_updates, "Auto check for updates").changed() {
                    settings_changed = true;
                }
                
                if self.app_state.settings_auto_check_updates {
                    ui.horizontal(|ui| {
                        ui.label("Check interval:");
                        if ui.add(egui::Slider::new(&mut self.app_state.settings_check_interval, 5..=120).suffix(" minutes")).changed() {
                            settings_changed = true;
                        }
                    });
                }
                
                if ui.checkbox(&mut self.app_state.settings_show_notifications, "Show notifications").changed() {
                    settings_changed = true;
                }
                if ui.checkbox(&mut self.app_state.settings_auto_refresh_on_startup, "Auto refresh tool status on startup").changed() {
                    settings_changed = true;
                }
                
                // Auto-save when settings change
                if settings_changed {
                    self.save_settings_from_state();
                    self.add_notification("Settings saved automatically".to_string(), NotificationLevel::Info);
                    
                    // If auto_check_updates was enabled, start the timer
                    if !old_auto_check && self.app_state.settings_auto_check_updates {
                        self.start_auto_check_timer();
                    }
                }
            });
            
            ui.separator();
            
            // Get actual config directory path - same as ConfigManager
            let config_dir = Self::get_config_dir();
            
            // Configuration Files
            ui.collapsing("📁 Configuration Files", |ui| {
                let settings_path = config_dir.join("settings.json");
                let tools_path = config_dir.join("tools.json");
                
                ui.horizontal(|ui| {
                    ui.label("Settings file:");
                    ui.code(settings_path.to_str().unwrap_or("settings.json"));
                    if ui.button("📝 Open").clicked() {
                        self.open_settings_file();
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Tools config:");
                    ui.code(tools_path.to_str().unwrap_or("tools.json"));
                    if ui.button("📝 Open").clicked() {
                        self.open_tools_file();
                    }
                });
                
                ui.separator();
                
                if ui.button("📂 Open Config Directory").clicked() {
                    self.open_config_directory();
                }
            });
            
            ui.separator();
            
            // Cache Settings
            ui.collapsing("📋 Cache", |ui| {
                let cache_path = config_dir.join("cache");
                ui.horizontal(|ui| {
                    ui.label("Cache directory:");
                    ui.code(cache_path.to_str().unwrap_or("cache"));
                    if ui.button("📁 Open").clicked() {
                        self.open_cache_directory();
                    }
                });
                
                let (status_count, help_count) = if let Ok(cache) = self.cache_manager.lock() {
                    cache.get_cache_stats()
                } else {
                    (0, 0)
                };
                
                ui.horizontal(|ui| {
                    ui.label(format!("Cached items: {} status, {} help docs", 
                        status_count, help_count));
                });
                
                ui.horizontal(|ui| {
                    if ui.button("🧹 Clear All Cache").clicked() {
                        // Clear cache and save, capturing the result
                        let save_result = if let Ok(mut cache) = self.cache_manager.lock() {
                            cache.clear_all();
                            self.runtime.block_on(cache.save())
                        } else {
                            Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to lock cache").into())
                        };
                        
                        // Clear help cache
                        if let Ok(mut help_cache) = self.help_cache.lock() {
                            help_cache.clear();
                        }
                        
                        // Add notification based on result
                        match save_result {
                            Ok(_) => self.add_notification("Cache cleared successfully".to_string(), NotificationLevel::Success),
                            Err(e) => self.add_notification(format!("Failed to clear cache: {}", e), NotificationLevel::Error),
                        }
                    }
                });
            });
            
            ui.separator();
            
            // Advanced Settings
            ui.collapsing("⚙ Advanced", |ui| {
                ui.checkbox(&mut self.app_state.settings_debug_mode, "Debug mode (verbose logging)");
                ui.checkbox(&mut self.app_state.settings_experimental_features, "Enable experimental features");
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("✅ Save Settings").clicked() {
                self.save_settings_from_state();
                self.add_notification("Settings saved successfully".to_string(), NotificationLevel::Success);
            }
            
            if ui.button("🔄 Reset to Defaults").clicked() {
                self.app_state.settings_theme = "dark".to_string();
                self.app_state.settings_font_size = 14.0;
                self.app_state.settings_auto_check_updates = false;
                self.app_state.settings_check_interval = 30;
                self.app_state.settings_show_notifications = true;
                self.app_state.settings_auto_refresh_on_startup = true;
                self.app_state.settings_debug_mode = false;
                self.app_state.settings_experimental_features = false;
                self.add_notification("Settings reset to defaults".to_string(), NotificationLevel::Warning);
            }
            
            if ui.button("◀ Back").clicked() {
                self.app_state.current_view = AppView::Main;
            }
        });
    }

    fn render_progress_log(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Tool Status Checks");
        ui.separator();

        if self.app_state.status_progress.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No status checks in progress. Click 'Refresh' to start checking tool statuses.");
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for progress in &self.app_state.status_progress {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let (icon, color) = match &progress.status {
                            ProgressStatus::Pending => ("⏳", egui::Color32::GRAY),
                            ProgressStatus::InProgress => ("🔄", egui::Color32::BLUE),
                            ProgressStatus::Completed => ("✅", egui::Color32::GREEN),
                            ProgressStatus::Failed => ("❌", egui::Color32::RED),
                        };
                        
                        ui.colored_label(color, icon);
                        ui.label(&progress.tool_name);
                        
                        // Show timestamp
                        let elapsed = progress.timestamp.elapsed().as_secs();
                        ui.small(format!("{}s ago", elapsed));
                    });
                    ui.small(&progress.message);
                });
                ui.add_space(4.0);
            }
        });

        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("🧹 Clear Log").clicked() {
                self.app_state.status_progress.clear();
            }
            
            if ui.button("🔄 Refresh All").clicked() {
                self.refresh_tools_with_progress();
            }
        });
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
                ui.label("• Real-time status monitoring with caching");
                ui.label("• Version checking and updates");
                ui.label("• Help documentation display");
                ui.label("• Clean and intuitive user interface");
            });

            ui.add_space(20.0);

            if ui.button("OK").clicked() {
                self.app_state.current_view = AppView::Main;
            }
        });
    }

    fn add_notification(&mut self, message: String, level: NotificationLevel) {
        self.app_state.notifications.push(Notification {
            message,
            level,
            timestamp: Instant::now(),
        });

        // Keep only last 10 notifications
        if self.app_state.notifications.len() > 10 {
            self.app_state.notifications.remove(0);
        }
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

    fn format_help_text(help_content: &str) -> String {
        help_content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    String::new()
                } else if trimmed.starts_with('-') && trimmed.len() > 2 {
                    format!("  {}", trimmed)
                } else {
                    trimmed.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn apply_theme_settings(&self, ctx: &egui::Context) {
        let visuals = match self.app_state.settings_theme.as_str() {
            "light" => egui::Visuals::light(),
            "dark" => egui::Visuals::dark(),
            "auto" => {
                // Use system theme detection or default to dark
                egui::Visuals::dark()
            }
            _ => egui::Visuals::dark(),
        };
        
        ctx.set_visuals(visuals);
        
        // Apply font size settings - just update style without custom fonts
        let base_size = self.app_state.settings_font_size;
        
        // Update font sizes with custom scaling
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(base_size * 1.6, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(base_size, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(base_size, egui::FontFamily::Monospace)),
            (egui::TextStyle::Button, egui::FontId::new(base_size, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(base_size * 0.83, egui::FontFamily::Proportional)),
        ].into();
        
        ctx.set_style(style);
    }
    
    fn open_cache_directory(&self) {
        let cache_path = Self::get_config_dir().join("cache");
        
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(cache_path.to_str().unwrap_or("."))
                .spawn();
        }
        
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(&cache_path)
                .spawn();
        }
        
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&cache_path)
                .spawn();
        }
    }
    
    fn open_settings_file(&self) {
        let settings_path = Self::get_config_dir().join("settings.json");
        
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("notepad")
                .arg(settings_path.to_str().unwrap_or("."))
                .spawn();
        }
        
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-t")
                .arg(&settings_path)
                .spawn();
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try common text editors
            let editors = ["gedit", "kate", "nano", "vi"];
            for editor in &editors {
                if std::process::Command::new("which")
                    .arg(editor)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    let _ = std::process::Command::new(editor)
                        .arg(&settings_path)
                        .spawn();
                    break;
                }
            }
        }
    }
    
    fn open_tools_file(&self) {
        let tools_path = Self::get_config_dir().join("tools.json");
        
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("notepad")
                .arg(tools_path.to_str().unwrap_or("."))
                .spawn();
        }
        
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-t")
                .arg(&tools_path)
                .spawn();
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try common text editors
            let editors = ["gedit", "kate", "nano", "vi"];
            for editor in &editors {
                if std::process::Command::new("which")
                    .arg(editor)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    let _ = std::process::Command::new(editor)
                        .arg(&tools_path)
                        .spawn();
                    break;
                }
            }
        }
    }
    
    fn open_config_directory(&self) {
        let config_dir = Self::get_config_dir();
        
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(config_dir.to_str().unwrap_or("."))
                .spawn();
        }
        
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(&config_dir)
                .spawn();
        }
        
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&config_dir)
                .spawn();
        }
    }
    
    fn start_auto_check_timer(&mut self) {
        // Reset the timer
        self.app_state.last_auto_check = Some(Instant::now());
        
        if self.app_state.settings_show_notifications {
            self.add_notification(
                format!("Auto check enabled - will check every {} minutes", 
                       self.app_state.settings_check_interval),
                NotificationLevel::Info
            );
        }
    }
    
    fn check_auto_update_timer(&mut self) {
        if !self.app_state.settings_auto_check_updates {
            return;
        }
        
        let should_check = if let Some(last_check) = self.app_state.last_auto_check {
            let elapsed = last_check.elapsed();
            let interval = Duration::from_secs((self.app_state.settings_check_interval as u64) * 60);
            elapsed >= interval
        } else {
            // First time check
            true
        };
        
        if should_check && !self.app_state.is_refreshing {
            self.app_state.last_auto_check = Some(Instant::now());
            
            if self.app_state.settings_show_notifications {
                self.add_notification(
                    "Auto checking tool updates...".to_string(),
                    NotificationLevel::Info
                );
            }
            
            // Start background status checking
            self.start_background_status_checking();
        }
    }
}

impl eframe::App for CLIvergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme settings
        self.apply_theme_settings(ctx);
        
        // Store context for background tasks
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
            if self.app_state.settings_auto_refresh_on_startup {
                self.start_background_status_checking();
            }
        }

        // Update progress from background tasks
        self.update_progress();
        
        // Check auto update timer
        self.check_auto_update_timer();

        // Reset refreshing state if all progress items are completed or failed
        if self.app_state.is_refreshing {
            let all_done = self.app_state.status_progress.iter()
                .all(|p| matches!(p.status, ProgressStatus::Completed | ProgressStatus::Failed));
            if all_done && !self.app_state.status_progress.is_empty() {
                self.app_state.is_refreshing = false;
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 CLIverge - AI CLI Tool Manager");
                
                ui.separator();
                
                let refresh_button_text = if self.app_state.is_refreshing {
                    "⏳ Refreshing..."
                } else {
                    "🔄 Refresh"
                };
                
                if ui.add_enabled(!self.app_state.is_refreshing, egui::Button::new(refresh_button_text)).clicked() {
                    self.refresh_tools_with_progress();
                }
                
                if ui.button("📊 Status Log").clicked() {
                    self.app_state.log_window_open = !self.app_state.log_window_open;
                }
                
                if ui.button("⚙ Settings").clicked() {
                    self.app_state.current_view = AppView::Settings;
                }
                
                if ui.button("ℹ About").clicked() {
                    self.app_state.current_view = AppView::About;
                }
            });
        });
        
        // Progress log window
        let mut log_window_open = self.app_state.log_window_open;
        if log_window_open {
            egui::Window::new("📊 Status Check Progress")
                .open(&mut log_window_open)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    self.render_progress_log(ui);
                });
            self.app_state.log_window_open = log_window_open;
        }

        // Show notifications
        for (i, notification) in self.app_state.notifications.iter().enumerate() {
            let age = notification.timestamp.elapsed().as_secs_f32();
            if age < 5.0 { // Show for 5 seconds
                let alpha = (1.0 - age / 5.0).max(0.0);
                let color = match notification.level {
                    NotificationLevel::Info => egui::Color32::from_rgba_unmultiplied(70, 130, 180, (255.0 * alpha) as u8),
                    NotificationLevel::Success => egui::Color32::from_rgba_unmultiplied(34, 139, 34, (255.0 * alpha) as u8),
                    NotificationLevel::Warning => egui::Color32::from_rgba_unmultiplied(255, 165, 0, (255.0 * alpha) as u8),
                    NotificationLevel::Error => egui::Color32::from_rgba_unmultiplied(220, 20, 60, (255.0 * alpha) as u8),
                };

                egui::Window::new(format!("notification_{}", i))
                    .title_bar(false)
                    .resizable(false)
                    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0 + (i as f32) * 60.0])
                    .frame(egui::Frame::popup(&ctx.style()).fill(color))
                    .show(ctx, |ui| {
                        ui.label(&notification.message);
                    });
            }
        }

        // Remove old notifications
        self.app_state.notifications.retain(|n| n.timestamp.elapsed().as_secs_f32() < 5.0);

        // Main content
        match self.app_state.current_view {
            AppView::Main => {
                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .default_width(300.0)
                    .show(ctx, |ui| {
                        self.render_tool_list(ui);
                    });

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
        // Save cache before dropping
        if let Ok(cache) = self.cache_manager.lock() {
            let _ = self.runtime.block_on(cache.save());
        }

        // Clean up background tasks
        if let Ok(mut tasks) = self.background_tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}
