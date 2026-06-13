use chrono::{Local, NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_yaml::Result;
use std::collections::HashMap;

struct SchedulerState {
    tasks: Vec<Task>,
    last_run: HashMap<String, NaiveDate>,
}

const POOLING_INTERVAL_IN_SEC: u64 = 60;

const DEFAULT_YAML_LOCATION: &str = "~/.config/task_scheduler/tasks.yml";

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
}

impl Task {
    fn is_in_window(&self, now: NaiveTime) -> bool {
        let from = parse_hr(&self.from_hr);
        let to = parse_hr(&self.to_hr);
        now >= from && now <= to
    }
}

#[derive(Subcommand, Clone)]
pub enum CmdTypes {
    Run { file_path: Option<String> },
    Update { file_path: String },
}

impl CmdTypes {
    async fn handle_cmd(&self) {
        match self {
            CmdTypes::Run { file_path: _ } => self.handle_run().await,
            CmdTypes::Update { file_path: _ } => self.handle_update().await,
        }
    }

    async fn handle_run(&self) {
        let CmdTypes::Run { file_path } = self else {
            unreachable!();
        };

        let file_path = file_path.as_deref().unwrap_or(DEFAULT_YAML_LOCATION);

        let content = std::fs::read_to_string(&file_path)
            .expect(&format!("Failed to read file: {}", file_path));

        let serde_content: Vec<Task> = match serde_yaml::from_str(&content) {
            Ok(tasks) => tasks,
            Err(e) => {
                eprintln!("Failed to parse YAML: {}", e);
                return;
            }
        };
    }

    async fn handle_update(&self) {}
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    CliCommands::parse().cmd.handle_cmd();
}

fn parse_hr(s: &str) -> NaiveTime {
    NaiveTime::parse_from_str(s, "%H:%M").expect("invalid time; use HH:MM")
}
