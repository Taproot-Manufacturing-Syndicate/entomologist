use crate::components::entomologist::{CommentsList, Entry, IssuesList};
use crate::event::{AppEvent, Event, EventHandler};
use ratatui::DefaultTerminal;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    EntError(#[from] crate::components::entomologist::Error),
}

/// view states
#[derive(Debug, Default, Clone)]
pub enum ViewState {
    #[default]
    Overview,
    Issue {
        issue: Entry,
        comments: CommentsList,
    },
}

impl ViewState {
    pub fn scroll_up(&self) {
        match &self {
            ViewState::Issue{comments, ..} => {
                comments.scroll_up();
            }
            _ => {}
        }
    }
    pub fn scroll_down(&self) {
        match &self {
            ViewState::Issue{comments, ..} => {
                comments.scroll_down();
            }
            _ => {}
        }
    }
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,

    pub issues_list: IssuesList,

    pub view_state: ViewState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            events: EventHandler::new(),
            // TODO: .unwrap() as laziness
            issues_list: IssuesList::new().unwrap(),
            view_state: ViewState::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            running: true,
            events: EventHandler::new(),
            // TODO: .unwrap() as laziness
            issues_list: IssuesList::new()?,
            view_state: ViewState::default(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Down => {
                self.issues_list.select_next();
            }
            KeyCode::Up => {
                self.issues_list.select_previous();
            }
            KeyCode::Char('j') => {
                // up
                self.view_state.scroll_up();
            }
            KeyCode::Char('k') => {
                self.view_state.scroll_down();
                // down
            }
            KeyCode::Enter => {
                if let Some(issue) = self.issues_list.get_selected() {
                    if let Ok(comments) = CommentsList::new(issue.clone()) {
                        self.view_state = ViewState::Issue { issue, comments };
                    }
                }
            }
            KeyCode::Esc => {
                self.view_state = ViewState::Overview;
            }
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
