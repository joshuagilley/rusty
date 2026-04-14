//! Rollover: completion bar, counts, incomplete checklist, Y/N — one screen.

use crate::state::{AppState, Task};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

const ACCENT: Color = Color::Rgb(204, 120, 50);
const MUTED: Color = Color::Rgb(139, 125, 107);
const PANEL: Color = Color::Rgb(60, 52, 46);
const NEEDLE: Color = Color::Rgb(222, 165, 132);

/// `mimic_preview`: `--ratatui` titles and dry-run footer; caller handles persistence.
pub fn run_rollover_flow(
    terminal: &mut DefaultTerminal,
    yesterday: &AppState,
    mimic_preview: bool,
) -> Result<Vec<Task>> {
    let total = yesterday.tasks.len();
    let done_n = yesterday.tasks.iter().filter(|t| t.done).count();
    let open_n = total.saturating_sub(done_n);
    let pct: u16 = if total > 0 {
        ((done_n as u64 * 100) / total as u64) as u16
    } else {
        0
    };

    let titles: Vec<String> = yesterday
        .tasks
        .iter()
        .filter(|t| !t.done)
        .map(|t| t.title.clone())
        .collect();

    let mut selected = vec![false; titles.len()];
    let mut list_state = ListState::default();
    if !titles.is_empty() {
        list_state.select(Some(0));
    }

    loop {
        terminal.draw(|f| {
            render_rollover(
                f,
                yesterday,
                total,
                done_n,
                open_n,
                pct,
                mimic_preview,
                &titles,
                &selected,
                &mut list_state,
            )
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let out: Vec<Task> = titles
                    .iter()
                    .zip(selected.iter())
                    .filter(|(_, s)| **s)
                    .map(|(title, _)| Task {
                        id: 0,
                        title: title.clone(),
                        done: false,
                        prioritized: false,
                    })
                    .collect();
                return Ok(out);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(Vec::new()),
            KeyCode::Char(' ') => {
                if let Some(i) = list_state.selected() {
                    if let Some(s) = selected.get_mut(i) {
                        *s = !*s;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !titles.is_empty() {
                    let i = list_state.selected().unwrap_or(0);
                    list_state.select(Some((i + 1).min(titles.len() - 1)));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !titles.is_empty() {
                    let i = list_state.selected().unwrap_or(0);
                    list_state.select(Some(i.saturating_sub(1)));
                }
            }
            _ => {}
        }
    }
}

fn render_rollover(
    frame: &mut Frame,
    yesterday: &AppState,
    total: usize,
    done_n: usize,
    open_n: usize,
    pct: u16,
    mimic_preview: bool,
    titles: &[String],
    selected: &[bool],
    list_state: &mut ListState,
) {
    let area = frame.area();
    let title_bar = if mimic_preview {
        format!(" preview · saved {} ", yesterday.date)
    } else {
        format!(" yesterday · {} ", yesterday.date)
    };
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(
            title_bar,
            Style::default().fg(NEEDLE).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(analog_h_bar(pct, (inner.width.saturating_sub(4)) as usize))
            .alignment(Alignment::Center),
        rows[0],
    );

    let metrics = Line::from(vec![
        Span::styled("tasks ", Style::default().fg(MUTED)),
        Span::styled(format!("{total}"), Style::default().fg(NEEDLE).bold()),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled("closed ", Style::default().fg(MUTED)),
        Span::styled(format!("{done_n}"), Style::default().fg(ACCENT).bold()),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled("open ", Style::default().fg(MUTED)),
        Span::styled(format!("{open_n}"), Style::default().fg(NEEDLE).bold()),
    ]);
    frame.render_widget(
        Paragraph::new(metrics).alignment(Alignment::Center),
        rows[1],
    );

    let list_area = rows[2];
    if titles.is_empty() {
        let msg = Line::from(Span::styled(
            "Nothing left open — Y to continue · N / Esc to start fresh",
            Style::default().fg(MUTED),
        ));
        frame.render_widget(
            Paragraph::new(msg)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Rgb(100, 90, 80))),
                ),
            list_area,
        );
    } else {
        let items: Vec<ListItem> = titles
            .iter()
            .enumerate()
            .map(|(i, title)| {
                let mark = if selected.get(i).copied().unwrap_or(false) {
                    "[×]"
                } else {
                    "[ ]"
                };
                let line = Line::from(vec![
                    Span::styled(format!("{mark} "), Style::default().fg(ACCENT).bold()),
                    Span::styled(title.clone(), Style::default().fg(Color::Rgb(230, 220, 210))),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 90, 80))),
            );

        frame.render_stateful_widget(list, list_area, list_state);
    }

    let telem = if mimic_preview {
        "dry-run · disk unchanged"
    } else {
        "rusty forge"
    };
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Y", Style::default().fg(ACCENT).bold()),
            Span::styled(" checked  ", Style::default().fg(MUTED)),
            Span::styled("N", Style::default().fg(ACCENT).bold()),
            Span::styled(" / Esc none  ", Style::default().fg(MUTED)),
            Span::styled("· Space row  ", Style::default().fg(MUTED)),
            Span::styled("· j/k  ", Style::default().fg(MUTED)),
            Span::styled("· ", Style::default().fg(MUTED)),
            Span::styled(telem, Style::default().fg(Color::Rgb(100, 90, 80))),
        ]),
        Line::from(""),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(footer, rows[3]);
}

fn analog_h_bar(pct: u16, width: usize) -> Line<'static> {
    let w = width.max(10).min(60);
    let fill = (pct as usize * w / 100).min(w);
    let mut s = String::with_capacity(w + 4);
    s.push('╭');
    for i in 0..w {
        if i < fill {
            s.push('█');
        } else {
            s.push('░');
        }
    }
    s.push('╯');
    Line::from(vec![
        Span::styled(s, Style::default().fg(ACCENT)),
        Span::styled(
            format!(" {pct}%"),
            Style::default().fg(NEEDLE).add_modifier(Modifier::BOLD),
        ),
    ])
}
