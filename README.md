# rusty

A small **daily** terminal to-do app written in Rust. You get a warm-themed [Ratatui](https://github.com/ratatui-org/ratatui) checklist for today’s tasks, plus a JSON file on disk that **resets when the local calendar day changes** (or when you clear it yourself).

## Features

- **First run of the day** (or after a reset): type tasks line by line; enter `done` or `finish` to save and open the UI.
- **TUI**: move with **j** / **k** or arrow keys, **Space** / **Enter** toggles done, **q** quits (state is saved).
- **In the UI**: **a** adds a task after the selection, **d** deletes the selected task, **p** moves the selected task to the top.
- **CLI**: `rusty add "…"`, `rusty delete <id>` (alias `rusty rm <id>`).
- **Manual reset**: `rusty --reset`, `rusty -r`, `rusty -reset` (normalized to `--reset`), or `rusty reset` — clears today’s list. You can combine with `add`, e.g. `rusty -reset add "one thing"`.

Run `rusty -h` for built-in help.

## Build and run

Requires a stable Rust toolchain.

```bash
cargo build --release
./target/release/rusty
```

Install on your PATH:

```bash
cargo install --path .
```

## Where state is stored

Tasks live in `state.json` under the app’s **local data** directory from the [`directories`](https://crates.io/crates/directories) crate (`ProjectDirs` for `com.rusty.rusty`). Typical locations:

- **macOS**: `~/Library/Application Support/com.rusty.rusty/state.json`
- **Linux**: `~/.local/share/rusty/state.json` (or `$XDG_DATA_HOME/rusty/state.json`)
- **Windows**: `%LOCALAPPDATA%\rusty\rusty\data\state.json` (pattern from the [`directories`](https://docs.rs/directories) crate)

The file records today’s date and the task list; if the date is not **today** when the app loads, the list is cleared and the file is rewritten for a fresh day.

## License

No license is set in this repository by default; add one if you publish or share the project.
