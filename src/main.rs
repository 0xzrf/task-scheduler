use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_yaml::Result;

#[derive(Parser)]
pub struct CliCommands {
    #[clap(subcommand)]
    cmd: CmdTypes,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Task {
    exec_path: String,
    from_hr: String,
    to_hr: String,
    daily: bool,
}

const DEFAULT_YAML_LOCATION: &str = "~/.config/task_scheduler/tasks.yml";

#[derive(Subcommand, Clone)]
pub enum CmdTypes {
    Run { file_path: Option<String> },
    Update { file_path: String },
}

impl CmdTypes {
    fn handle_cmd(&self) {
        match self {
            CmdTypes::Run { file_path: _ } => self.handle_run(),
            CmdTypes::Update { file_path: _ } => self.handle_update(),
        }
    }

    fn handle_run(&self) {
        let CmdTypes::Run { file_path } = self else {
            unreachable!();
        };

        let file_path = file_path.as_deref().unwrap_or(DEFAULT_YAML_LOCATION);

        let content = std::fs::read_to_string(&file_path)
            .expect(&format!("Failed to read file: {}", file_path));

        let serde_content: Result<Vec<Task>> = serde_yaml::from_str(&content);

        println!("serde_content: {serde_content:#?}");
    }

    fn handle_update(&self) {}
}

fn main() {
    CliCommands::parse().cmd.handle_cmd();
}
