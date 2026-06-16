use chrono::{Local, NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{process::Command, time::Interval};

const POOLING_INTERVAL_IN_SEC: u64 = 60;
const DEFAULT_YAML_LOCATION: &str = "~/.config/task_scheduler/tasks.yml";
const DEFAULT_ERR_OUT_LOCATION: &str = "~/.config/task_scheduler/error.log";

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
    last_run: HashMap<usize, NaiveDate>,
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

    pub fn is_task_done_for_today(&self, task_id: usize) -> bool {
        let today = Local::now().naive_local().date();
        match self.last_run.get(&task_id) {
            Some(last_run_date) => *last_run_date == today,
            None => false,
        }
    }
}

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
    fn is_in_window(&self) -> bool {
        let now = Local::now().time();
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

        let mut schedule_state = SchedulerState::new(tasks, file_path.to_string());

        let interval = get_interval!();

        execute_tasks(&mut schedule_state, interval).await;
    }

    async fn handle_update(&self) {}
}

async fn execute_tasks(schedule_state: &mut SchedulerState, mut interval: Interval) {
    let tasks = &schedule_state.tasks;
    loop {
        interval.tick().await;
        for (task_id, task) in tasks.iter().enumerate() {
            if !task.is_in_window() || schedule_state.is_task_done_for_today(task_id) {
                println!("skipping task: {}", task.exec_path);
                continue;
            }

            let task_exec_path = task.exec_path.as_str();
            let status = Command::new(task_exec_path).status().await.unwrap();

            if !status.success() {
                let msg = format!("script failed to run: {task_exec_path}");
                tracing::error!("{}", &msg);
                error(&msg).await;
                continue;
            }

            // put it as done for the day
            schedule_state
                .last_run
                .insert(task_id, Local::now().naive_local().date());
        }
    }
}

async fn error(msg: &str) {
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;

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
    CliCommands::parse().cmd.handle_cmd().await;
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
