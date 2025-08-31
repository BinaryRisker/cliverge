//! Built-in AI CLI Tool Adapters for CLIverge
//! 
//! This crate provides built-in adapters for popular AI CLI tools

pub mod claude_code;
pub mod gemini;
pub mod qwen;
pub mod openai_codex;
pub mod opencode;
pub mod iflow;

pub use claude_code::ClaudeCodeTool;
pub use gemini::GeminiTool;
pub use qwen::QwenTool;
pub use openai_codex::OpenAiCodexTool;
pub use opencode::OpenCodeTool;
pub use iflow::IFlowTool;

/// Get all built-in tools
pub fn get_builtin_tools() -> Vec<Box<dyn cliverge_sdk::CliTool>> {
    vec![
        Box::new(ClaudeCodeTool::new()),
        Box::new(GeminiTool::new()),
        Box::new(QwenTool::new()),
        Box::new(OpenAiCodexTool::new()),
        Box::new(OpenCodeTool::new()),
        Box::new(IFlowTool::new()),
    ]
}
