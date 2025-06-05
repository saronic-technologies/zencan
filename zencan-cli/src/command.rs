use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{path::PathBuf, str::FromStr};

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
    /// Load a configuration from a file to a node
    LoadConfig(LoadConfigArgs),
    /// NMT commands
    Nmt(NmtArgs),
    /// LSS commands
    #[command(subcommand)]
    Lss(LssCommands),
}

#[derive(Debug, Args)]
pub struct LoadConfigArgs {
    /// The ID of the node to load the configuration into
    pub node_id: u8,
    /// Path to a node config TOML file
    #[arg(value_hint=clap::ValueHint::FilePath)]
    pub path: PathBuf,
}

/// Specifies a node to apply an NMT command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NmtNodeArg {
    All,
    Specific(u8),
}

impl NmtNodeArg {
    pub fn raw(&self) -> u8 {
        match self {
            Self::All => 0,
            Self::Specific(id) => *id,
        }
    }
}

impl FromStr for NmtNodeArg {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u8>() {
            Ok(num) => {
                if num == 0 {
                    Ok(Self::All)
                } else if num < 128 {
                    Ok(Self::Specific(num))
                } else {
                    Err("Node ID must be between 0 and 127")
                }
            }
            Err(_) => {
                if s == "all" {
                    Ok(Self::All)
                } else {
                    Err("Must specify a node ID, or 'all' to broadcast")
                }
            }
        }
    }
}

#[derive(Debug, Args)]
pub struct NmtArgs {
    pub action: NmtAction,
    /// Specify the node ID to command. Use '0' or 'all' to broadcast to all nodes.
    pub node: NmtNodeArg,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum NmtAction {
    ResetApp,
    ResetComms,
    Start,
    Stop,
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
