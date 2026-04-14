mod state;
mod ui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use directories::ProjectDirs;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout, Write};
use std::path::PathBuf;

use crate::state::{AppState, Task};

#[derive(Parser)]
#[command(name = "rusty")]
#[command(
    about = "Daily terminal todo list with a small TUI.\n\
             Tasks are stored in your user data directory as JSON; when the local calendar day changes, the file is reset to an empty list.",
    version
)]
struct Cli {
    /// Clear every task for today and write an empty list (then exit unless you also pass a subcommand)
    #[arg(long = "reset", short = 'r')]
    reset: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clear today's list and exit (same as --reset)
    Reset,
    /// Append a task for today
    Add {
        /// Task text
        title: String,
    },
    /// Remove a task by its id (see the list in the UI)
    #[command(alias = "rm")]
    Delete {
        /// Task id
        id: u64,
    },
}

fn state_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "rusty", "rusty")
        .context("could not resolve data directory; set HOME")?;
    Ok(dirs.data_local_dir().join("state.json"))
}

fn prompt_initial_tasks(state: &mut AppState, path: &PathBuf) -> Result<()> {
    println!();
    println!(" No tasks yet for today. Enter what you want to accomplish.");
    println!("  Type {} or {} on a line by itself when you're finished.\n", "done", "finish");

    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("  task: ");
        io::stdout().flush()?;
        line.clear();
        stdin.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower == "done" || lower == "finish" {
            break;
        }
        let id = state.next_id();
        state.tasks.push(Task {
            id,
            title: trimmed.to_string(),
            done: false,
        });
    }

    state.save(path)?;
    Ok(())
}

fn run_tui(state: &mut AppState, path: &PathBuf) -> Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let res = ui::run_ui(&mut terminal, state, path);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    res
}

fn normalize_argv(argv: Vec<String>) -> Vec<String> {
    argv.into_iter()
        .enumerate()
        .map(|(i, mut arg)| {
            if i > 0 && arg == "-reset" {
                arg = "--reset".to_string();
            }
            arg
        })
        .collect()
}

fn main() -> Result<()> {
    let cli = Cli::parse_from(normalize_argv(std::env::args().collect()));
    let path = state_path()?;

    let reset_requested = cli.reset || matches!(&cli.command, Some(Commands::Reset));
    if reset_requested {
        AppState::empty_today().save(&path)?;
        println!("Cleared today's task list.");
    }

    match &cli.command {
        Some(Commands::Reset) => {
            return Ok(());
        }
        Some(Commands::Add { title }) => {
            let mut state = AppState::load_or_reset(&path)?;
            if title.trim().is_empty() {
                anyhow::bail!("task title cannot be empty");
            }
            let id = state.next_id();
            state.tasks.push(Task {
                id,
                title: title.trim().to_string(),
                done: false,
            });
            state.save(&path)?;
            println!("added task #{} — {}", id, title.trim());
        }
        Some(Commands::Delete { id }) => {
            let mut state = AppState::load_or_reset(&path)?;
            let before = state.tasks.len();
            state.tasks.retain(|t| t.id != *id);
            if state.tasks.len() == before {
                anyhow::bail!("no task with id {}", id);
            }
            state.save(&path)?;
            println!("deleted task #{}", id);
        }
        None => {
            if reset_requested {
                return Ok(());
            }
            let mut state = AppState::load_or_reset(&path)?;
            if state.tasks.is_empty() {
                prompt_initial_tasks(&mut state, &path)?;
            }
            run_tui(&mut state, &path)?;
        }
    }

    Ok(())
}
