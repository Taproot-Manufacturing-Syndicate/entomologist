pub mod ui;

use core::cell::RefCell;
use entomologist::{
    comment::Comment,
    issue::{Issue, IssueHandle},
    issues::Issues,
};
use ratatui::widgets::ListState;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    EntIssuesError(#[from] entomologist::issues::ReadIssuesError),
    #[error(transparent)]
    EntDbError(#[from] entomologist::database::Error),
    #[error("invalid issue")]
    InvalidIssue,
}

#[derive(Debug, Clone)]
pub struct Entry {
    title: String,
    id: IssueHandle,
    state: String,
    assignee: Option<String>,
    description: String,
}

impl Entry {
    pub fn new_from_id_issue(id: &IssueHandle, issue: &Issue) -> Self {
        Entry {
            title: String::from(issue.title()),
            id: id.clone(),
            state: issue.state.to_string(),
            assignee: issue.assignee.clone(),
            description: issue.description.clone(),
        }
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
        let issues_db_source =
            entomologist::database::IssuesDatabaseSource::Branch("entomologist-data");
        let issues = entomologist::database::read_issues_database(&issues_db_source)?;
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
        let issues_database_source =
            entomologist::database::IssuesDatabaseSource::Branch("entomologist-data");
        let issues = entomologist::database::read_issues_database(&issues_database_source)?;

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
}
