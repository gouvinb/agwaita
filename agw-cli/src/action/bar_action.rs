#[derive(clap::Subcommand, Debug)]
pub enum BarAction {
    /// Toggle the top bar visibility
    Toggle,
    /// Show the top bar
    Show,
    /// Hide the top bar
    Hide,
}
