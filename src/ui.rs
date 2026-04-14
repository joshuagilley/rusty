use crate::state::{AppState, Task};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

enum UiMode {
    List,
    AddTask { insert_at: usize, buffer: String },
}

pub fn run_ui(
    terminal: &mut DefaultTerminal,
    state: &mut AppState,
    state_path: &std::path::PathBuf,
    mimic: bool,
) -> Result<()> {
    let mut list_state = ListState::default();
    if !state.tasks.is_empty() {
        list_state.select(Some(0));
    }

    let mut mode = UiMode::List;

    loop {
        terminal.draw(|f| render(f, state, &mut list_state, &mode, mimic))?;

        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match &mut mode {
                UiMode::AddTask { insert_at, buffer } => match key.code {
                    KeyCode::Esc => {
                        mode = UiMode::List;
                    }
                    KeyCode::Enter => {
                        let title = buffer.trim();
                        if !title.is_empty() {
                            let id = state.next_id();
                            let pos = (*insert_at).min(state.tasks.len());
                            state.tasks.insert(
                                pos,
                                Task {
                                    id,
                                    title: title.to_string(),
                                    done: false,
                                    prioritized: false,
                                },
                            );
                            state.renumber_ids();
                            state.persist_to_disk(state_path, mimic)?;
                            list_state.select(Some(pos));
                        }
                        mode = UiMode::List;
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        buffer.push(c);
                    }
                    _ => {}
                },
                UiMode::List => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        state.persist_to_disk(state_path, mimic)?;
                        break;
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        let insert_at = list_state
                            .selected()
                            .map(|i| i + 1)
                            .unwrap_or(0)
                            .min(state.tasks.len());
                        mode = UiMode::AddTask {
                            insert_at,
                            buffer: String::new(),
                        };
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(i) = list_state.selected() {
                            if state.tasks.get(i).is_some() {
                                state.tasks.remove(i);
                                state.renumber_ids();
                                state.persist_to_disk(state_path, mimic)?;
                                if state.tasks.is_empty() {
                                    list_state.select(None);
                                } else {
                                    let next = i.min(state.tasks.len().saturating_sub(1));
                                    list_state.select(Some(next));
                                }
                            }
                        }
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        if let Some(i) = list_state.selected() {
                            if i < state.tasks.len() {
                                if i > 0 {
                                    let task = state.tasks.remove(i);
                                    state.tasks.insert(0, task);
                                }
                                for t in &mut state.tasks {
                                    t.prioritized = false;
                                }
                                if let Some(head) = state.tasks.first_mut() {
                                    head.prioritized = true;
                                }
                                state.renumber_ids();
                                state.persist_to_disk(state_path, mimic)?;
                                list_state.select(Some(0));
                            }
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if let Some(i) = list_state.selected() {
                            if let Some(t) = state.tasks.get_mut(i) {
                                t.done = !t.done;
                                state.persist_to_disk(state_path, mimic)?;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !state.tasks.is_empty() {
                            let i = list_state.selected().unwrap_or(0);
                            let next = (i + 1).min(state.tasks.len().saturating_sub(1));
                            list_state.select(Some(next));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !state.tasks.is_empty() {
                            let i = list_state.selected().unwrap_or(0);
                            let next = i.saturating_sub(1);
                            list_state.select(Some(next));
                        }
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, state: &AppState, list_state: &mut ListState, mode: &UiMode, mimic: bool) {
    let area = frame.area();

    let header_style = Style::default()
        .fg(Color::Rgb(222, 165, 132))
        .add_modifier(Modifier::BOLD);
    let accent = Color::Rgb(204, 120, 50);
    let muted = Color::Rgb(139, 125, 107);
    let done_fg = Color::Rgb(106, 153, 85);

    let title = if mimic {
        Line::from(vec![
            Span::styled("rusty", header_style),
            Span::styled(" — today's forge", Style::default().fg(muted)),
            Span::styled("  [mimic]", Style::default().fg(accent).add_modifier(Modifier::DIM)),
        ])
    } else {
        Line::from(vec![
            Span::styled("rusty", header_style),
            Span::styled(" — today's forge", Style::default().fg(muted)),
        ])
    };

    let date_line = Line::from(vec![
        Span::styled("date ", Style::default().fg(muted)),
        Span::styled(&state.date, Style::default().fg(accent).bold()),
    ]);

    let help = Line::from(vec![
        Span::styled("j/↓k/↑ ", Style::default().fg(accent)),
        Span::styled("move ", Style::default().fg(muted)),
        Span::styled("space ", Style::default().fg(accent)),
        Span::styled("toggle  ", Style::default().fg(muted)),
        Span::styled("a ", Style::default().fg(accent)),
        Span::styled("add after  ", Style::default().fg(muted)),
        Span::styled("d ", Style::default().fg(accent)),
        Span::styled("del  ", Style::default().fg(muted)),
        Span::styled("p ", Style::default().fg(accent)),
        Span::styled("prioritize  ", Style::default().fg(muted)),
        Span::styled("q ", Style::default().fg(accent)),
        Span::styled("quit", Style::default().fg(muted)),
    ]);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title_bottom(help);

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let footer_h = if matches!(mode, UiMode::AddTask { .. }) {
        2
    } else if mimic {
        2
    } else {
        1
    };

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(footer_h),
    ])
    .split(inner);

    frame.render_widget(Paragraph::new(title), chunks[0]);
    frame.render_widget(Paragraph::new(date_line), chunks[1]);

    let items: Vec<ListItem> = state
        .tasks
        .iter()
        .map(|t| task_row(t, accent, muted, done_fg))
        .collect();

    let list_title = if state.tasks.is_empty() {
        " tasks · a adds first "
    } else {
        " tasks "
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 90, 80)))
                .title(Span::styled(list_title, Style::default().fg(accent))),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 50, 45))
                .add_modifier(Modifier::BOLD),
        );

    let list_area = chunks[2].inner(Margin::new(0, 0));
    frame.render_stateful_widget(list, list_area, list_state);

    match mode {
        UiMode::List => {
            if mimic {
                let lines = vec![
                    Line::from(Span::styled(
                        "mimic — state.json is not written (disk unchanged)",
                        Style::default().fg(accent).add_modifier(Modifier::ITALIC),
                    )),
                    Line::from(vec![Span::styled(
                        "CLI: rusty add \"…\" · rusty delete <id> · rusty --reset (-r) · rusty reset",
                        Style::default().fg(muted),
                    )]),
                ];
                frame.render_widget(Paragraph::new(lines), chunks[3]);
            } else {
                let footer = Line::from(vec![Span::styled(
                    "CLI: rusty add \"…\" · rusty delete <id> · rusty --reset (-r) · rusty reset",
                    Style::default().fg(muted),
                )]);
                frame.render_widget(Paragraph::new(footer), chunks[3]);
            }
        }
        UiMode::AddTask { buffer, .. } => {
            let hint = Line::from(vec![
                Span::styled("new task ", Style::default().fg(accent)),
                Span::styled("(Enter save · Esc cancel)", Style::default().fg(muted)),
            ]);
            let input_line = Line::from(vec![
                Span::styled("> ", Style::default().fg(accent)),
                Span::styled(buffer.as_str(), Style::default().fg(Color::Rgb(230, 220, 210))),
                Span::styled("\u{258c}", Style::default().fg(accent)),
            ]);
            let footer_area = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(chunks[3]);
            frame.render_widget(Paragraph::new(hint), footer_area[0]);
            frame.render_widget(Paragraph::new(input_line), footer_area[1]);
        }
    }
}

fn task_row(task: &Task, accent: Color, muted: Color, done_fg: Color) -> ListItem<'static> {
    let mark = if task.done { "[x]" } else { "[ ]" };
    let rust_bold = Style::default()
        .fg(accent)
        .add_modifier(Modifier::BOLD);

    let (mark_style, id_style, text_style) = if task.prioritized && !task.done {
        (
            rust_bold,
            rust_bold,
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD),
        )
    } else if task.done {
        let mark_style = Style::default().fg(done_fg);
        let text = Style::default()
            .fg(done_fg)
            .add_modifier(Modifier::DIM | Modifier::CROSSED_OUT);
        (mark_style, Style::default().fg(muted), text)
    } else {
        (
            Style::default().fg(accent),
            Style::default().fg(muted),
            Style::default().fg(Color::Rgb(230, 220, 210)),
        )
    };

    let line = Line::from(vec![
        Span::styled(format!("{} ", mark), mark_style),
        Span::styled(format!("#{} ", task.id), id_style),
        Span::styled(task.title.clone(), text_style),
    ]);
    ListItem::new(line)
}
