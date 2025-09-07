use cliverge_core::{
    AppSettings, AppearanceSettings, BehaviorSettings, CacheManager, ConfigManager, ToolInfo,
    ToolManager, ToolStatus,
};
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

// 类型别名以减少复杂度警告
type BackgroundTasks = Arc<Mutex<Vec<JoinHandle<()>>>>;
type ToolsCache = Arc<Mutex<Vec<ToolInfo>>>;
type HelpCache = Arc<Mutex<HashMap<String, String>>>;
type ProgressSender = Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<StatusCheckProgress>>>>;
type ProgressReceiver =
    Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<StatusCheckProgress>>>>;
type InstallSender = Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<InstallProgress>>>>;
type InstallReceiver = Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<InstallProgress>>>>;
type LogEntry = (Instant, String);
type UpdateConfigMethods = std::collections::HashMap<String, Vec<String>>;
type LoadResult = Result<cliverge_core::ToolsConfig, Box<dyn std::error::Error>>;

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

#[derive(Debug, Clone)]
pub struct InstallProgress {
    pub tool_id: String,
    pub tool_name: String,
    pub operation: InstallOperation,
    pub status: ProgressStatus,
    pub message: String,
    pub command: Option<String>, // 用于显示执行的命令
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum InstallOperation {
    Install,
    Uninstall,
    #[allow(dead_code)] // 为未来的更新功能预留
    Update,
}

pub struct AppState {
    // UI State
    pub selected_tool: Option<String>,
    pub search_query: String,
    pub show_only_installed: bool,
    pub bottom_log_panel_open: bool, // 修改此行：从 log_window_open 改名
    pub notifications: Vec<Notification>,
    pub current_view: AppView,
    pub status_progress: Vec<StatusCheckProgress>,
    pub install_progress: Vec<InstallProgress>, // 新增此行
    pub is_refreshing: bool,

    // Tool configuration editor state
    pub show_tool_editor: bool,
    pub editing_tool_id: Option<String>, // None for new tool, Some(id) for editing
    pub tool_form_state: ToolFormState,

    // Delete confirmation dialog state
    pub show_delete_confirmation: bool,
    pub tool_to_delete: Option<String>,

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

// Tool form state for adding/editing tools
#[derive(Debug, Clone)]
pub struct ToolFormState {
    // Basic info
    pub id: String,
    pub name: String,
    pub description: String,
    pub website: String,
    pub command: String,

    // Version check methods (per platform)
    pub version_check_methods: std::collections::HashMap<String, String>, // 平台 -> 版本检查参数
    // Update check methods (per platform)
    pub update_check_methods: std::collections::HashMap<String, String>, // 平台 -> 更新检查参数

    // Install methods (per platform)
    pub install_methods: std::collections::HashMap<String, InstallMethodForm>,
    // Uninstall methods (per platform)
    pub uninstall_methods: std::collections::HashMap<String, InstallMethodForm>,
    // Update methods (per platform)
    pub update_methods: std::collections::HashMap<String, InstallMethodForm>,

    // Validation errors
    pub errors: Vec<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct InstallMethodForm {
    pub method: String,       // npm, brew, pip, script, etc.
    pub command_args: String, // Space-separated command args
    pub url: String,          // For script installs
    pub package_name: String, // Package name for package managers
}

impl Default for ToolFormState {
    fn default() -> Self {
        let mut version_check_methods = std::collections::HashMap::new();
        let mut update_check_methods = std::collections::HashMap::new();
        let mut install_methods = std::collections::HashMap::new();
        let mut uninstall_methods = std::collections::HashMap::new();
        let mut update_methods = std::collections::HashMap::new();

        // Initialize with empty forms for all platforms
        for platform in ["windows", "macos", "linux"] {
            version_check_methods.insert(platform.to_string(), "--version".to_string());
            update_check_methods.insert(platform.to_string(), String::new());
            install_methods.insert(platform.to_string(), InstallMethodForm::default());
            uninstall_methods.insert(platform.to_string(), InstallMethodForm::default());
            update_methods.insert(platform.to_string(), InstallMethodForm::default());
        }

        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            website: String::new(),
            command: String::new(),
            version_check_methods,
            update_check_methods,
            install_methods,
            uninstall_methods,
            update_methods,
            errors: Vec::new(),
            is_valid: false,
        }
    }
}

impl Default for InstallMethodForm {
    fn default() -> Self {
        Self {
            method: "npm".to_string(),
            command_args: String::new(),
            url: String::new(),
            package_name: String::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_tool: None,
            search_query: String::new(),
            show_only_installed: false,
            bottom_log_panel_open: true, // 修改此行，默认打开
            notifications: Vec::new(),
            current_view: AppView::Main,
            status_progress: Vec::new(),
            install_progress: Vec::new(), // 新增此行
            is_refreshing: false,

            // Tool editor state
            show_tool_editor: false,
            editing_tool_id: None,
            tool_form_state: ToolFormState::default(),

            // Delete confirmation state
            show_delete_confirmation: false,
            tool_to_delete: None,

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
    background_tasks: BackgroundTasks,
    tools_cache: ToolsCache,
    help_cache: HelpCache,
    progress_sender: ProgressSender,
    progress_receiver: ProgressReceiver,
    install_sender: InstallSender,
    install_receiver: InstallReceiver,
    ctx: Option<egui::Context>,
}

impl CLIvergeApp {
    pub fn new() -> Self {
        let runtime =
            Arc::new(tokio::runtime::Runtime::new().expect("Failed to create async runtime"));

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
        let (install_sender, install_receiver) = tokio::sync::mpsc::unbounded_channel(); // 新增此行

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
            install_sender: Arc::new(Mutex::new(Some(install_sender))), // 新增此行
            install_receiver: Arc::new(Mutex::new(Some(install_receiver))), // 新增此行
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

    fn load_embedded_tools_config() -> LoadResult {
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
            self.app_state.settings_auto_refresh_on_startup =
                settings.behavior.auto_refresh_on_startup;
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
            settings.behavior.auto_refresh_on_startup =
                self.app_state.settings_auto_refresh_on_startup;

            config.update_app_settings(settings);

            // Save asynchronously - use std::thread to avoid Send issues
            let config_manager = Arc::clone(&self.config_manager);
            std::thread::spawn(move || {
                // Create a new runtime for this thread
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    // This is safe because we're in a blocking context, not async
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
        let tool_infos = self
            .tool_manager
            .get_all_tools_configs()
            .unwrap_or_default();

        // Load from cache first
        if let (Ok(mut tools), Ok(cache)) = (self.tools_cache.lock(), self.cache_manager.lock()) {
            *tools = tool_infos
                .into_iter()
                .map(|mut tool_info| {
                    let cached_status = cache
                        .get_tool_status(&tool_info.config.id)
                        .unwrap_or(ToolStatus::Unknown);

                    tool_info.status = cached_status;
                    tool_info
                })
                .collect();
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
                                // This is safe because we're in a blocking context, not async
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
                                message: format!("Failed: {e}"),
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
        // 更新状态检查进度 (原有逻辑)
        if let Ok(mut receiver_guard) = self.progress_receiver.lock() {
            if let Some(receiver) = receiver_guard.as_mut() {
                while let Ok(progress) = receiver.try_recv() {
                    // Update tool name from cache
                    let tool_name = if let Ok(tools) = self.tools_cache.lock() {
                        tools
                            .iter()
                            .find(|tool| tool.config.id == progress.tool_id)
                            .map(|tool| tool.config.name.clone())
                            .unwrap_or_else(|| progress.tool_id.clone())
                    } else {
                        progress.tool_id.clone()
                    };

                    let mut updated_progress = progress;
                    updated_progress.tool_name = tool_name;

                    // Update or add progress entry
                    if let Some(existing) = self
                        .app_state
                        .status_progress
                        .iter_mut()
                        .find(|p| p.tool_id == updated_progress.tool_id)
                    {
                        *existing = updated_progress;
                    } else {
                        self.app_state.status_progress.push(updated_progress);
                    }
                }
            }
        }

        // 新增：更新安装进度
        if let Ok(mut receiver_guard) = self.install_receiver.lock() {
            if let Some(receiver) = receiver_guard.as_mut() {
                while let Ok(progress) = receiver.try_recv() {
                    // 更新工具名称从缓存
                    let tool_name = if let Ok(tools) = self.tools_cache.lock() {
                        tools
                            .iter()
                            .find(|tool| tool.config.id == progress.tool_id)
                            .map(|tool| tool.config.name.clone())
                            .unwrap_or_else(|| progress.tool_id.clone())
                    } else {
                        progress.tool_id.clone()
                    };

                    let mut updated_progress = progress;
                    updated_progress.tool_name = tool_name;

                    // 更新或添加安装进度条目
                    if let Some(existing) = self.app_state.install_progress.iter_mut().find(|p| {
                        p.tool_id == updated_progress.tool_id
                            && std::mem::discriminant(&p.operation)
                                == std::mem::discriminant(&updated_progress.operation)
                    }) {
                        *existing = updated_progress;
                    } else {
                        self.app_state.install_progress.push(updated_progress);
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
                            // This is safe because we're in a blocking context, not async
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
        ui.heading("CLI Tools");
        ui.separator();

        // Search controls
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.app_state.search_query).desired_width(180.0),
            );
        });

        // Filter and Add Tool controls on same line
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.app_state.show_only_installed,
                "Show only installed tools",
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Add Tool").clicked() {
                    self.open_tool_editor_for_new_tool();
                }
            });
        });

        ui.separator();

        let tools_data = if let Ok(tools) = self.tools_cache.lock() {
            tools.clone()
        } else {
            Vec::new()
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            if !tools_data.is_empty() {
                let filtered_tools: Vec<_> = tools_data
                    .iter()
                    .filter(|tool| {
                        let matches_search = if self.app_state.search_query.is_empty() {
                            true
                        } else {
                            tool.config
                                .name
                                .to_lowercase()
                                .contains(&self.app_state.search_query.to_lowercase())
                                || tool
                                    .config
                                    .id
                                    .to_lowercase()
                                    .contains(&self.app_state.search_query.to_lowercase())
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
            if ui
                .selectable_label(is_selected, &tool.config.name)
                .clicked()
            {
                self.app_state.selected_tool = Some(tool.config.id.clone());
                // Pre-load help when tool is selected
                if matches!(tool.status, ToolStatus::Installed { .. }) {
                    self.get_tool_help(tool.config.id.clone());
                }
            }

            // Edit button on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✏").clicked() {
                    self.open_tool_editor_for_existing_tool(&tool.config.id);
                }
            });
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
                        self.install_tool(tool.config.id.clone());
                        self.add_notification(
                            "Installation started".to_string(),
                            NotificationLevel::Info,
                        );
                    }
                }
                ToolStatus::Installed { version } => {
                    ui.label(format!("Version: {version}"));

                    if ui.button("🗑 Uninstall").clicked() {
                        self.uninstall_tool(tool.config.id.clone());
                        self.add_notification(
                            "Uninstallation started".to_string(),
                            NotificationLevel::Info,
                        );
                    }

                    if ui.button("🔄 Check Updates").clicked() {
                        // TODO: Implement update check
                        self.add_notification(
                            "Checking for updates...".to_string(),
                            NotificationLevel::Info,
                        );
                    }
                }
                ToolStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("Error: {msg}"));
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
                        ui.colored_label(
                            egui::Color32::GRAY,
                            "ℹ Help available after installation",
                        );
                    }
                });
            }

            ui.separator();

            // Configuration schema display
            if let Some(config_schema) = &tool.config.config_schema {
                if !config_schema.is_empty() {
                    ui.collapsing("⚙ Configuration Options", |ui| {
                        egui::Grid::new(format!("{tool_id}_config_grid"))
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
                            ui.selectable_value(
                                &mut self.app_state.settings_theme,
                                "dark".to_string(),
                                "Dark",
                            );
                            ui.selectable_value(
                                &mut self.app_state.settings_theme,
                                "light".to_string(),
                                "Light",
                            );
                            ui.selectable_value(
                                &mut self.app_state.settings_theme,
                                "auto".to_string(),
                                "Auto",
                            );
                        });
                    if old_theme != self.app_state.settings_theme {
                        settings_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    if ui
                        .add(
                            egui::Slider::new(&mut self.app_state.settings_font_size, 10.0..=24.0)
                                .suffix("px"),
                        )
                        .changed()
                    {
                        settings_changed = true;
                    }
                });

                // Auto-save when settings change
                if settings_changed {
                    self.save_settings_from_state();
                    self.add_notification(
                        "Settings saved automatically".to_string(),
                        NotificationLevel::Info,
                    );
                }
            });

            ui.separator();

            // Behavior Settings
            ui.collapsing("⚙ Behavior", |ui| {
                let mut settings_changed = false;

                let old_auto_check = self.app_state.settings_auto_check_updates;
                if ui
                    .checkbox(
                        &mut self.app_state.settings_auto_check_updates,
                        "Auto check for updates",
                    )
                    .changed()
                {
                    settings_changed = true;
                }

                if self.app_state.settings_auto_check_updates {
                    ui.horizontal(|ui| {
                        ui.label("Check interval:");
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut self.app_state.settings_check_interval,
                                    5..=120,
                                )
                                .suffix(" minutes"),
                            )
                            .changed()
                        {
                            settings_changed = true;
                        }
                    });
                }

                if ui
                    .checkbox(
                        &mut self.app_state.settings_show_notifications,
                        "Show notifications",
                    )
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(
                        &mut self.app_state.settings_auto_refresh_on_startup,
                        "Auto refresh tool status on startup",
                    )
                    .changed()
                {
                    settings_changed = true;
                }

                // Auto-save when settings change
                if settings_changed {
                    self.save_settings_from_state();
                    self.add_notification(
                        "Settings saved automatically".to_string(),
                        NotificationLevel::Info,
                    );

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
                    ui.label(format!(
                        "Cached items: {status_count} status, {help_count} help docs"
                    ));
                });

                ui.horizontal(|ui| {
                    if ui.button("🧹 Clear All Cache").clicked() {
                        // Clear cache and save, capturing the result
                        let save_result = if let Ok(mut cache) = self.cache_manager.lock() {
                            cache.clear_all();
                            self.runtime.block_on(cache.save())
                        } else {
                            Err(std::io::Error::other("Failed to lock cache").into())
                        };

                        // Clear help cache
                        if let Ok(mut help_cache) = self.help_cache.lock() {
                            help_cache.clear();
                        }

                        // Add notification based on result
                        match save_result {
                            Ok(_) => self.add_notification(
                                "Cache cleared successfully".to_string(),
                                NotificationLevel::Success,
                            ),
                            Err(e) => self.add_notification(
                                format!("Failed to clear cache: {e}"),
                                NotificationLevel::Error,
                            ),
                        }
                    }
                });
            });

            ui.separator();

            // Advanced Settings
            ui.collapsing("⚙ Advanced", |ui| {
                ui.checkbox(
                    &mut self.app_state.settings_debug_mode,
                    "Debug mode (verbose logging)",
                );
                ui.checkbox(
                    &mut self.app_state.settings_experimental_features,
                    "Enable experimental features",
                );
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("✅ Save Settings").clicked() {
                self.save_settings_from_state();
                self.add_notification(
                    "Settings saved successfully".to_string(),
                    NotificationLevel::Success,
                );
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
                self.add_notification(
                    "Settings reset to defaults".to_string(),
                    NotificationLevel::Warning,
                );
            }

            if ui.button("◀ Back").clicked() {
                self.app_state.current_view = AppView::Main;
            }
        });
    }

    fn render_comprehensive_log(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("📊 Operations Log");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🧹 Clear All").clicked() {
                    self.app_state.status_progress.clear();
                    self.app_state.install_progress.clear();
                }

                if ui.button("📋 Copy All").clicked() {
                    let all_logs = self.get_all_logs_as_text();
                    ui.output_mut(|o| o.copied_text = all_logs);
                }

                if ui.button("🔄 Refresh Tools").clicked() {
                    self.refresh_tools_with_progress();
                }

                let toggle_text = if self.app_state.bottom_log_panel_open {
                    "⬇ Hide Log"
                } else {
                    "⬆ Show Log"
                };
                if ui.button(toggle_text).clicked() {
                    self.app_state.bottom_log_panel_open = !self.app_state.bottom_log_panel_open;
                }
            });
        });

        if !self.app_state.bottom_log_panel_open {
            return;
        }

        ui.separator();

        // 创建合并的日志条目
        let mut combined_entries: Vec<LogEntry> = Vec::new();

        // 添加状态检查条目
        for progress in &self.app_state.status_progress {
            let (icon, _color_name) = match &progress.status {
                ProgressStatus::Pending => ("⏳", "GRAY"),
                ProgressStatus::InProgress => ("🔄", "BLUE"),
                ProgressStatus::Completed => ("✅", "GREEN"),
                ProgressStatus::Failed => ("❌", "RED"),
            };

            let entry = format!(
                "[STATUS] {} {} - {}",
                icon, progress.tool_name, progress.message
            );
            combined_entries.push((progress.timestamp, entry));
        }

        // 添加安装进度条目
        for progress in &self.app_state.install_progress {
            let (icon, _color_name) = match &progress.status {
                ProgressStatus::Pending => ("⏳", "GRAY"),
                ProgressStatus::InProgress => ("🔄", "BLUE"),
                ProgressStatus::Completed => ("✅", "GREEN"),
                ProgressStatus::Failed => ("❌", "RED"),
            };

            let operation = match &progress.operation {
                InstallOperation::Install => "INSTALL",
                InstallOperation::Uninstall => "UNINSTALL",
                InstallOperation::Update => "UPDATE",
            };

            let mut entry = format!(
                "[{}] {} {} - {}",
                operation, icon, progress.tool_name, progress.message
            );

            // 添加命令信息（如果可用）
            if let Some(command) = &progress.command {
                entry.push_str(&format!("\n   Command: {command}"));
            }

            combined_entries.push((progress.timestamp, entry));
        }

        // 按时间戳排序（最新的在最后）
        combined_entries.sort_by_key(|&(timestamp, _)| timestamp);

        if combined_entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No operations logged yet. Install, uninstall, or refresh tools to see activity here.");
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (timestamp, entry) in combined_entries {
                    // 格式化时间戳为事件发生的实际时间 (HH:MM:SS)
                    let time_str = {
                        use chrono::{DateTime, Local};

                        // 计算事件发生的实际时间
                        let now = std::time::SystemTime::now();
                        let event_time = now - timestamp.elapsed();

                        // 转换为本地日期时间
                        let datetime: DateTime<Local> = event_time.into();
                        datetime.format("%H:%M:%S").to_string()
                    };

                    // 处理多行消息（例如，详细的错误消息）
                    let lines: Vec<&str> = entry.lines().collect();
                    if lines.len() > 1 {
                        // 多行消息 - 显示时带缩进
                        let first_line = format!("[{}] {}", time_str, lines[0]);
                        let mut first_line_text = first_line.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut first_line_text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1)
                                .interactive(true)
                                .frame(false),
                        );

                        // 显示后续行时带缩进
                        for line in &lines[1..] {
                            if !line.trim().is_empty() {
                                let indented_line = format!("           {}", line.trim());
                                let mut indented_text = indented_line.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut indented_text)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(1)
                                        .interactive(true)
                                        .frame(false),
                                );
                            }
                        }
                    } else {
                        // 单行消息 - 使整行可选择
                        let full_text = format!("[{time_str}] {entry}");
                        let mut text_copy = full_text.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut text_copy)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1)
                                .interactive(true)
                                .frame(false),
                        );
                    }
                }
            });
    }

    pub fn install_tool(&mut self, tool_id: String) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);
        let cache_manager = Arc::clone(&self.cache_manager);
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();
        let sender = if let Ok(sender_guard) = self.install_sender.lock() {
            sender_guard.clone()
        } else {
            None
        };

        // 获取安装命令用于显示目的
        let install_command = if let Ok(tools) = self.tools_cache.lock() {
            if let Some(tool_info) = tools.iter().find(|t| t.config.id == tool_id) {
                let platform = std::env::consts::OS;
                tool_info
                    .config
                    .install
                    .get(platform)
                    .and_then(|install_method| install_method.command.as_ref())
                    .map(|cmd| cmd.join(" "))
                    .or_else(|| {
                        // 回退：从方法和包名构造命令
                        tool_info
                            .config
                            .install
                            .get(platform)
                            .map(|install_method| {
                                format!(
                                    "{} install -g {}",
                                    install_method.method,
                                    install_method.package_name.as_ref().unwrap_or(&tool_id)
                                )
                            })
                    })
            } else {
                None
            }
        } else {
            None
        };

        let handle = runtime.spawn(async move {
            // 发送进度更新 - 开始
            if let Some(sender) = &sender {
                let _ = sender.send(InstallProgress {
                    tool_id: tool_id.clone(),
                    tool_name: tool_id.clone(),
                    operation: InstallOperation::Install,
                    status: ProgressStatus::InProgress,
                    message: "Starting installation...".to_string(),
                    command: install_command.clone(),
                    timestamp: Instant::now(),
                });
            }

            match tool_manager.install_tool(&tool_id).await {
                Ok(_) => {
                    // 安装后重新检查状态
                    let new_status = tool_manager.check_tool_status(&tool_id).await;

                    // 更新工具缓存状态
                    if let Ok(status) = &new_status {
                        if let Ok(mut tools) = tools_cache.lock() {
                            for tool in tools.iter_mut() {
                                if tool.config.id == tool_id {
                                    tool.status = status.clone();
                                    break;
                                }
                            }
                        }
                    }

                    // 使用我们已经检查的状态更新缓存
                    if let Ok(status) = &new_status {
                        let cache_manager_clone = Arc::clone(&cache_manager);
                        let status_clone = status.clone();
                        let tool_id_for_cache = tool_id.clone();

                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                if let Ok(mut cache) = cache_manager_clone.lock() {
                                    cache.set_tool_status(&tool_id_for_cache, status_clone);
                                    let _ = cache.save().await;
                                }
                            });
                        });
                    }

                    // 发送完成进度
                    if let Some(sender) = &sender {
                        let _ = sender.send(InstallProgress {
                            tool_id: tool_id.clone(),
                            tool_name: tool_id.clone(),
                            operation: InstallOperation::Install,
                            status: ProgressStatus::Completed,
                            message: "Installation completed successfully".to_string(),
                            command: install_command.clone(),
                            timestamp: Instant::now(),
                        });
                    }

                    if let Some(context) = &ctx {
                        context.request_repaint();
                    }
                }
                Err(e) => {
                    // 发送失败进度，包含详细错误信息
                    if let Some(sender) = &sender {
                        let _ = sender.send(InstallProgress {
                            tool_id: tool_id.clone(),
                            tool_name: tool_id.clone(),
                            operation: InstallOperation::Install,
                            status: ProgressStatus::Failed,
                            message: format!("Installation failed: {e}"),
                            command: install_command.clone(),
                            timestamp: Instant::now(),
                        });
                    }

                    tracing::error!("Installation failed for {}: {}", tool_id, e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    pub fn uninstall_tool(&mut self, tool_id: String) {
        let tool_manager = self.tool_manager.clone();
        let tools_cache = Arc::clone(&self.tools_cache);
        let cache_manager = Arc::clone(&self.cache_manager);
        let runtime = Arc::clone(&self.runtime);
        let ctx = self.ctx.clone();
        let sender = if let Ok(sender_guard) = self.install_sender.lock() {
            sender_guard.clone()
        } else {
            None
        };

        // 获取卸载命令用于显示目的
        let uninstall_command = if let Ok(tools) = self.tools_cache.lock() {
            if let Some(tool_info) = tools.iter().find(|t| t.config.id == tool_id) {
                let platform = std::env::consts::OS;
                if let Some(install_method) = tool_info.config.install.get(platform) {
                    // 根据安装方法构造卸载命令
                    match install_method.method.as_str() {
                        "npm" => install_method
                            .package_name
                            .as_ref()
                            .map(|pkg| format!("npm uninstall -g {pkg}")),
                        "brew" => install_method
                            .package_name
                            .as_ref()
                            .map(|pkg| format!("brew uninstall {pkg}")),
                        "pip" => install_method
                            .package_name
                            .as_ref()
                            .map(|pkg| format!("pip uninstall -y {pkg}")),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let handle = runtime.spawn(async move {
            // 发送进度更新 - 开始
            if let Some(sender) = &sender {
                let _ = sender.send(InstallProgress {
                    tool_id: tool_id.clone(),
                    tool_name: tool_id.clone(),
                    operation: InstallOperation::Uninstall,
                    status: ProgressStatus::InProgress,
                    message: "Starting uninstallation...".to_string(),
                    command: uninstall_command.clone(),
                    timestamp: Instant::now(),
                });
            }

            match tool_manager.uninstall_tool(&tool_id).await {
                Ok(_) => {
                    // 卸载后重新检查状态
                    let new_status = tool_manager.check_tool_status(&tool_id).await;

                    // 更新工具缓存状态
                    if let Ok(status) = &new_status {
                        if let Ok(mut tools) = tools_cache.lock() {
                            for tool in tools.iter_mut() {
                                if tool.config.id == tool_id {
                                    tool.status = status.clone();
                                    break;
                                }
                            }
                        }
                    }

                    // 使用我们已经检查的状态更新缓存
                    if let Ok(status) = &new_status {
                        let cache_manager_clone = Arc::clone(&cache_manager);
                        let status_clone = status.clone();
                        let tool_id_for_cache = tool_id.clone();

                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                if let Ok(mut cache) = cache_manager_clone.lock() {
                                    cache.set_tool_status(&tool_id_for_cache, status_clone);
                                    let _ = cache.save().await;
                                }
                            });
                        });
                    }

                    // 发送完成进度
                    if let Some(sender) = &sender {
                        let _ = sender.send(InstallProgress {
                            tool_id: tool_id.clone(),
                            tool_name: tool_id.clone(),
                            operation: InstallOperation::Uninstall,
                            status: ProgressStatus::Completed,
                            message: "Uninstallation completed successfully".to_string(),
                            command: uninstall_command.clone(),
                            timestamp: Instant::now(),
                        });
                    }

                    if let Some(context) = &ctx {
                        context.request_repaint();
                    }
                }
                Err(e) => {
                    // 发送失败进度，包含详细错误信息
                    if let Some(sender) = &sender {
                        let _ = sender.send(InstallProgress {
                            tool_id: tool_id.clone(),
                            tool_name: tool_id.clone(),
                            operation: InstallOperation::Uninstall,
                            status: ProgressStatus::Failed,
                            message: format!("Uninstallation failed: {e}"),
                            command: uninstall_command.clone(),
                            timestamp: Instant::now(),
                        });
                    }

                    tracing::error!("Uninstallation failed for {}: {}", tool_id, e);
                }
            }
        });

        if let Ok(mut tasks) = self.background_tasks.lock() {
            tasks.push(handle);
        }
    }

    fn get_all_logs_as_text(&self) -> String {
        let mut combined_entries = Vec::new();

        // 添加状态检查进度条目
        for progress in &self.app_state.status_progress {
            let (icon, _color_name) = match &progress.status {
                ProgressStatus::Pending => ("⏳", "GRAY"),
                ProgressStatus::InProgress => ("🔄", "BLUE"),
                ProgressStatus::Completed => ("✅", "GREEN"),
                ProgressStatus::Failed => ("❌", "RED"),
            };

            let entry = format!(
                "[STATUS] {} {} - {}",
                icon, progress.tool_name, progress.message
            );
            combined_entries.push((progress.timestamp, entry));
        }

        // 添加安装进度条目
        for progress in &self.app_state.install_progress {
            let (icon, _color_name) = match &progress.status {
                ProgressStatus::Pending => ("⏳", "GRAY"),
                ProgressStatus::InProgress => ("🔄", "BLUE"),
                ProgressStatus::Completed => ("✅", "GREEN"),
                ProgressStatus::Failed => ("❌", "RED"),
            };

            let operation = match &progress.operation {
                InstallOperation::Install => "INSTALL",
                InstallOperation::Uninstall => "UNINSTALL",
                InstallOperation::Update => "UPDATE",
            };

            let mut entry = format!(
                "[{}] {} {} - {}",
                operation, icon, progress.tool_name, progress.message
            );

            // 添加命令信息（如果可用）
            if let Some(command) = &progress.command {
                entry.push_str(&format!("\n   Command: {command}"));
            }

            combined_entries.push((progress.timestamp, entry));
        }

        // 按时间戳排序（最新的在最后）
        combined_entries.sort_by_key(|&(timestamp, _)| timestamp);

        let mut result = String::new();
        for (timestamp, entry) in combined_entries {
            // 格式化时间戳为事件发生的实际时间 (HH:MM:SS)
            let time_str = {
                use chrono::{DateTime, Local};

                // 计算事件发生的实际时间
                let now = std::time::SystemTime::now();
                let event_time = now - timestamp.elapsed();

                // 转换为本地日期时间
                let datetime: DateTime<Local> = event_time.into();
                datetime.format("%H:%M:%S").to_string()
            };

            result.push_str(&format!("[{time_str}] {entry}\n"));
        }

        if result.is_empty() {
            "No operations logged yet.".to_string()
        } else {
            result
        }
    }

    fn render_method_form(ui: &mut egui::Ui, method: &mut InstallMethodForm, id_prefix: &str) {
        egui::Grid::new(format!("{id_prefix}_grid"))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("Method:");
                egui::ComboBox::from_id_source(format!("{id_prefix}_method"))
                    .selected_text(&method.method)
                    .show_ui(ui, |ui| {
                        // 跨平台包管理器
                        ui.selectable_value(&mut method.method, "npm".to_string(), "npm (Node.js)");
                        ui.selectable_value(&mut method.method, "pip".to_string(), "pip (Python)");
                        ui.selectable_value(
                            &mut method.method,
                            "cargo".to_string(),
                            "cargo (Rust)",
                        );
                        ui.selectable_value(&mut method.method, "go".to_string(), "go (Go)");
                        ui.selectable_value(&mut method.method, "gem".to_string(), "gem (Ruby)");

                        ui.separator();

                        // Windows 包管理器
                        ui.selectable_value(
                            &mut method.method,
                            "winget".to_string(),
                            "winget (Windows)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "choco".to_string(),
                            "choco (Chocolatey)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "scoop".to_string(),
                            "scoop (Scoop)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "powershell".to_string(),
                            "powershell (PowerShell)",
                        );

                        ui.separator();

                        // macOS 包管理器
                        ui.selectable_value(
                            &mut method.method,
                            "brew".to_string(),
                            "brew (Homebrew)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "port".to_string(),
                            "port (MacPorts)",
                        );

                        ui.separator();

                        // Linux 包管理器
                        ui.selectable_value(
                            &mut method.method,
                            "apt".to_string(),
                            "apt (Debian/Ubuntu)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "yum".to_string(),
                            "yum (RHEL/CentOS)",
                        );
                        ui.selectable_value(&mut method.method, "dnf".to_string(), "dnf (Fedora)");
                        ui.selectable_value(
                            &mut method.method,
                            "pacman".to_string(),
                            "pacman (Arch)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "zypper".to_string(),
                            "zypper (openSUSE)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "emerge".to_string(),
                            "emerge (Gentoo)",
                        );
                        ui.selectable_value(&mut method.method, "apk".to_string(), "apk (Alpine)");
                        ui.selectable_value(
                            &mut method.method,
                            "snap".to_string(),
                            "snap (Snapcraft)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "flatpak".to_string(),
                            "flatpak (Flatpak)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "appimage".to_string(),
                            "appimage (AppImage)",
                        );

                        ui.separator();

                        // 通用方法
                        ui.selectable_value(
                            &mut method.method,
                            "curl".to_string(),
                            "curl (Download)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "wget".to_string(),
                            "wget (Download)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "script".to_string(),
                            "script (Custom Script)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "binary".to_string(),
                            "binary (Direct Binary)",
                        );
                        ui.selectable_value(
                            &mut method.method,
                            "manual".to_string(),
                            "manual (Manual Install)",
                        );
                    });
                ui.end_row();

                ui.label("Package Name:");
                ui.text_edit_singleline(&mut method.package_name);
                ui.end_row();

                ui.label("Command Args:");
                ui.text_edit_singleline(&mut method.command_args);
                ui.end_row();

                if method.method == "script" || method.method == "curl" || method.method == "wget" {
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut method.url);
                    ui.end_row();
                }
            });
    }

    fn render_about(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("ℹ About CLIverge");
            ui.separator();

            ui.label(format!("Version: {VERSION}"));
            ui.add_space(10.0);

            ui.label("CLIverge is a lightweight CLI tool manager that simplifies");
            ui.label("the installation and management of various CLI development tools.");
            ui.add_space(20.0);

            ui.group(|ui| {
                ui.label("📋 Key Features:");
                ui.label("• One-click installation of CLI development tools");
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
            ToolStatus::Installed { version } => format!("Installed (v{version})"),
            ToolStatus::NotInstalled => "Not Installed".to_string(),
            ToolStatus::Error(msg) => format!("Error: {msg}"),
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
                    format!("  {trimmed}")
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
            (
                egui::TextStyle::Heading,
                egui::FontId::new(base_size * 1.6, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(base_size, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(base_size, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(base_size, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(base_size * 0.83, egui::FontFamily::Proportional),
            ),
        ]
        .into();

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
            let _ = std::process::Command::new("open").arg(&cache_path).spawn();
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
                    let _ = std::process::Command::new(editor).arg(&tools_path).spawn();
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
            let _ = std::process::Command::new("open").arg(&config_dir).spawn();
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
                format!(
                    "Auto check enabled - will check every {} minutes",
                    self.app_state.settings_check_interval
                ),
                NotificationLevel::Info,
            );
        }
    }

    fn check_auto_update_timer(&mut self) {
        if !self.app_state.settings_auto_check_updates {
            return;
        }

        let should_check = if let Some(last_check) = self.app_state.last_auto_check {
            let elapsed = last_check.elapsed();
            let interval =
                Duration::from_secs((self.app_state.settings_check_interval as u64) * 60);
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
                    NotificationLevel::Info,
                );
            }

            // Start background status checking
            self.start_background_status_checking();
        }
    }

    // Tool editor methods

    /// Open the tool editor for creating a new tool
    fn open_tool_editor_for_new_tool(&mut self) {
        self.app_state.show_tool_editor = true;
        self.app_state.editing_tool_id = None;
        self.app_state.tool_form_state = ToolFormState::default();

        // Generate a default ID based on name (will be updated when name is entered)
        self.app_state.tool_form_state.id = format!("new-tool-{}", chrono::Utc::now().timestamp());
    }

    /// Open the tool editor for editing an existing tool
    fn open_tool_editor_for_existing_tool(&mut self, tool_id: &str) {
        // Clone tool config to avoid borrowing issues
        let tool_config = if let Ok(tools) = self.tools_cache.lock() {
            tools
                .iter()
                .find(|t| t.config.id == tool_id)
                .map(|t| t.config.clone())
        } else {
            None
        };

        if let Some(config) = tool_config {
            self.app_state.show_tool_editor = true;
            self.app_state.editing_tool_id = Some(tool_id.to_string());

            // Populate form with existing tool data
            self.populate_form_from_tool_config(&config);
        }
    }

    /// Populate the form state from an existing tool configuration
    fn populate_form_from_tool_config(&mut self, tool_config: &cliverge_core::ToolConfig) {
        self.app_state.tool_form_state.id = tool_config.id.clone();
        self.app_state.tool_form_state.name = tool_config.name.clone();
        self.app_state.tool_form_state.description = tool_config.description.clone();
        self.app_state.tool_form_state.website = tool_config.website.clone();
        self.app_state.tool_form_state.command = tool_config.command.clone();

        // Convert version check args from HashMap<String, Vec<String>> to per-platform strings
        for (platform, args) in &tool_config.version_check {
            if let Some(form_args) = self
                .app_state
                .tool_form_state
                .version_check_methods
                .get_mut(platform)
            {
                *form_args = args.join(" ");
            }
        }

        // Convert update check args (if present)
        if let Some(update_check_configs) = &tool_config.update_check {
            for (platform, args) in update_check_configs {
                if let Some(form_args) = self
                    .app_state
                    .tool_form_state
                    .update_check_methods
                    .get_mut(platform)
                {
                    *form_args = args.join(" ");
                }
            }
        }

        // Populate install methods
        for (platform, install_method) in &tool_config.install {
            if let Some(form_method) = self
                .app_state
                .tool_form_state
                .install_methods
                .get_mut(platform)
            {
                form_method.method = install_method.method.clone();
                form_method.command_args = install_method
                    .command
                    .as_ref()
                    .map(|args| args.join(" "))
                    .unwrap_or_default();
                form_method.url = install_method.url.clone().unwrap_or_default();
                form_method.package_name = install_method.package_name.clone().unwrap_or_default();
            }
        }

        // Clear errors and validate
        self.app_state.tool_form_state.errors.clear();
        self.validate_tool_form();
    }

    /// Validate the tool form and update errors
    fn validate_tool_form(&mut self) {
        let form = &mut self.app_state.tool_form_state;
        form.errors.clear();

        // Required field validation
        if form.name.trim().is_empty() {
            form.errors.push("Tool name is required".to_string());
        }

        if form.command.trim().is_empty() {
            form.errors.push("Command is required".to_string());
        }

        if form.description.trim().is_empty() {
            form.errors.push("Description is required".to_string());
        }

        if form.website.trim().is_empty() {
            form.errors.push("Website URL is required".to_string());
        } else if !form.website.starts_with("http://") && !form.website.starts_with("https://") {
            form.errors
                .push("Website must be a valid HTTP/HTTPS URL".to_string());
        }

        // Validate version check methods for all platforms
        let mut has_version_check = false;
        for args in form.version_check_methods.values() {
            if !args.trim().is_empty() {
                has_version_check = true;
                break;
            }
        }
        if !has_version_check {
            form.errors
                .push("At least one platform must have version check arguments".to_string());
        }

        // Check for duplicate names (only for new tools)
        if self.app_state.editing_tool_id.is_none() {
            if let Ok(tools) = self.tools_cache.lock() {
                if tools
                    .iter()
                    .any(|t| t.config.name.to_lowercase() == form.name.to_lowercase())
                {
                    form.errors
                        .push("A tool with this name already exists".to_string());
                }
            }
        }

        // Validate at least one install method is configured
        let has_valid_install_method = form.install_methods.values().any(|method| {
            !method.method.is_empty()
                && (!method.command_args.is_empty()
                    || !method.package_name.is_empty()
                    || !method.url.is_empty())
        });

        if !has_valid_install_method {
            form.errors
                .push("At least one platform install method must be configured".to_string());
        }

        form.is_valid = form.errors.is_empty();
    }

    /// Test a command to see if it's available
    fn test_tool_command(&mut self, command: &str, args: &str) {
        let command = command.trim();
        let args: Vec<&str> = args.split_whitespace().collect();

        if command.is_empty() {
            self.add_notification(
                "Command cannot be empty".to_string(),
                NotificationLevel::Error,
            );
            return;
        }

        let command = command.to_string();
        let args: Vec<String> = args.into_iter().map(|s| s.to_string()).collect();
        let runtime = Arc::clone(&self.runtime);

        // Test the command in the background
        runtime.spawn(async move {
            match tokio::process::Command::new(&command)
                .args(&args)
                .output()
                .await
            {
                Ok(output) => {
                    let success = output.status.success();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    // Note: We can't easily update the UI from here due to borrowing rules
                    // In a real implementation, we'd use channels or other async communication
                    println!(
                        "Command test result: success={success}, stdout={stdout}, stderr={stderr}"
                    );
                }
                Err(e) => {
                    println!("Command test failed: {e}");
                }
            }
        });

        self.add_notification("Testing command...".to_string(), NotificationLevel::Info);
    }

    /// Delete a tool from configuration and cache
    fn delete_tool(&mut self, tool_id: &str) {
        let tool_name = if let Ok(tools) = self.tools_cache.lock() {
            tools
                .iter()
                .find(|t| t.config.id == tool_id)
                .map(|t| t.config.name.clone())
                .unwrap_or_else(|| tool_id.to_string())
        } else {
            tool_id.to_string()
        };

        // Remove from configuration
        let config_success = if let Ok(mut config_manager) = self.config_manager.lock() {
            // Remove tool from configuration (this method returns () not Result)
            config_manager.remove_tool(tool_id);

            // Save asynchronously in background
            let config_manager_clone = Arc::clone(&self.config_manager);
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(config) = config_manager_clone.lock() {
                        if let Err(e) = config.save().await {
                            tracing::error!("Failed to save config after deletion: {}", e);
                        }
                    }
                });
            });
            true
        } else {
            false
        };

        if config_success {
            // Remove from tools cache
            if let Ok(mut tools) = self.tools_cache.lock() {
                tools.retain(|tool| tool.config.id != tool_id);
            }

            // Remove from help cache
            if let Ok(mut help_cache) = self.help_cache.lock() {
                help_cache.remove(tool_id);
            }

            // Remove from persistent cache
            if let Ok(mut cache) = self.cache_manager.lock() {
                cache.invalidate_tool(tool_id);

                // Save cache in background
                let cache_manager_clone = Arc::clone(&self.cache_manager);
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Ok(cache) = cache_manager_clone.lock() {
                            if let Err(e) = cache.save().await {
                                tracing::warn!("Failed to save cache after tool deletion: {}", e);
                            }
                        }
                    });
                });
            }

            // Clear selection if the deleted tool was selected
            if self.app_state.selected_tool.as_ref() == Some(&tool_id.to_string()) {
                self.app_state.selected_tool = None;
            }

            self.add_notification(
                format!("Tool '{tool_name}' deleted successfully"),
                NotificationLevel::Success,
            );
        } else {
            self.add_notification(
                format!("Failed to delete tool '{tool_name}'"),
                NotificationLevel::Error,
            );
        }
    }

    /// Save the tool configuration from the form data
    fn save_tool_from_form(&mut self) -> bool {
        // Double-check validation before saving
        self.validate_tool_form();
        if !self.app_state.tool_form_state.is_valid {
            self.add_notification(
                "Cannot save: form has validation errors".to_string(),
                NotificationLevel::Error,
            );
            return false;
        }

        let form = &self.app_state.tool_form_state;

        // Create tool config from form data
        let tool_config = cliverge_core::ToolConfig {
            id: form.id.clone(),
            name: form.name.clone(),
            description: form.description.clone(),
            website: form.website.clone(),
            command: form.command.clone(),
            version_check: form
                .version_check_methods
                .iter()
                .filter_map(|(platform, args)| {
                    if !args.trim().is_empty() {
                        Some((
                            platform.clone(),
                            args.split_whitespace().map(|s| s.to_string()).collect(),
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
            update_check: {
                let update_configs: UpdateConfigMethods = form
                    .update_check_methods
                    .iter()
                    .filter_map(|(platform, args)| {
                        if !args.trim().is_empty() {
                            Some((
                                platform.clone(),
                                args.split_whitespace().map(|s| s.to_string()).collect(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                if update_configs.is_empty() {
                    None
                } else {
                    Some(update_configs)
                }
            },
            install: form
                .install_methods
                .iter()
                .filter_map(|(platform, method)| {
                    // Only include methods that have some configuration
                    if method.method.is_empty()
                        || (method.command_args.is_empty()
                            && method.package_name.is_empty()
                            && method.url.is_empty())
                    {
                        return None;
                    }

                    let install_method = cliverge_core::InstallMethod {
                        method: method.method.clone(),
                        command: if method.command_args.trim().is_empty() {
                            None
                        } else {
                            Some(
                                method
                                    .command_args
                                    .split_whitespace()
                                    .map(|s| s.to_string())
                                    .collect(),
                            )
                        },
                        url: if method.url.trim().is_empty() {
                            None
                        } else {
                            Some(method.url.clone())
                        },
                        package_name: if method.package_name.trim().is_empty() {
                            None
                        } else {
                            Some(method.package_name.clone())
                        },
                    };

                    Some((platform.clone(), install_method))
                })
                .collect(),
            uninstall: None,     // TODO: Add uninstall configuration in form
            update: None,        // TODO: Add update configuration in form
            config_schema: None, // Not editable in form for now
        };

        // Save configuration
        match self.app_state.editing_tool_id {
            Some(ref existing_id) => {
                // Update existing tool
                let success = if let Ok(mut config_manager) = self.config_manager.lock() {
                    config_manager.update_tool_config(existing_id, tool_config.clone());
                    // Save asynchronously in background
                    let config_manager_clone = Arc::clone(&self.config_manager);
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Ok(config) = config_manager_clone.lock() {
                                if let Err(e) = config.save().await {
                                    tracing::error!("Failed to save config: {}", e);
                                }
                            }
                        });
                    });

                    // Update tools cache
                    if let Ok(mut tools) = self.tools_cache.lock() {
                        if let Some(tool_info) =
                            tools.iter_mut().find(|t| t.config.id == *existing_id)
                        {
                            tool_info.config = tool_config;
                        }
                    }

                    true
                } else {
                    false
                };

                if success {
                    self.add_notification(
                        format!("Tool '{}' updated successfully", form.name),
                        NotificationLevel::Success,
                    );
                    true
                } else {
                    self.add_notification(
                        "Failed to access configuration manager".to_string(),
                        NotificationLevel::Error,
                    );
                    false
                }
            }
            None => {
                // Add new tool
                let success = if let Ok(mut config_manager) = self.config_manager.lock() {
                    config_manager.add_tool(tool_config.clone());
                    // Save asynchronously in background
                    let config_manager_clone = Arc::clone(&self.config_manager);
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Ok(config) = config_manager_clone.lock() {
                                if let Err(e) = config.save().await {
                                    tracing::error!("Failed to save config: {}", e);
                                }
                            }
                        });
                    });

                    // Add to tools cache
                    if let Ok(mut tools) = self.tools_cache.lock() {
                        let tool_info = ToolInfo {
                            config: tool_config,
                            status: ToolStatus::Unknown,
                            version_info: None,
                            user_config: HashMap::new(),
                        };
                        tools.push(tool_info);
                    }

                    true
                } else {
                    false
                };

                if success {
                    self.add_notification(
                        format!("Tool '{}' added successfully", form.name),
                        NotificationLevel::Success,
                    );
                    true
                } else {
                    self.add_notification(
                        "Failed to access configuration manager".to_string(),
                        NotificationLevel::Error,
                    );
                    false
                }
            }
        }
    }

    /// Render the tool editor form UI
    fn render_tool_editor(&mut self, ui: &mut egui::Ui) {
        // Validate form on each render to ensure errors are up-to-date
        self.validate_tool_form();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Error display at the top
            if !self.app_state.tool_form_state.errors.is_empty() {
                ui.group(|ui| {
                    ui.colored_label(egui::Color32::RED, "⚠ Validation Errors:");
                    for error in &self.app_state.tool_form_state.errors {
                        ui.label(format!("• {error}"));
                    }
                });
                ui.add_space(10.0);
            }

            // Basic Information Section
            ui.collapsing("📝 Basic Information", |ui| {
                egui::Grid::new("basic_info_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Tool Name *:");
                        let name_response =
                            ui.text_edit_singleline(&mut self.app_state.tool_form_state.name);
                        if name_response.changed() && self.app_state.editing_tool_id.is_none() {
                            // Auto-generate ID from name for new tools
                            self.app_state.tool_form_state.id = self
                                .app_state
                                .tool_form_state
                                .name
                                .to_lowercase()
                                .replace(" ", "-")
                                .replace("_", "-")
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == '-')
                                .collect();
                        }
                        ui.end_row();

                        ui.label("Tool ID *:");
                        ui.add_enabled(
                            self.app_state.editing_tool_id.is_none(),
                            egui::TextEdit::singleline(&mut self.app_state.tool_form_state.id)
                                .hint_text("auto-generated-from-name"),
                        );
                        ui.end_row();

                        ui.label("Command *:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.app_state.tool_form_state.command);
                            if ui.button("🧪 Test").clicked() {
                                let command = self.app_state.tool_form_state.command.clone();
                                // Use current platform's version check args for testing
                                let platform = std::env::consts::OS;
                                let args = self
                                    .app_state
                                    .tool_form_state
                                    .version_check_methods
                                    .get(platform)
                                    .cloned()
                                    .unwrap_or_default();
                                self.test_tool_command(&command, &args);
                            }
                        });
                        ui.end_row();

                        ui.label("Description *:");
                        ui.text_edit_multiline(&mut self.app_state.tool_form_state.description);
                        ui.end_row();

                        ui.label("Website *:");
                        ui.text_edit_singleline(&mut self.app_state.tool_form_state.website);
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            // Version Checking Section
            ui.collapsing("🔍 Version Checking", |ui| {
                ui.label("Configure version and update check commands for different platforms:");
                ui.add_space(5.0);

                for platform in ["windows", "macos", "linux"] {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let platform_icon = match platform {
                                "windows" => "🪟",
                                "macos" => "🍎",
                                "linux" => "🐧",
                                _ => "💻",
                            };
                            ui.label(format!("{} {}", platform_icon, platform.to_uppercase()));
                        });

                        egui::Grid::new(format!("{platform}_version_grid"))
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Version Check Args *:");
                                let mut test_clicked = false;
                                let mut command_to_test = String::new();
                                let mut args_to_test = String::new();

                                if let Some(version_args) = self
                                    .app_state
                                    .tool_form_state
                                    .version_check_methods
                                    .get_mut(platform)
                                {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(version_args);
                                        if ui.button("🧪 Test").clicked() {
                                            test_clicked = true;
                                            command_to_test =
                                                self.app_state.tool_form_state.command.clone();
                                            args_to_test = version_args.clone();
                                        }
                                    });
                                }

                                if test_clicked {
                                    self.test_tool_command(&command_to_test, &args_to_test);
                                }
                                ui.end_row();

                                ui.label("Update Check Args:");
                                if let Some(update_args) = self
                                    .app_state
                                    .tool_form_state
                                    .update_check_methods
                                    .get_mut(platform)
                                {
                                    ui.text_edit_singleline(update_args);
                                }
                                ui.end_row();
                            });
                    });
                    ui.add_space(8.0);
                }
                ui.small("Tip: Use space-separated arguments like '--version' or 'update --check'");
            });

            ui.add_space(10.0);

            // Installation Methods Section
            ui.collapsing("⚙ Installation Methods", |ui| {
                ui.label("Configure how this tool can be installed on different platforms:");
                ui.add_space(5.0);

                for platform in ["windows", "macos", "linux"] {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let platform_icon = match platform {
                                "windows" => "🪟",
                                "macos" => "🍎",
                                "linux" => "🐧",
                                _ => "💻",
                            };
                            ui.label(format!("{} {}", platform_icon, platform.to_uppercase()));
                        });

                        if let Some(install_method) = self
                            .app_state
                            .tool_form_state
                            .install_methods
                            .get_mut(platform)
                        {
                            Self::render_method_form(
                                ui,
                                install_method,
                                &format!("{platform}_install"),
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
            });

            ui.add_space(10.0);

            // Uninstallation Methods Section
            ui.collapsing("🗑 Uninstallation Methods", |ui| {
                ui.label("Configure how this tool can be uninstalled on different platforms:");
                ui.add_space(5.0);

                for platform in ["windows", "macos", "linux"] {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let platform_icon = match platform {
                                "windows" => "🪟",
                                "macos" => "🍎",
                                "linux" => "🐧",
                                _ => "💻",
                            };
                            ui.label(format!("{} {}", platform_icon, platform.to_uppercase()));
                        });

                        if let Some(uninstall_method) = self
                            .app_state
                            .tool_form_state
                            .uninstall_methods
                            .get_mut(platform)
                        {
                            Self::render_method_form(
                                ui,
                                uninstall_method,
                                &format!("{platform}_uninstall"),
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
            });

            ui.add_space(10.0);

            // Update Methods Section
            ui.collapsing("🔄 Update Methods", |ui| {
                ui.label("Configure how this tool can be updated on different platforms:");
                ui.add_space(5.0);

                for platform in ["windows", "macos", "linux"] {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let platform_icon = match platform {
                                "windows" => "🪟",
                                "macos" => "🍎",
                                "linux" => "🐧",
                                _ => "💻",
                            };
                            ui.label(format!("{} {}", platform_icon, platform.to_uppercase()));
                        });

                        if let Some(update_method) = self
                            .app_state
                            .tool_form_state
                            .update_methods
                            .get_mut(platform)
                        {
                            Self::render_method_form(
                                ui,
                                update_method,
                                &format!("{platform}_update"),
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        });

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            let save_text = if self.app_state.editing_tool_id.is_some() {
                "💾 Update Tool"
            } else {
                "💾 Create Tool"
            };

            if ui
                .add_enabled(
                    self.app_state.tool_form_state.is_valid,
                    egui::Button::new(save_text),
                )
                .clicked()
                && self.save_tool_from_form()
            {
                self.app_state.show_tool_editor = false;
            }

            if ui.button("❌ Cancel").clicked() {
                self.app_state.show_tool_editor = false;
            }

            // Show delete button only for existing tools
            if self.app_state.editing_tool_id.is_some() && ui.button("🗑 Delete Tool").clicked() {
                // Show confirmation dialog
                self.app_state.tool_to_delete = self.app_state.editing_tool_id.clone();
                self.app_state.show_delete_confirmation = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Reset Form").clicked() {
                    if self.app_state.editing_tool_id.is_some() {
                        // Reset to original values for edit mode
                        let tool_config = if let Ok(tools) = self.tools_cache.lock() {
                            tools
                                .iter()
                                .find(|t| {
                                    Some(&t.config.id) == self.app_state.editing_tool_id.as_ref()
                                })
                                .map(|t| t.config.clone())
                        } else {
                            None
                        };

                        if let Some(config) = tool_config {
                            self.populate_form_from_tool_config(&config);
                        }
                    } else {
                        // Reset to defaults for new tool mode
                        self.app_state.tool_form_state = ToolFormState::default();
                    }
                }
            });
        });

        if !self.app_state.tool_form_state.is_valid {
            ui.add_space(5.0);
            ui.small("Please fix the errors above before saving.");
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
            let all_done =
                self.app_state.status_progress.iter().all(|p| {
                    matches!(p.status, ProgressStatus::Completed | ProgressStatus::Failed)
                });
            if all_done && !self.app_state.status_progress.is_empty() {
                self.app_state.is_refreshing = false;
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🚀 CLIverge - Universal CLI Tool Manager");

                ui.separator();

                let refresh_button_text = if self.app_state.is_refreshing {
                    "⏳ Refreshing..."
                } else {
                    "🔄 Refresh"
                };

                if ui
                    .add_enabled(
                        !self.app_state.is_refreshing,
                        egui::Button::new(refresh_button_text),
                    )
                    .clicked()
                {
                    self.refresh_tools_with_progress();
                }

                if ui.button("📊 Operations Log").clicked() {
                    self.app_state.bottom_log_panel_open = !self.app_state.bottom_log_panel_open;
                }

                if ui.button("⚙ Settings").clicked() {
                    self.app_state.current_view = AppView::Settings;
                }

                if ui.button("ℹ About").clicked() {
                    self.app_state.current_view = AppView::About;
                }
            });
        });

        // Show notifications
        for (i, notification) in self.app_state.notifications.iter().enumerate() {
            let age = notification.timestamp.elapsed().as_secs_f32();
            if age < 5.0 {
                // Show for 5 seconds
                let alpha = (1.0 - age / 5.0).max(0.0);
                let color = match notification.level {
                    NotificationLevel::Info => {
                        egui::Color32::from_rgba_unmultiplied(70, 130, 180, (255.0 * alpha) as u8)
                    }
                    NotificationLevel::Success => {
                        egui::Color32::from_rgba_unmultiplied(34, 139, 34, (255.0 * alpha) as u8)
                    }
                    NotificationLevel::Warning => {
                        egui::Color32::from_rgba_unmultiplied(255, 165, 0, (255.0 * alpha) as u8)
                    }
                    NotificationLevel::Error => {
                        egui::Color32::from_rgba_unmultiplied(220, 20, 60, (255.0 * alpha) as u8)
                    }
                };

                egui::Window::new(format!("notification_{i}"))
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
        self.app_state
            .notifications
            .retain(|n| n.timestamp.elapsed().as_secs_f32() < 5.0);

        // Delete confirmation dialog
        if self.app_state.show_delete_confirmation {
            let tool_name = if let Some(tool_id) = &self.app_state.tool_to_delete {
                if let Ok(tools) = self.tools_cache.lock() {
                    tools
                        .iter()
                        .find(|t| &t.config.id == tool_id)
                        .map(|t| t.config.name.clone())
                        .unwrap_or_else(|| tool_id.clone())
                } else {
                    tool_id.clone()
                }
            } else {
                "Unknown Tool".to_string()
            };

            let mut window_open = true;
            egui::Window::new("⚠ Confirm Delete")
                .open(&mut window_open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label("Are you sure you want to delete this tool?");
                        ui.add_space(5.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 255, 255),
                            format!("Tool: {tool_name}"),
                        );
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::YELLOW, "⚠ This action cannot be undone!");
                        ui.add_space(15.0);

                        ui.horizontal(|ui| {
                            if ui.button("🗑 Delete").clicked() {
                                if let Some(tool_id) = self.app_state.tool_to_delete.take() {
                                    self.delete_tool(&tool_id);
                                }
                                self.app_state.show_delete_confirmation = false;
                                self.app_state.show_tool_editor = false;
                            }

                            if ui.button("❌ Cancel").clicked() {
                                self.app_state.show_delete_confirmation = false;
                                self.app_state.tool_to_delete = None;
                            }
                        });
                        ui.add_space(10.0);
                    });
                });

            // If window was closed by X button, also close the confirmation
            if !window_open {
                self.app_state.show_delete_confirmation = false;
                self.app_state.tool_to_delete = None;
            }
        }

        // Tool editor window
        let mut show_tool_editor = self.app_state.show_tool_editor;
        if show_tool_editor {
            let title = if self.app_state.editing_tool_id.is_some() {
                "✏ Edit AI Tool"
            } else {
                "➕ Add New AI Tool"
            };

            egui::Window::new(title)
                .open(&mut show_tool_editor)
                .resizable(true)
                .default_size([600.0, 700.0])
                .show(ctx, |ui| {
                    self.render_tool_editor(ui);
                });

            // Only update the state if the window wasn't closed by our buttons
            // The window's .open() will set show_tool_editor to false if user clicks X
            // But our buttons directly set self.app_state.show_tool_editor to false
            // So we only sync back if our internal state is still true
            if self.app_state.show_tool_editor {
                self.app_state.show_tool_editor = show_tool_editor;
            }
        }

        // 底部面板用于综合日志记录
        egui::TopBottomPanel::bottom("bottom_log_panel")
            .resizable(true)
            .default_height(if self.app_state.bottom_log_panel_open {
                250.0
            } else {
                50.0
            })
            .min_height(50.0)
            .max_height(600.0)
            .height_range(50.0..=600.0) // 关键：允许用户拖动调整
            .show(ctx, |ui| {
                self.render_comprehensive_log(ui);
            });

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
