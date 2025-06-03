use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Scan all node IDs to find configured devices
    Scan,
    /// Print info about nodes
    Info,
    /// LSS commands
    #[command(subcommand)]
    Lss(LssCommands),
}

#[derive(Debug, Subcommand)]
pub enum LssCommands {
    /// Perform a fastscan to find unconfigured nodes
    Fastscan,
    /// Globally enable or disable configuration mode
    Global {
        #[clap(action=clap::ArgAction::Set)]
        enable: bool,
    },
}
