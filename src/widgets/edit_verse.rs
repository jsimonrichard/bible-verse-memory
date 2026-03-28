use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};

use crate::data::MemoryVerse;
use crate::input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFocus {
    Reference,
    Text,
}

pub struct EditVerseWidget {
    pub index: usize,
    pub reference: String,
    pub text: String,
    focus: InputFocus,
    pub error_message: Option<String>,
    cursor_pos_ref: usize,
    cursor_pos_text: usize,
}

impl EditVerseWidget {
    pub fn new(index: usize, verse: &MemoryVerse) -> Self {
        let ref_len = verse.reference.len();
        let text_len = verse.text.len();
        Self {
            index,
            reference: verse.reference.clone(),
            text: verse.text.clone(),
            focus: InputFocus::Reference,
            error_message: None,
            cursor_pos_ref: ref_len,
            cursor_pos_text: text_len,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Reference input
                Constraint::Min(5),    // Text input
                Constraint::Length(3), // Error / Help
                Constraint::Length(3), // Footer
            ])
            .split(area);

        let header = Paragraph::new("Edit Memory Verse")
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

        let ref_style = if self.focus == InputFocus::Reference {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        let ref_block = Block::default()
            .borders(Borders::ALL)
            .title("Reference")
            .border_style(ref_style);
        let ref_text = Paragraph::new(self.reference.as_str()).block(ref_block);
        frame.render_widget(ref_text, chunks[1]);

        if self.focus == InputFocus::Reference {
            let ref_display_width = input::display_width(&self.reference[..self.cursor_pos_ref]);
            frame.set_cursor_position(Position::new(
                chunks[1].x + ref_display_width as u16 + 1,
                chunks[1].y + 1,
            ));
        }

        let text_style = if self.focus == InputFocus::Text {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        let text_block = Block::default()
            .borders(Borders::ALL)
            .title("Verse Text")
            .border_style(text_style);

        let inner_width = chunks[2].width.saturating_sub(2) as usize;
        let wrapped = input::wrap_text(&self.text, inner_width);
        let text_paragraph = Paragraph::new(wrapped).block(text_block);
        frame.render_widget(text_paragraph, chunks[2]);

        if self.focus == InputFocus::Text {
            let (cx, cy) =
                input::cursor_position_in_wrapped(&self.text, self.cursor_pos_text, inner_width);
            frame.set_cursor_position(Position::new(
                chunks[2].x + cx as u16 + 1,
                chunks[2].y + cy as u16 + 1,
            ));
        }

        if let Some(err) = &self.error_message {
            let error = Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error, chunks[3]);
        } else {
            let help = Paragraph::new(" Tab: switch field | Ctrl+S: save | Esc: cancel")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(help, chunks[3]);
        }

        let footer = Paragraph::new("Ctrl+S: Save | Esc: Cancel | Tab: Switch Field")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[4]);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> EditVerseAction {
        self.error_message = None;

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => return EditVerseAction::Cancel,
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => return EditVerseAction::Save,
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => match self.focus {
                InputFocus::Reference => {
                    let start = 0;
                    self.reference.drain(start..self.cursor_pos_ref);
                    self.cursor_pos_ref = start;
                }
                InputFocus::Text => {
                    let start = input::line_start(&self.text, self.cursor_pos_text);
                    self.text.drain(start..self.cursor_pos_text);
                    self.cursor_pos_text = start;
                }
            },
            (_, KeyCode::Tab) | (_, KeyCode::BackTab) => {
                self.focus = match self.focus {
                    InputFocus::Reference => InputFocus::Text,
                    InputFocus::Text => InputFocus::Reference,
                };
            }
            (m, KeyCode::Char(c)) if !m.contains(KeyModifiers::CONTROL) => match self.focus {
                InputFocus::Reference => {
                    self.reference.insert(self.cursor_pos_ref, c);
                    self.cursor_pos_ref += c.len_utf8();
                }
                InputFocus::Text => {
                    self.text.insert(self.cursor_pos_text, c);
                    self.cursor_pos_text += c.len_utf8();
                }
            },
            (_, KeyCode::Backspace) => match self.focus {
                InputFocus::Reference => {
                    if self.cursor_pos_ref > 0 {
                        let prev = input::prev_char_boundary(&self.reference, self.cursor_pos_ref);
                        self.reference.drain(prev..self.cursor_pos_ref);
                        self.cursor_pos_ref = prev;
                    }
                }
                InputFocus::Text => {
                    if self.cursor_pos_text > 0 {
                        let prev = input::prev_char_boundary(&self.text, self.cursor_pos_text);
                        self.text.drain(prev..self.cursor_pos_text);
                        self.cursor_pos_text = prev;
                    }
                }
            },
            (_, KeyCode::Delete) => match self.focus {
                InputFocus::Reference => {
                    if self.cursor_pos_ref < self.reference.len() {
                        let next = input::next_char_boundary(&self.reference, self.cursor_pos_ref);
                        self.reference.drain(self.cursor_pos_ref..next);
                    }
                }
                InputFocus::Text => {
                    if self.cursor_pos_text < self.text.len() {
                        let next = input::next_char_boundary(&self.text, self.cursor_pos_text);
                        self.text.drain(self.cursor_pos_text..next);
                    }
                }
            },
            (KeyModifiers::CONTROL, KeyCode::Left) => match self.focus {
                InputFocus::Reference => {
                    self.cursor_pos_ref =
                        input::prev_word_boundary(&self.reference, self.cursor_pos_ref);
                }
                InputFocus::Text => {
                    self.cursor_pos_text =
                        input::prev_word_boundary(&self.text, self.cursor_pos_text);
                }
            },
            (KeyModifiers::CONTROL, KeyCode::Right) => match self.focus {
                InputFocus::Reference => {
                    self.cursor_pos_ref =
                        input::next_word_boundary(&self.reference, self.cursor_pos_ref);
                }
                InputFocus::Text => {
                    self.cursor_pos_text =
                        input::next_word_boundary(&self.text, self.cursor_pos_text);
                }
            },
            (_, KeyCode::Left) => match self.focus {
                InputFocus::Reference => {
                    self.cursor_pos_ref =
                        input::prev_char_boundary(&self.reference, self.cursor_pos_ref);
                }
                InputFocus::Text => {
                    self.cursor_pos_text =
                        input::prev_char_boundary(&self.text, self.cursor_pos_text);
                }
            },
            (_, KeyCode::Right) => match self.focus {
                InputFocus::Reference => {
                    self.cursor_pos_ref =
                        input::next_char_boundary(&self.reference, self.cursor_pos_ref);
                }
                InputFocus::Text => {
                    self.cursor_pos_text =
                        input::next_char_boundary(&self.text, self.cursor_pos_text);
                }
            },
            (_, KeyCode::Home) => match self.focus {
                InputFocus::Reference => self.cursor_pos_ref = 0,
                InputFocus::Text => self.cursor_pos_text = 0,
            },
            (_, KeyCode::End) => match self.focus {
                InputFocus::Reference => self.cursor_pos_ref = self.reference.len(),
                InputFocus::Text => self.cursor_pos_text = self.text.len(),
            },
            (_, KeyCode::Enter) => {
                if self.focus == InputFocus::Text {
                    self.text.insert(self.cursor_pos_text, '\n');
                    self.cursor_pos_text += 1;
                } else {
                    self.focus = InputFocus::Text;
                }
            }
            _ => {}
        }

        EditVerseAction::None
    }

    pub fn handle_paste(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        match self.focus {
            InputFocus::Reference => {
                let clean = normalized.replace('\n', " ");
                self.reference.insert_str(self.cursor_pos_ref, &clean);
                self.cursor_pos_ref += clean.len();
            }
            InputFocus::Text => {
                self.text.insert_str(self.cursor_pos_text, &normalized);
                self.cursor_pos_text += normalized.len();
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.reference.trim().is_empty() {
            return Err("Reference cannot be empty".to_string());
        }
        if self.text.trim().is_empty() {
            return Err("Verse text cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditVerseAction {
    None,
    Cancel,
    Save,
}
