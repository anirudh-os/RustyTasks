use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "RustyTasks")]
#[command(about = "A CRDT-powered CLI to-do list", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start interactive mode
    Interactive,

    /// Add a task
    Add {
        name: String,
    },

    /// Remove a task by index
    Remove {
        index: usize,
    },

    /// Mark a task as done by index
    Done {
        index: usize,
    },

    /// List all tasks
    List,
}
