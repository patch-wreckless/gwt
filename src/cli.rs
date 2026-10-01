use clap::{Parser, Subcommand};

use crate::clone;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Clone(clone::Cmd),
}
