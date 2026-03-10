//! Agwaita CLI - Command-line interface for Agwaita Shell.
//!
//! This binary provides the main entry point for interacting with Agwaita,
//! a Wayland compositor shell. It supports various subcommands to launch
//! UI components, manage the session daemon, and control the top bar.

mod action;

use crate::action::CliAction;
use agw_lib_outcome::error::AgwError;
use clap::Parser;
use log::debug;

/// Trait for executing CLI commands.
pub trait CommandRun {
    /// Execute the command.
    ///
    /// # Errors
    /// Returns an `AgwError` if the command fails to execute.
    fn run(&self) -> Result<(), AgwError>;
}

#[derive(clap::Parser)]
#[command(name = "agwaita")]
#[command(version, about = "Agwaita Shell - A Wayland compositor shell", long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    action: Option<CliAction>,
}

fn main() {
    if let Err(err) = agw_lib_logger::initialize_logger() {
        err.print_and_exit()
    }

    debug!("Starting Agwaita CLI");

    let cli = Cli::parse();

    if let Some(action) = cli.action
        && let Err(err) = action.run()
    {
        err.print_and_exit();
    }
}
