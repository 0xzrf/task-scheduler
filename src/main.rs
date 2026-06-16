use chrono::{Local, NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_yaml::Result;
use std::collections::HashMap;
use tokio::{process::Command, time::Interval};

macro_rules! get_interval {
    () => {
        tokio::time::interval(std::time::Duration::from_secs(POOLING_INTERVAL_IN_SEC))
    };
    ($secs:expr) => {
        tokio::time::interval(std::time::Duration::from_secs($secs))
    };
}

struct SchedulerState {
    tasks: Vec<Task>,
    last_run: HashMap<u8, NaiveDate>,
    target_config: String, // path to the config
}

impl SchedulerState {
    pub fn new(tasks: Vec<Task>, target_config: String) -> Self {
        assert_ne!(tasks.len(), 0); // shouldn't have 0 tasks
        Self {
            tasks,
            last_run: HashMap::new(),
            target_config,
        }
    }

    pub fn is_first_scheduled_run(&self) -> bool {
        self.last_run.is_empty()
    }
}

const POOLING_INTERVAL_IN_SEC: u64 = 60;

const DEFAULT_YAML_LOCATION: &str = "~/.config/task_scheduler/tasks.yml";
const DEFAULT_ERR_OUT_LOCATION: &str = "~/.config/task_scheduler/error.txt";

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
        let tasks = load_tasks(file_path);

        let schedule_state = SchedulerState::new(tasks, file_path.to_string());

        let mut interval = get_interval!();

        execute_tasks(&schedule_state, interval).await;
    }

    async fn handle_update(&self) {}
}

async fn execute_tasks(schedule_state: &SchedulerState, interval: Interval) {
    let tasks = &schedule_state.tasks;
    loop {
        interval.tick();
        for (task_id, task) in tasks.iter().enumerate() {
            let task_exec_path = task.exec_path.as_str();
            let status = Command::new(task_exec_path).status().await?;

            if !status.success() {
                let msg = format!("script failed to run: {task_exec_path}");
                tracing::error!(&msg);
                error(&msg).await;
            }
        }
    }
}

async fn error(msg: &str) {
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;

    // You may need to define this constant elsewhere if not already defined.
    // const DEFAULT_ERR_OUT_LOCATION: &str = "error.log";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEFAULT_ERR_OUT_LOCATION)
        .await
        .expect("failed to open error file");

    file.write_all(msg.as_bytes())
        .await
        .expect("failed to write error message");
    file.write_all(b"\n")
        .await
        .expect("failed to write newline");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    CliCommands::parse().cmd.handle_cmd();
}

fn load_tasks(file_path: &str) -> Vec<Task> {
    let content =
        std::fs::read_to_string(&file_path).expect(&format!("Failed to read file: {}", file_path));

    match serde_yaml::from_str(&content) {
        Ok(tasks) => return tasks,
        Err(e) => {
            tracing::error!("Failed to parse YAML: {}", e);
            std::process::exit(1);
        }
    };
}

fn parse_hr(s: &str) -> NaiveTime {
    NaiveTime::parse_from_str(s, "%H:%M").expect("invalid time; use HH:MM")
}
