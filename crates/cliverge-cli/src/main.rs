use clap::{Parser, Subcommand};
use cliverge_tools::get_builtin_tools;
use cliverge_sdk::ToolStatus;
use colored::Colorize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "cliverge")]
#[command(about = "A unified CLI tool manager for AI development tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available tools
    List,
    /// Show detailed status of all tools
    Status,
    /// Install a specific tool
    Install {
        /// Tool ID to install
        tool_id: String,
    },
    /// Uninstall a specific tool
    Uninstall {
        /// Tool ID to uninstall
        tool_id: String,
    },
    /// Update a specific tool
    Update {
        /// Tool ID to update
        tool_id: String,
        /// Target version (optional)
        #[arg(long)]
        version: Option<String>,
    },
    /// Execute a command with a specific tool
    Exec {
        /// Tool ID to use
        tool_id: String,
        /// Arguments to pass to the tool
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Show help for a specific tool
    ToolHelp {
        /// Tool ID to show help for
        tool_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Get all built-in tools
    let tools = get_builtin_tools();
    
    match cli.command {
        Commands::List => {
            println!("{}", "Available AI CLI Tools:".bright_blue().bold());
            println!();
            
            for tool in &tools {
                let status_indicator = match tool.status().await {
                    Ok(ToolStatus::Installed { .. }) => "✓".green(),
                    Ok(ToolStatus::NotInstalled) => "✗".red(),
                    Ok(ToolStatus::Error(_)) => "⚠".yellow(),
                    Err(_) => "?".red(),
                };
                
                println!("  {} {} - {}", 
                    status_indicator,
                    tool.id().bright_white().bold(),
                    tool.name().dimmed()
                );
            }
            println!();
            println!("{}", "Use 'cliverge status' for detailed information.".dimmed());
        }
        
        Commands::Status => {
            println!("{}", "Tool Status Report:".bright_blue().bold());
            println!();
            
            for tool in &tools {
                print!("{}:", tool.name().bright_white().bold());
                
                match tool.status().await {
                    Ok(ToolStatus::Installed { version }) => {
                        println!(" {} v{}", "Installed".green().bold(), version.green());
                    }
                    Ok(ToolStatus::NotInstalled) => {
                        println!(" {}", "Not Installed".red().bold());
                    }
                    Ok(ToolStatus::Error(msg)) => {
                        println!(" {} ({})", "Error".yellow().bold(), msg.yellow());
                    }
                    Err(e) => {
                        println!(" {} ({})", "Error".red().bold(), e.to_string().red());
                    }
                }
            }
        }
        
        Commands::Install { tool_id } => {
            if let Some(tool) = tools.iter().find(|t| t.id() == tool_id) {
                println!("{} {}...", "Installing".green().bold(), tool.name());
                
                match tool.install(&cliverge_sdk::InstallConfig::default()).await {
                    Ok(_) => {
                        println!("{} {} installed successfully!", "✓".green(), tool.name());
                    }
                    Err(e) => {
                        eprintln!("{} Failed to install {}: {}", "✗".red(), tool.name(), e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("{} Tool '{}' not found", "✗".red(), tool_id);
                std::process::exit(1);
            }
        }
        
        Commands::Uninstall { tool_id } => {
            if let Some(tool) = tools.iter().find(|t| t.id() == tool_id) {
                println!("{} {}...", "Uninstalling".yellow().bold(), tool.name());
                
                match tool.uninstall().await {
                    Ok(_) => {
                        println!("{} {} uninstalled successfully!", "✓".green(), tool.name());
                    }
                    Err(e) => {
                        eprintln!("{} Failed to uninstall {}: {}", "✗".red(), tool.name(), e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("{} Tool '{}' not found", "✗".red(), tool_id);
                std::process::exit(1);
            }
        }
        
        Commands::Update { tool_id, version } => {
            if let Some(tool) = tools.iter().find(|t| t.id() == tool_id) {
                let target_version = version.as_deref().unwrap_or("latest");
                println!("{} {} to version {}...", "Updating".blue().bold(), tool.name(), target_version);
                
                match tool.update(target_version).await {
                    Ok(_) => {
                        println!("{} {} updated successfully!", "✓".green(), tool.name());
                    }
                    Err(e) => {
                        eprintln!("{} Failed to update {}: {}", "✗".red(), tool.name(), e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("{} Tool '{}' not found", "✗".red(), tool_id);
                std::process::exit(1);
            }
        }
        
        Commands::Exec { tool_id, args } => {
            if let Some(tool) = tools.iter().find(|t| t.id() == tool_id) {
                match tool.execute(&args).await {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            print!("{}", output.stdout);
                        }
                        if !output.stderr.is_empty() {
                            eprint!("{}", output.stderr);
                        }
                        std::process::exit(output.exit_code);
                    }
                    Err(e) => {
                        eprintln!("{} Failed to execute {}: {}", "✗".red(), tool.name(), e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("{} Tool '{}' not found", "✗".red(), tool_id);
                std::process::exit(1);
            }
        }
        
        Commands::ToolHelp { tool_id } => {
            if let Some(tool) = tools.iter().find(|t| t.id() == tool_id) {
                println!("{}", tool.help());
            } else {
                eprintln!("{} Tool '{}' not found", "✗".red(), tool_id);
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}
