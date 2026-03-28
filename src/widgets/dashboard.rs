use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::data::VerseCollection;

pub struct DashboardWidget {
    pub selected: usize,
    pub scroll_offset: usize,
}

impl DashboardWidget {
    pub fn new(_collection: &VerseCollection) -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, collection: &VerseCollection, today: NaiveDate) {
        let area = frame.area();
        let due_count = collection.due_count(today);
        let total = collection.verses.len();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Stats bar
                Constraint::Min(0),    // Verse list
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Header
        let header = Paragraph::new("Bible Verse Memory")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(header, chunks[0]);

        // Stats bar
        let stats_text = format!(
            " {} verse{} total  |  {} due for review",
            total,
            if total == 1 { "" } else { "s" },
            due_count,
        );
        let stats_style = if due_count > 0 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let stats = Paragraph::new(stats_text)
            .style(stats_style)
            .block(Block::default().borders(Borders::ALL).title("Summary"));
        frame.render_widget(stats, chunks[1]);

        // Verse list
        if collection.verses.is_empty() {
            let empty = Paragraph::new("  No verses yet. Press 'a' to add your first verse.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Memory Verses"),
                );
            frame.render_widget(empty, chunks[2]);
        } else {
            let list_area = chunks[2];
            let inner_height = list_area.height.saturating_sub(2) as usize; // borders
            let visible_rows = inner_height / 2; // each verse takes 2 lines

            // Header row
            let header_row = Row::new(vec![
                Cell::from(" Status").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Reference").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Level").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Next Review").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Reviews").style(Style::default().add_modifier(Modifier::BOLD)),
            ])
            .height(1);

            let rows: Vec<Row> = collection
                .verses
                .iter()
                .enumerate()
                .map(|(i, verse)| {
                    let status = verse.status_label(today);
                    let status_style = match status {
                        "Due" | "New" => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        "Learning" => Style::default().fg(Color::LightBlue),
                        "Reviewing" => Style::default().fg(Color::LightGreen),
                        "Mastered" => Style::default().fg(Color::Green),
                        _ => Style::default(),
                    };

                    let next_review = if verse.last_reviewed.is_none() {
                        "Now".to_string()
                    } else {
                        let days = verse.days_until_due(today);
                        if days <= 0 {
                            "Now".to_string()
                        } else if days == 1 {
                            "Tomorrow".to_string()
                        } else {
                            format!("In {} days", days)
                        }
                    };

                    let level_display = format!("{} ({}d)", verse.level, verse.interval_days());

                    let row_style = if i == self.selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(format!(" {}", status)).style(status_style),
                        Cell::from(verse.reference.clone()),
                        Cell::from(level_display),
                        Cell::from(next_review),
                        Cell::from(format!("{}", verse.review_count)),
                    ])
                    .style(row_style)
                    .height(1)
                })
                .collect();

            let widths = [
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(10),
                Constraint::Length(14),
                Constraint::Length(9),
            ];

            let table = Table::new(rows, widths)
                .header(header_row)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Memory Verses"),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 60))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            let mut table_state = TableState::default();
            if !collection.verses.is_empty() {
                table_state.select(Some(self.selected));
                // Scroll into view
                if self.selected >= self.scroll_offset + visible_rows {
                    table_state.scroll_down_by(
                        (self.selected - self.scroll_offset - visible_rows + 1) as u16,
                    );
                }
            }

            frame.render_stateful_widget(table, list_area, &mut table_state);
        }

        // Footer
        let footer_text = if collection.verses.is_empty() {
            "a: Add Verse | q: Quit"
        } else {
            "a: Add | Enter: Review | r: Review All Due | e: Edit | d: Delete | q: Quit"
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[3]);
    }

    pub fn handle_key(&mut self, key: KeyEvent, collection: &VerseCollection) -> DashboardAction {
        let len = collection.verses.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => DashboardAction::Quit,
            KeyCode::Char('a') => DashboardAction::AddVerse,
            KeyCode::Char('r') => DashboardAction::StartReview,
            KeyCode::Enter => {
                if len > 0 {
                    DashboardAction::ReviewVerse(self.selected)
                } else {
                    DashboardAction::None
                }
            }
            KeyCode::Char('e') => {
                if len > 0 {
                    DashboardAction::EditVerse(self.selected)
                } else {
                    DashboardAction::None
                }
            }
            KeyCode::Char('d') => {
                if len > 0 {
                    DashboardAction::ConfirmDelete(self.selected)
                } else {
                    DashboardAction::None
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if len > 0 {
                    self.selected = self.selected.saturating_sub(1);
                }
                DashboardAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 {
                    self.selected = (self.selected + 1).min(len - 1);
                }
                DashboardAction::None
            }
            KeyCode::Home => {
                self.selected = 0;
                DashboardAction::None
            }
            KeyCode::End => {
                if len > 0 {
                    self.selected = len - 1;
                }
                DashboardAction::None
            }
            _ => DashboardAction::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardAction {
    None,
    Quit,
    AddVerse,
    StartReview,
    ReviewVerse(usize),
    EditVerse(usize),
    ConfirmDelete(usize),
}
