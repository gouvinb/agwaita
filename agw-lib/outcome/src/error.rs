//! Error type for Agwaita.

use std::{
    error::Error,
    fmt::{
        Debug,
        Display,
        Formatter,
    },
};

/// Agwaita error type with exit code and optional help text.
#[derive(Debug)]
pub struct AgwError {
    pub code: i32,
    pub message: String,
    pub help: Option<String>,
    pub source: Option<Box<dyn Error>>,
}

impl Error for AgwError {}

impl Display for AgwError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut display = String::new();
        display.push_str(&format!("Error({}): {}", self.code, self.message));
        if let Some(help) = &self.help {
            display.push_str(&format!("\nhelp: {}", help));
        }
        if self.source.is_some() {
            display.push_str("\ndebug: Set `AGWAITA_LOG_LEVEL` to debug to view source error");
        }

        write!(f, "{}", display)
    }
}

impl AgwError {
    /// Create a new error with code and message.
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            help: None,
            source: None,
        }
    }

    /// Create a new error with code, message, and help text.
    pub fn with_help(code: i32, message: String, help: String) -> Self {
        Self {
            code,
            message,
            help: Some(help),
            source: None,
        }
    }

    /// Create a new error with code, message, and source error.
    pub fn with_source(code: i32, message: String, source: Box<dyn Error>) -> Self {
        Self {
            code,
            message,
            help: None,
            source: Some(source),
        }
    }

    /// Create a new error with code, message, help, and source.
    pub fn with_all(code: i32, message: String, help: String, source: Box<dyn Error>) -> Self {
        Self {
            code,
            message,
            help: Some(help),
            source: Some(source),
        }
    }

    /// Print the error to stderr and exit the process with the error code.
    pub fn print_and_exit(&self) -> ! {
        eprintln!("{}", self);
        std::process::exit(self.code);
    }
}
