use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct CliCommands {
    #[clap(subcommand)]
    cmd: CmdTypes,
}

#[derive(Subcommand, Clone)]
pub enum CmdTypes {
    Run { file_path: Option<String> },
    Update { file_path: String },
}

fn main() {}
