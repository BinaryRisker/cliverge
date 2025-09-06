# CLIverge 日志窗口功能改进方案

本文档详细说明了如何对 `CLIverge` GUI 的日志窗口进行一系列功能增强，以提升用户体验。

## 1. 总体目标

- **统一日志视图**: 将分散的日志（如状态检查、安装、卸载）合并到一个统一的"Operations Log"面板中，并移除多余的日志窗口。
- **提升可用性**: 允许用户自由选择和复制日志内容。
- **优化显示**: 将日志时间从倒计时改为标准的 `HH:MM:SS` 格式，并正确显示多行日志。
- **增强交互性**: 增加"复制全部日志"按钮，并修复日志面板无法自由调整大小的问题。

## 2. 功能特性

### 2.1 已实现的功能
- ✅ **时间显示改进**: 使用chrono库将时间戳格式化为HH:MM:SS格式
- ✅ **文本选择和复制**: 使用TextEdit::multiline替换selectable_label，启用interactive(true)
- ✅ **复制全部日志按钮**: 添加"📋 Copy All"按钮，可以复制所有日志内容
- ✅ **面板可调整大小**: 使用TopBottomPanel::bottom替代Window，支持resizable和height_range
- ✅ **删除多余窗口**: 移除了独立的Status Log窗口，统一使用底部面板
- ✅ **显示实际执行时间**: 计算事件发生的实际时间而不是倒计时
- ✅ **多行日志支持**: 正确处理和显示多行错误消息，带有适当的缩进
- ✅ **命令显示**: 在日志中显示实际执行的命令

### 2.2 UI 改进
- **底部面板**: 从独立窗口改为底部面板，更好地集成到主界面
- **可调整大小**: 用户可以通过拖拽调整日志面板的高小
- **自动滚动**: 新日志自动滚动到底部
- **防止自动收缩**: 设置auto_shrink([false, false])防止面板自动收缩

## 3. Cargo.toml 依赖准备

确保 `crates/cliverge-gui/Cargo.toml` 文件中已添加 `chrono` 依赖，用于时间格式化。

```toml
# crates/cliverge-gui/Cargo.toml

[dependencies]
# ... 其他依赖
chrono = { workspace = true, features = ["serde"] }
# ... 其他依赖
```

## 4. 核心数据结构修改 (`crates/cliverge-gui/src/app.rs`)

### 4.1 增加安装操作相关的结构体

为了区分安装、卸载等不同操作的日志，需要定义 `InstallOperation` 和 `InstallProgress` 结构体。将以下代码添加到 `ProgressStatus` 枚举定义之后。

```rust
// 位于 ProgressStatus 定义之后

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
    Update,
}
```

### 4.2 更新 `AppState` 结构体

1. 将 `log_window_open` 重命名为 `bottom_log_panel_open` 以更好地反映其功能。
2. 添加 `install_progress` 字段来存储安装、卸载等操作的日志。

```rust
// 在 struct AppState 定义中

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
    
    // ... 其他字段
}
```

### 4.3 更新 AppState 初始化

```rust
// 在 AppState::new() 或 Default 实现中

impl AppState {
    pub fn new() -> Self {
        Self {
            // ... 其他字段
            search_query: String::new(),
            show_only_installed: false,
            bottom_log_panel_open: true, // 修改此行，默认打开
            notifications: Vec::new(),
            current_view: AppView::Main,
            status_progress: Vec::new(),
            install_progress: Vec::new(), // 新增此行
            is_refreshing: false,
            // ... 其他字段
        }
    }
}
```

### 4.4 添加异步消息通道

在 `CLIvergeApp` 结构体中添加用于接收安装/卸载进度的通道。

```rust
// 在 struct CLIvergeApp 定义中

pub struct CLIvergeApp {
    // ... 其他字段
    progress_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<StatusCheckProgress>>>>,
    progress_receiver: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<StatusCheckProgress>>>>,
    install_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<InstallProgress>>>>, // 新增此行
    install_receiver: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<InstallProgress>>>>, // 新增此行
    ctx: Option<egui::Context>,
}
```

### 4.5 更新 CLIvergeApp 初始化

```rust
// 在 CLIvergeApp::new() 中

impl CLIvergeApp {
    pub fn new() -> Self {
        // ... 其他初始化代码
        
        // 创建进度通道
        let (progress_sender, progress_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (install_sender, install_receiver) = tokio::sync::mpsc::unbounded_channel(); // 新增此行

        let mut app = Self {
            // ... 其他字段
            progress_sender: Arc::new(Mutex::new(Some(progress_sender))),
            progress_receiver: Arc::new(Mutex::new(Some(progress_receiver))),
            install_sender: Arc::new(Mutex::new(Some(install_sender))), // 新增此行
            install_receiver: Arc::new(Mutex::new(Some(install_receiver))), // 新增此行
            ctx: None,
        };
        
        // ... 其他初始化代码
        app
    }
}
```

## 5. 日志处理与渲染逻辑 (`crates/cliverge-gui/src/app.rs`)

### 5.1 更新 `update_progress` 函数

修改此函数以同时处理状态检查和安装操作的日志消息。

```rust
// 在 impl CLIvergeApp 中

fn update_progress(&mut self) {
    // 更新状态检查进度 (原有逻辑)
    if let Ok(mut receiver_guard) = self.progress_receiver.lock() {
        if let Some(receiver) = receiver_guard.as_mut() {
            while let Ok(progress) = receiver.try_recv() {
                // 更新工具名称从缓存
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

                // 更新或添加状态进度条目
                if let Some(existing) = self.app_state.status_progress.iter_mut()
                    .find(|p| p.tool_id == updated_progress.tool_id) {
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
                    tools.iter()
                        .find(|tool| tool.config.id == progress.tool_id)
                        .map(|tool| tool.config.name.clone())
                        .unwrap_or_else(|| progress.tool_id.clone())
                } else {
                    progress.tool_id.clone()
                };

                let mut updated_progress = progress;
                updated_progress.tool_name = tool_name;

                // 更新或添加安装进度条目
                if let Some(existing) = self.app_state.install_progress.iter_mut()
                    .find(|p| p.tool_id == updated_progress.tool_id && std::mem::discriminant(&p.operation) == std::mem::discriminant(&updated_progress.operation)) {
                    *existing = updated_progress;
                } else {
                    self.app_state.install_progress.push(updated_progress);
                }
            }
        }
    }
}
```

### 5.2 创建新的综合日志渲染函数

创建 `render_comprehensive_log` 函数，完全替代旧的 `render_progress_log`。

```rust
// 在 impl CLIvergeApp 中

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
    let mut combined_entries: Vec<(Instant, String)> = Vec::new();

    // 添加状态检查条目
    for progress in &self.app_state.status_progress {
        let (icon, _color_name) = match &progress.status {
            ProgressStatus::Pending => ("⏳", "GRAY"),
            ProgressStatus::InProgress => ("🔄", "BLUE"),
            ProgressStatus::Completed => ("✅", "GREEN"),
            ProgressStatus::Failed => ("❌", "RED"),
        };
        
        let entry = format!("[STATUS] {} {} - {}", icon, progress.tool_name, progress.message);
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
        
        let mut entry = format!("[{}] {} {} - {}", operation, icon, progress.tool_name, progress.message);
        
        // 添加命令信息（如果可用）
        if let Some(command) = &progress.command {
            entry.push_str(&format!("\n   Command: {}", command));
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
                    ui.add(egui::TextEdit::multiline(&mut first_line_text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .interactive(true)
                        .frame(false));
                    
                    // 显示后续行时带缩进  
                    for line in &lines[1..] {
                        if !line.trim().is_empty() {
                            let indented_line = format!("           {}", line.trim());
                            let mut indented_text = indented_line.clone();
                            ui.add(egui::TextEdit::multiline(&mut indented_text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1)
                                .interactive(true)
                                .frame(false));
                        }
                    }
                } else {
                    // 单行消息 - 使整行可选择
                    let full_text = format!("[{}] {}", time_str, entry);
                    let mut text_copy = full_text.clone();
                    ui.add(egui::TextEdit::multiline(&mut text_copy)
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .interactive(true)
                        .frame(false));
                }
            }
        });
}
```

### 5.3 创建复制全部日志功能

```rust
// 在 impl CLIvergeApp 中

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
        
        let entry = format!("[STATUS] {} {} - {}", icon, progress.tool_name, progress.message);
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
        
        let mut entry = format!("[{}] {} {} - {}", operation, icon, progress.tool_name, progress.message);
        
        // 添加命令信息（如果可用）
        if let Some(command) = &progress.command {
            entry.push_str(&format!("\n   Command: {}", command));
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
        
        result.push_str(&format!("[{}] {}\n", time_str, entry));
    }

    if result.is_empty() {
        "No operations logged yet.".to_string()
    } else {
        result
    }
}
```

## 6. 主 UI 布局集成 (`crates/cliverge-gui/src/app.rs`)

### 6.1 移除旧的日志窗口逻辑

在 `CLIvergeApp::update` 函数中，找到并删除所有关于 `egui::Window::new("📊 Status Check Progress")` 的代码块。

**需要删除的代码示例：**
```rust
// 删除这样的代码块
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
```

### 6.2 添加新的底部日志面板

在 `CLIvergeApp::update` 函数中，在主要视图渲染之前，添加以下代码来渲染可调整大小的底部面板。

```rust
// 在 impl eframe::App for CLIvergeApp 的 update 函数中
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // ... (顶部菜单栏等逻辑)

    // 底部面板用于综合日志记录
    egui::TopBottomPanel::bottom("bottom_log_panel")
        .resizable(true)
        .default_height(if self.app_state.bottom_log_panel_open { 250.0 } else { 50.0 })
        .min_height(50.0)
        .max_height(600.0)
        .height_range(50.0..=600.0) // 关键：允许用户拖动调整
        .show(ctx, |ui| {
            self.render_comprehensive_log(ui);
        });

    // 主要视图渲染
    match self.app_state.current_view {
        AppView::Main => {
            egui::CentralPanel::default().show(ctx, |ui| {
                // ... (中央面板的主体内容)
            });
        }
        // ... 其他视图
    }
    
    // ... (通知等逻辑)
}
```

### 6.3 更新函数调用

如果代码中有其他地方调用 `render_progress_log`，需要将其更新为 `render_comprehensive_log`。

## 7. 工具操作函数更新 (`crates/cliverge-gui/src/app.rs`)

### 7.1 更新 `install_tool` 函数

确保 `install_tool` 函数会通过新创建的 `install_sender` 发送进度消息。

```rust
// 在 impl CLIvergeApp 中

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
            tool_info.config.install.get(platform)
                .and_then(|install_method| install_method.command.as_ref())
                .map(|cmd| cmd.join(" "))
                .or_else(|| {
                    // 回退：从方法和包名构造命令
                    tool_info.config.install.get(platform).map(|install_method| {
                        format!("{} install -g {}", 
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
                        message: format!("Installation failed: {}", e),
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
```

### 7.2 更新 `uninstall_tool` 函数

类似地更新 `uninstall_tool` 函数。

```rust
// 在 impl CLIvergeApp 中

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
                    "npm" => install_method.package_name.as_ref()
                        .map(|pkg| format!("npm uninstall -g {}", pkg)),
                    "brew" => install_method.package_name.as_ref()
                        .map(|pkg| format!("brew uninstall {}", pkg)),
                    "pip" => install_method.package_name.as_ref()
                        .map(|pkg| format!("pip uninstall -y {}", pkg)),
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
                        message: format!("Uninstallation failed: {}", e),
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
```

## 8. 实施步骤

### 8.1 准备阶段
1. 确保 `chrono` 依赖已添加到 `Cargo.toml`
2. 备份当前的 `app.rs` 文件

### 8.2 结构体修改
1. 添加 `InstallProgress` 和 `InstallOperation` 结构体
2. 更新 `AppState` 结构体，添加 `install_progress` 字段
3. 将 `log_window_open` 重命名为 `bottom_log_panel_open`
4. 更新 `CLIvergeApp` 结构体，添加安装进度通道

### 8.3 功能实现
1. 更新 `update_progress` 函数处理安装进度
2. 创建 `render_comprehensive_log` 函数
3. 创建 `get_all_logs_as_text` 函数
4. 更新 `install_tool` 和 `uninstall_tool` 函数

### 8.4 UI集成
1. 移除旧的日志窗口代码
2. 添加新的底部面板代码
3. 更新函数调用引用

### 8.5 测试
1. 编译并运行应用程序
2. 测试日志面板的显示和隐藏
3. 测试文本选择和复制功能
4. 测试"复制全部"按钮
5. 测试面板大小调整
6. 测试安装/卸载操作的日志显示

## 9. 故障排除

### 9.1 常见问题
- **编译错误**: 确保所有新结构体和字段都已正确添加
- **日志不显示**: 检查进度通道是否正确设置
- **文本无法选择**: 确保使用 `TextEdit::multiline` 并设置 `interactive(true)`
- **面板无法调整大小**: 确保设置了 `resizable(true)` 和 `height_range`

### 9.2 调试技巧
- 使用 `tracing::debug!` 添加调试日志
- 检查通道消息是否正确发送和接收
- 验证时间戳格式化是否正确

## 10. 总结

通过实施这个方案，CLIverge 将拥有一个功能完整、用户友好的日志系统，包括：

- 统一的操作日志视图
- 可选择和复制的文本
- 实际时间显示
- 可调整大小的面板
- 复制全部日志功能
- 多行日志支持
- 命令显示

这些改进将显著提升用户体验，使日志查看和故障排除变得更加容易。
