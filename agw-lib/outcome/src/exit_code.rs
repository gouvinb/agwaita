//! Standard exit codes for Agwaita processes.

/// Successful program execution.
pub const SUCCESS: i32 = 0;

/// General failure.
pub const FAILURE: i32 = 1;

/// Process terminated by SIGINT (Ctrl+C).
pub const SIGINT: i32 = 130;

/// Service-specific error.
pub const SERVICE_ERROR: i32 = 50;
