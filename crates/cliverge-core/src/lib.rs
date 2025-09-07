//! Core engine and business logic for CLIverge

pub mod cache;
pub mod config;
pub mod error;
pub mod tool;
pub mod version;

// Re-export main types for convenience
pub use cache::*;
pub use config::*;
pub use error::*;
pub use tool::*;
pub use version::*;

pub fn hello() {
    println!("Hello from cliverge-core!");
}
