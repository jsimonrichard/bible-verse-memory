use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::data::{ReviewGrade, VerseCollection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewPhase {
    ShowReference,
    ShowVerse,
}

pub struct ReviewWidget {
    /// Indices into the VerseCollection of due verses, in review order
    due_indices: Vec<usize>,
    /// Current position within due_indices
    current: usize,
    phase: ReviewPhase,
    pub finished: bool,
    reviewed_count: usize,
    good_count: usize,
    hard_count: usize,
    again_count: usize,
}

impl ReviewWidget {
    pub fn new(collection: &VerseCollection, today: NaiveDate) -> Self {
        let mut due_indices: Vec<usize> = collection
            .due_verses(today)
            .iter()
            .map(|(i, _)| *i)
            .collect();

        // Sort so most overdue come first
        due_indices.sort_by(|&a, &b| {
            let va = &collection.verses[a];
            let vb = &collection.verses[b];
            va.days_until_due(today).cmp(&vb.days_until_due(today))
        });

        let finished = due_indices.is_empty();

        Self {
            due_indices,
            current: 0,
            phase: ReviewPhase::ShowReference,
            finished,
            reviewed_count: 0,
            good_count: 0,
            hard_count: 0,
            again_count: 0,
        }
    }

    /// Review a single verse (regardless of whether it's due).
    pub fn new_single(index: usize) -> Self {
        Self {
            due_indices: vec![index],
            current: 0,
            phase: ReviewPhase::ShowReference,
            finished: false,
            reviewed_count: 0,
            good_count: 0,
            hard_count: 0,
            again_count: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, collection: &VerseCollection, _today: NaiveDate) {
        let area = frame.area();

        if self.finished {
            self.render_summary(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header / progress
                Constraint::Min(0),    // Card area
                Constraint::Length(3), // Footer
            ])
            .split(area);

        let total = self.due_indices.len();
        let progress_text = format!(
            "Review: {} / {}  (Good: {} | Hard: {} | Again: {})",
            self.current + 1,
            total,
            self.good_count,
            self.hard_count,
            self.again_count,
        );
        let header = Paragraph::new(progress_text)
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

        let verse_idx = self.due_indices[self.current];
        let verse = &collection.verses[verse_idx];

        // Card area
        let card_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Reference
                Constraint::Length(3), // Metadata
                Constraint::Min(0),    // Verse text or prompt
            ])
            .split(chunks[1]);

        // Reference (always shown)
        let reference = Paragraph::new(verse.reference.clone())
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Reference")
                    .border_style(Style::default().fg(Color::Yellow)),
            );
        frame.render_widget(reference, card_chunks[0]);

        // Metadata line
        let level_info = format!(
            "Level: {} | Interval: {}d | Reviews: {}",
            verse.level,
            verse.interval_days(),
            verse.review_count,
        );
        let meta = Paragraph::new(level_info)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(meta, card_chunks[1]);

        match self.phase {
            ReviewPhase::ShowReference => {
                let prompt = Paragraph::new("\n\n  Try to recall the verse from memory...\n\n  Press Space or Enter to reveal the verse text.")
                    .style(Style::default().fg(Color::Gray))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Recall")
                            .border_style(Style::default().fg(Color::DarkGray)),
                    )
                    .wrap(Wrap { trim: false });
                frame.render_widget(prompt, card_chunks[2]);
            }
            ReviewPhase::ShowVerse => {
                let text = Paragraph::new(verse.text.clone())
                    .style(Style::default().fg(Color::White))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Verse Text")
                            .border_style(Style::default().fg(Color::Green)),
                    )
                    .wrap(Wrap { trim: false });
                frame.render_widget(text, card_chunks[2]);
            }
        }

        // Footer
        let footer_text = match self.phase {
            ReviewPhase::ShowReference => "Space/Enter: Reveal | Esc: Cancel Review",
            ReviewPhase::ShowVerse => "g: Good | h: Hard | a: Again | Space: Hide | Esc: Cancel",
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        let header = Paragraph::new("Review Complete!")
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            );
        frame.render_widget(header, chunks[0]);

        let summary = if self.reviewed_count == 0 {
            "  No verses were due for review. Great job staying on top of it!".to_string()
        } else {
            let mut note = String::new();
            if self.good_count == self.reviewed_count {
                note.push_str("Perfect session! All verses recalled from memory.");
            } else {
                if self.hard_count > 0 {
                    note.push_str("'Hard' verses keep the same review interval. ");
                }
                if self.again_count > 0 {
                    note.push_str(
                        "'Again' verses drop one level for slightly more frequent review.",
                    );
                }
            }
            format!(
                "\n  Reviewed: {}\n  Good (recalled): {}\n  Hard (read it): {}\n  Again (lost): {}\n\n  {}",
                self.reviewed_count, self.good_count, self.hard_count, self.again_count, note,
            )
        };

        let body = Paragraph::new(summary)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Session Summary"),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(body, chunks[1]);

        let footer = Paragraph::new("Press any key to return to dashboard")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        collection: &mut VerseCollection,
        today: NaiveDate,
    ) -> ReviewAction {
        if self.finished {
            return ReviewAction::Done;
        }

        match self.phase {
            ReviewPhase::ShowReference => match key.code {
                KeyCode::Esc => ReviewAction::Cancel,
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.phase = ReviewPhase::ShowVerse;
                    ReviewAction::None
                }
                _ => ReviewAction::None,
            },
            ReviewPhase::ShowVerse => match key.code {
                KeyCode::Esc => ReviewAction::Cancel,
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.phase = ReviewPhase::ShowReference;
                    ReviewAction::None
                }
                KeyCode::Char('g') => {
                    let verse_idx = self.due_indices[self.current];
                    collection.mark_and_schedule(verse_idx, ReviewGrade::Good, today);
                    self.good_count += 1;
                    self.reviewed_count += 1;
                    self.advance();
                    ReviewAction::Save
                }
                KeyCode::Char('h') => {
                    let verse_idx = self.due_indices[self.current];
                    collection.mark_and_schedule(verse_idx, ReviewGrade::Hard, today);
                    self.hard_count += 1;
                    self.reviewed_count += 1;
                    self.advance();
                    ReviewAction::Save
                }
                KeyCode::Char('a') => {
                    let verse_idx = self.due_indices[self.current];
                    collection.mark_and_schedule(verse_idx, ReviewGrade::Again, today);
                    self.again_count += 1;
                    self.reviewed_count += 1;
                    self.advance();
                    ReviewAction::Save
                }
                _ => ReviewAction::None,
            },
        }
    }

    fn advance(&mut self) {
        self.current += 1;
        if self.current >= self.due_indices.len() {
            self.finished = true;
        } else {
            self.phase = ReviewPhase::ShowReference;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    None,
    Cancel,
    Done,
    /// A verse was graded; caller should save
    Save,
}
