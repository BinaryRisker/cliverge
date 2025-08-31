//! Core engine and business logic for CLIverge

pub mod config;
pub mod tool;
pub mod version;
pub mod error;
pub mod cache;

// Re-export main types for convenience
pub use config::*;
pub use tool::*;
pub use version::*;
pub use error::*;
pub use cache::*;

pub fn hello() {
    println!("Hello from cliverge-core!");
}
