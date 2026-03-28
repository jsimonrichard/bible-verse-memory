use chrono::NaiveDate;
use clap::Parser;
use color_eyre::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind,
};
use ratatui::prelude::*;
use std::io;

use bible_verse_memory::config::{Config, load_verses, save_verses};
use bible_verse_memory::data::{MemoryVerse, VerseCollection};
use bible_verse_memory::widgets::add_verse::{AddVerseAction, AddVerseWidget};
use bible_verse_memory::widgets::dashboard::{DashboardAction, DashboardWidget};
use bible_verse_memory::widgets::edit_verse::{EditVerseAction, EditVerseWidget};
use bible_verse_memory::widgets::review::{ReviewAction, ReviewWidget};

#[derive(Parser, Debug)]
#[command(name = "bvm")]
#[command(about = "Bible Verse Memory - Spaced repetition for Scripture memorization")]
struct Args {
    #[arg(long)]
    show_config: bool,
}

enum AppMode {
    Dashboard(DashboardWidget),
    AddVerse(AddVerseWidget),
    EditVerse(EditVerseWidget),
    Review(ReviewWidget),
    ConfirmDelete { index: usize },
}

struct App {
    running: bool,
    mode: AppMode,
    collection: VerseCollection,
    config: Config,
    today: NaiveDate,
}

impl App {
    fn new(config: Config) -> Result<Self> {
        let mut collection = load_verses(&config)?;
        collection.migrate();
        let dashboard = DashboardWidget::new(&collection);
        let today = chrono::Utc::now().date_naive();

        Ok(Self {
            running: true,
            mode: AppMode::Dashboard(dashboard),
            collection,
            config,
            today,
        })
    }

    fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        match &mut self.mode {
            AppMode::Dashboard(dashboard) => {
                dashboard.render(frame, &self.collection, self.today);
            }
            AppMode::AddVerse(add) => add.render(frame),
            AppMode::EditVerse(edit) => edit.render(frame),
            AppMode::Review(review) => {
                review.render(frame, &self.collection, self.today);
            }
            AppMode::ConfirmDelete { index } => {
                let index = *index;
                render_confirm_delete(frame, &self.collection, index);
            }
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Paste(text) => match &mut self.mode {
                AppMode::AddVerse(add) => {
                    add.handle_paste(&text);
                }
                AppMode::EditVerse(edit) => {
                    edit.handle_paste(&text);
                }
                _ => {}
            },
            Event::Key(key) if key.kind == KeyEventKind::Press => match &mut self.mode {
                AppMode::Dashboard(dashboard) => {
                    let action = dashboard.handle_key(key, &self.collection);
                    self.handle_dashboard_action(action)?;
                }
                AppMode::AddVerse(add) => {
                    let action = add.handle_key(key);
                    match action {
                        AddVerseAction::None => {}
                        AddVerseAction::Cancel => self.switch_to_dashboard(),
                        AddVerseAction::Save => {
                            if let Err(e) = add.validate() {
                                add.error_message = Some(e);
                            } else {
                                let verse = MemoryVerse::new(
                                    add.reference.trim().to_string(),
                                    add.text.trim().to_string(),
                                );
                                self.collection.add(verse);
                                save_verses(&self.collection, &self.config)?;
                                self.switch_to_dashboard();
                            }
                        }
                    }
                }
                AppMode::EditVerse(edit) => {
                    let action = edit.handle_key(key);
                    match action {
                        EditVerseAction::None => {}
                        EditVerseAction::Cancel => self.switch_to_dashboard(),
                        EditVerseAction::Save => {
                            if let Err(e) = edit.validate() {
                                edit.error_message = Some(e);
                            } else {
                                let idx = edit.index;
                                self.collection.verses[idx].reference =
                                    edit.reference.trim().to_string();
                                self.collection.verses[idx].text = edit.text.trim().to_string();
                                save_verses(&self.collection, &self.config)?;
                                self.switch_to_dashboard();
                            }
                        }
                    }
                }
                AppMode::Review(review) => {
                    let action = review.handle_key(key, &mut self.collection, self.today);
                    match action {
                        ReviewAction::None => {}
                        ReviewAction::Cancel | ReviewAction::Done => {
                            save_verses(&self.collection, &self.config)?;
                            self.switch_to_dashboard();
                        }
                        ReviewAction::Save => {
                            save_verses(&self.collection, &self.config)?;
                        }
                    }
                }
                AppMode::ConfirmDelete { index } => {
                    let idx = *index;
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            self.collection.remove(idx);
                            save_verses(&self.collection, &self.config)?;
                            self.switch_to_dashboard();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            self.switch_to_dashboard();
                        }
                        _ => {}
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_dashboard_action(&mut self, action: DashboardAction) -> Result<()> {
        match action {
            DashboardAction::None => {}
            DashboardAction::Quit => self.quit()?,
            DashboardAction::AddVerse => {
                self.mode = AppMode::AddVerse(AddVerseWidget::new());
            }
            DashboardAction::StartReview => {
                let review = ReviewWidget::new(&self.collection, self.today);
                self.mode = AppMode::Review(review);
            }
            DashboardAction::ReviewVerse(idx) => {
                let review = ReviewWidget::new_single(idx);
                self.mode = AppMode::Review(review);
            }
            DashboardAction::EditVerse(idx) => {
                let verse = &self.collection.verses[idx];
                self.mode = AppMode::EditVerse(EditVerseWidget::new(idx, verse));
            }
            DashboardAction::ConfirmDelete(idx) => {
                self.mode = AppMode::ConfirmDelete { index: idx };
            }
        }
        Ok(())
    }

    fn switch_to_dashboard(&mut self) {
        let mut dashboard = DashboardWidget::new(&self.collection);
        let len = self.collection.verses.len();
        if len > 0 {
            dashboard.selected = dashboard.selected.min(len - 1);
        }
        self.mode = AppMode::Dashboard(dashboard);
    }

    fn quit(&mut self) -> Result<()> {
        save_verses(&self.collection, &self.config)?;
        self.running = false;
        Ok(())
    }
}

fn render_confirm_delete(frame: &mut Frame, collection: &VerseCollection, index: usize) {
    let area = frame.area();

    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 7.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let verse = &collection.verses[index];
    let text = format!(
        "Delete \"{}\"?\n\nPress 'y' to confirm or 'n'/Esc to cancel.",
        verse.reference
    );
    let block = ratatui::widgets::Block::default()
        .title("Confirm Delete")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let config = Config::load()?;

    if args.show_config {
        println!("Configuration:");
        println!("  Config file: {}", config.config_file_path().display());
        let data_path = config.data_path_absolute();
        if config.data_path_overridden() {
            println!(
                "  Data path: {} (overridden in dev mode)",
                data_path.display()
            );
        } else {
            println!("  Data path: {}", data_path.display());
        }
        return Ok(());
    }

    let mut terminal = ratatui::init();
    crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
    let mut app = App::new(config)?;
    let result = app.run(&mut terminal);
    crossterm::execute!(io::stdout(), DisableBracketedPaste)?;
    ratatui::restore();
    result
}
