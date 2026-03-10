#[derive(clap::Subcommand, Debug)]
pub enum NotificationAction {
    /// Close the last notification
    #[command(name = "close-last")]
    CloseLast,

    /// Do Not Disturb commands
    Dnd {
        #[command(subcommand)]
        command: DndAction,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum DndAction {
    /// Enable Do Not Disturb mode
    Enable,
    /// Disable Do Not Disturb mode
    Disable,
    /// Toggle Do Not Disturb mode
    Toggle,
    /// Show the current Do Not Disturb status
    Status,
}
