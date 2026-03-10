//! Logger initialization for Agwaita.
//!
//! This module provides functionality to initialize the logging system using
//! `pretty_env_logger`. The log level is automatically set based on build mode
//! (Debug or Release) and can be overridden via the `AGWAITA_LOG_LEVEL` environment variable.

use agw_lib_outcome::error::AgwError;
use log::LevelFilter;

const ENV_VAR: &str = "AGWAITA_LOG_LEVEL";

/// Initialize the logger with appropriate settings.
///
/// Debug builds default to `Trace` level, release builds default to `Off`.
/// Override with the ` AGWAITA_LOG_LEVEL ` environment variable.
///
/// # Errors
/// Returns an error if the logger has already been initialized.
pub fn initialize_logger() -> Result<(), AgwError> {
    let level = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Off
    };

    pretty_env_logger::formatted_timed_builder()
        .default_format()
        .filter_level(level)
        .parse_env(ENV_VAR)
        .parse_default_env()
        .try_init()
        .map_err(|e| AgwError::with_source(1, "Logger initialization failed".to_string(), Box::new(e)))
}
