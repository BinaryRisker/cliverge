use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone)]
pub struct Terminal {
    pub output: Arc<Mutex<Vec<String>>>,
    pub input: String,
    pub is_running: Arc<Mutex<bool>>,
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            output: Arc::new(Mutex::new(Vec::new())),
            input: String::new(),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
}

impl Terminal {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn clear(&mut self) {
        if let Ok(mut output) = self.output.lock() {
            output.clear();
        }
    }
    
    pub fn add_output(&self, line: String) {
        if let Ok(mut output) = self.output.lock() {
            output.push(line);
        }
    }
    
    pub async fn execute_command(&self, tool_id: &str, args: &str) {
        // Set running state
        if let Ok(mut running) = self.is_running.lock() {
            *running = true;
        }
        
        self.add_output(format!("> {} {}", tool_id, args));
        
        // Build command based on tool_id
        let (program, full_args) = match tool_id {
            "claude-code" => ("claude", args),
            "gemini-cli" => ("gemini", args),
            "qwen-code-cli" => ("qwen", args),
            "openai-codex" => ("openai", args),
            "opencode" => ("opencode", args),
            "iflow" => ("iflow", args),
            _ => {
                self.add_output(format!("Unknown tool: {}", tool_id));
                if let Ok(mut running) = self.is_running.lock() {
                    *running = false;
                }
                return;
            }
        };
        
        // Create command
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", &format!("{} {}", program, full_args)]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &format!("{} {}", program, full_args)]);
            c
        };
        
        // Capture output
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        match cmd.spawn() {
            Ok(mut child) => {
                // Read stdout
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        self.add_output(line);
                    }
                }
                
                // Read stderr
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        self.add_output(format!("[ERROR] {}", line));
                    }
                }
                
                // Wait for process to complete
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            self.add_output("Command completed successfully.".to_string());
                        } else {
                            self.add_output(format!("Command exited with status: {}", status));
                        }
                    }
                    Err(e) => {
                        self.add_output(format!("Failed to wait for command: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_output(format!("Failed to execute command: {}", e));
                self.add_output("Make sure the tool is installed and in your PATH.".to_string());
            }
        }
        
        // Clear running state
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|r| *r).unwrap_or(false)
    }
}
