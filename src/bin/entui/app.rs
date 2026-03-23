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
#[derive(Debug)]
pub enum ViewState {
    Overview {
        issue_list: IssuesList,
    },
    Issue {
        issue: Entry,
        comments: CommentsList,
    },
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState::Overview {
            // TODO: unwrap as lazy
            issue_list: IssuesList::new().unwrap(),
        }
    }
}

impl ViewState {
    pub fn scroll_up(&self) {
        match &self {
            ViewState::Overview {issue_list} => {
                issue_list.select_next();
            }
            ViewState::Issue{comments, ..} => {
                comments.scroll_up();
            },
        }
    }
    pub fn scroll_down(&self) {
        match &self {
            ViewState::Overview {issue_list} => {
                issue_list.select_previous();
            }
            ViewState::Issue{comments, ..} => {
                comments.scroll_down();
            }
        }
    }
}

#[derive(Debug)]
pub enum PopupState {
    StateSelection
}

#[derive(Debug)]
pub struct ViewManager {
    // TODO: make these not need publicity
    pub view_state: ViewState,
    pub popup_state: Option<PopupState>,
}

impl Default for ViewManager {
    fn default() -> Self {
        Self {
            view_state: ViewState::default(),
            popup_state: None,
        }
    }
}

impl ViewManager {
    pub fn scroll_up(&self) {
        self.view_state.scroll_up();
    }
    pub fn scroll_down(&self) {
        self.view_state.scroll_down();
    }
    // TODO: make this not need mutability
    pub fn escape(&mut self) {
        if let Some(_) = self.popup_state {            
            self.popup_state = None;
        }
        else {
            self.view_state = ViewState::default();
        }
    }

    pub fn enter(&mut self) {
        match &self.view_state {    
            ViewState::Overview {issue_list} => {
                if let Some(issue) = issue_list.get_selected() {
                    if let Ok(comments) = CommentsList::new(issue.clone()) {
                        self.view_state = ViewState::Issue { issue, comments };
                    }
                }
            }
            ViewState::Issue{comments, ..} => {
                comments.scroll_down();
            }
        }
    }

    pub fn issue_state_popup_toggle(&mut self) {
         match &self.popup_state {
             Some(popup) => self.popup_state = None,
             None => self.popup_state = Some(PopupState::StateSelection),
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

    pub view_manager: ViewManager,

    // pub view_state: ViewState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            events: EventHandler::new(),
            view_manager: ViewManager::default(),
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
            view_manager: ViewManager::default(),
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
            KeyCode::Down | KeyCode::Char('j') => {
                self.view_manager.scroll_up();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.view_manager.scroll_down();
            }
            KeyCode::Enter => {
                self.view_manager.enter();
            }
            KeyCode::Esc => {
                self.view_manager.escape();
            }
            KeyCode::Char('s') => {
                self.view_manager.issue_state_popup_toggle();
                // set the state of an issue
                
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
