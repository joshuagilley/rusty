mod rollover;
mod state;
mod ui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use directories::ProjectDirs;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;

use crate::state::{AppState, Task};

#[derive(Parser)]
#[command(name = "rusty")]
#[command(
    about = "Daily terminal todo list with a small TUI.\n\
             Tasks live in your user data directory as JSON. A new local calendar day runs a short recap before today’s list.",
    version
)]
struct Cli {
    /// Clear every task for today (unless combined with --ratatui)
    #[arg(long = "reset", short = 'r')]
    reset: bool,

    /// Mimic mode: no disk writes. Rollover preview only when the saved date is not today.
    #[arg(long = "ratatui")]
    ratatui: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clear today’s list and exit (same as --reset)
    Reset,
    /// Append a task for today
    Add {
        title: String,
    },
    /// Remove a task by id (as shown in the TUI)
    #[command(alias = "rm")]
    Delete {
        id: u64,
    },
}

fn state_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "rusty", "rusty")
        .context("could not resolve data directory; set HOME")?;
    Ok(dirs.data_local_dir().join("state.json"))
}

fn run_rollover_terminal(previous: &AppState, mimic_preview: bool) -> Result<Vec<Task>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let carried = rollover::run_rollover_flow(&mut terminal, previous, mimic_preview)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(carried)
}

fn apply_rollover_carried(carried: Vec<Task>) -> AppState {
    let mut state = AppState {
        date: state::today_string(),
        tasks: carried,
    };
    state.renumber_ids();
    state
}

fn run_tui(state: &mut AppState, path: &PathBuf, mimic: bool) -> Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let res = ui::run_ui(&mut terminal, state, path, mimic);
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
            } else if i > 0 && arg == "-ratatui" {
                arg = "--ratatui".to_string();
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
        if cli.ratatui {
            println!("--ratatui: not clearing disk (--reset ignored).");
        } else {
            AppState::empty_today().save(&path)?;
            println!("Cleared today's task list.");
        }
    }

    match &cli.command {
        Some(Commands::Reset) => return Ok(()),
        Some(Commands::Add { title }) => {
            let mut state = AppState::load_for_cli(&path)?;
            if title.trim().is_empty() {
                anyhow::bail!("task title cannot be empty");
            }
            let id = state.next_id();
            state.tasks.push(Task {
                id,
                title: title.trim().to_string(),
                done: false,
                prioritized: false,
            });
            state.renumber_ids();
            state.save(&path)?;
            println!("added task #{} — {}", id, title.trim());
        }
        Some(Commands::Delete { id }) => {
            let mut state = AppState::load_for_cli(&path)?;
            let before = state.tasks.len();
            state.tasks.retain(|t| t.id != *id);
            if state.tasks.len() == before {
                anyhow::bail!("no task with id {}", id);
            }
            state.renumber_ids();
            state.save(&path)?;
            println!("deleted task #{}", id);
        }
        None => {
            if reset_requested && !cli.ratatui {
                return Ok(());
            }
            let mimic = cli.ratatui;
            let today = state::today_string();

            let mut state = if mimic {
                let loaded = AppState::read_mimic(&path)?;
                if loaded.date != today {
                    let carried = run_rollover_terminal(&loaded, true)?;
                    apply_rollover_carried(carried)
                } else {
                    let mut s = loaded;
                    s.renumber_ids();
                    s
                }
            } else {
                match AppState::read_session_start(&path)? {
                    state::SessionStart::Fresh(s) | state::SessionStart::Today(s) => s,
                    state::SessionStart::NeedsRollover(previous) => {
                        let carried = run_rollover_terminal(&previous, false)?;
                        let new_state = apply_rollover_carried(carried);
                        new_state.save(&path)?;
                        new_state
                    }
                }
            };

            run_tui(&mut state, &path, mimic)?;
        }
    }

    Ok(())
}
