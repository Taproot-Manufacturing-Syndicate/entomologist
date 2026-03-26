pub mod ui;

use core::cell::RefCell;
use entomologist::{
    comment::Comment,
    issue::{Issue, IssueHandle, State},
    Issues, IssuesMut,
};
use ratatui::widgets::ListState;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    EntIssuesError(#[from] entomologist::issues::Error),
    #[error(transparent)]
    EntMutIssuesError(#[from] entomologist::issues_mut::Error),
    #[error("invalid issue")]
    InvalidIssue,
    #[error(transparent)]
    GitError(#[from] entomologist::git::GitError),
}

#[derive(Debug, Clone)]
pub struct Entry {
    title: String,
    id: IssueHandle,
    pub state: State,
    assignee: Option<String>,
    tags: Vec<String>,
    description: String,
}

impl Entry {
    pub fn new_from_id_issue(id: &IssueHandle, issue: &Issue) -> Self {
        Entry {
            title: String::from(issue.title()),
            id: id.clone(),
            state: issue.state.clone(),
            assignee: issue.assignee.clone(),
            tags: issue.tags.clone(),
            description: issue.description.clone(),
        }
    }
    pub fn write_issue_to_db(&self) -> Result<(), Error> {
        let git_ref = "entomologist-data";
        let mut issues = entomologist::IssuesMut::new_from_git(git_ref)?;

        // TODO: for now only update state
        if let Some(issue) = issues.get_issue_mut(&self.id) {
            issue.set_state(self.state.clone());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct StateSelectorWidget {
    list_state: RefCell<ListState>,
    selected_state: RefCell<State>,
}

impl StateSelectorWidget {
    pub fn new() -> Self {
        Self {
            list_state: RefCell::new(ListState::default()),
            selected_state: RefCell::new(State::New),
        }
    }

    pub fn scroll_up(&self) {
        self.list_state.borrow_mut().select_next();
    }

    pub fn scroll_down(&self) {
        self.list_state.borrow_mut().select_previous();
    }

    pub fn get_selected(&self) -> State {
        self.selected_state.borrow().clone()
    }
}

#[derive(Debug)]
pub struct IssuesList {
    issues: Issues,
    // safety: this is only accessed from the UI thread
    list_state: RefCell<ListState>,
    selected_issue: RefCell<Option<Entry>>,
}

impl IssuesList {
    pub fn new() -> Result<Self, Error> {
        let git_ref = "entomologist-data";
        let issues = entomologist::Issues::new_from_git(git_ref)?;

        Ok(Self {
            issues,
            list_state: RefCell::new(ListState::default()),
            selected_issue: RefCell::new(None),
        })
    }

    pub fn select_previous(&self) {
        self.list_state.borrow_mut().select_previous();
    }

    pub fn select_next(&self) {
        self.list_state.borrow_mut().select_next();
    }

    pub fn get_selected(&self) -> Option<Entry> {
        self.selected_issue.borrow().clone()
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct CommentEntry {
    uuid: String,
    author: String,
    creation_time: chrono::DateTime<chrono::Local>,
    description: String,
}

impl CommentEntry {
    pub fn new_from_comment(comment: &Comment) -> Self {
        CommentEntry {
            uuid: comment.uuid.clone(),
            author: comment.author.clone(),
            creation_time: comment.creation_time,
            description: comment.description.clone(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct CommentsList {
    comments: Vec<Comment>,
    // safety: this is only accessed from the UI thread
    list_state: RefCell<tui_widget_list::ListState>,
    selected_comment: RefCell<Option<Comment>>,
}

impl CommentsList {
    pub fn new(entry: Entry) -> Result<Self, Error> {
        let git_ref = "entomologist-data";
        let issues = entomologist::Issues::new_from_git(git_ref)?;

        if let Some(issue) = issues.get_issue(&entry.id) {
            let comments = issue.get_comments();
            Ok(Self {
                comments,
                list_state: RefCell::new(tui_widget_list::ListState::default()),
                selected_comment: RefCell::new(None),
            })
        } else {
            Err(Error::InvalidIssue)
        }
    }

    pub fn scroll_down(&self) {
        self.list_state.borrow_mut().previous();
    }

    pub fn scroll_up(&self) {
        self.list_state.borrow_mut().next();
    }
}

#[derive(Debug)]
pub struct EntManager {
    remote: String,
    git_ref: String,
}

impl EntManager {
    pub fn new(remote: &str, git_ref: &str) -> Self {
        Self {
            remote: String::from(remote),
            git_ref: String::from(git_ref),
        }
    }

    pub fn sync(&self) -> Result<(), Error> {
        let issues = entomologist::IssuesMut::new_from_git(&self.git_ref)?;
        entomologist::git::sync(&issues.path(), &self.remote, &self.git_ref)?;
        Ok(())
    }
}
