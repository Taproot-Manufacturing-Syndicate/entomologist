pub mod ui;

use core::cell::RefCell;
use ratatui::widgets::ListState;
use thiserror::Error;
use entomologist::{issue::{Issue, IssueHandle, State}, issues::Issues};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    EntIssuesError(#[from] entomologist::issues::ReadIssuesError),
    #[error(transparent)]
    EntDbError(#[from] entomologist::database::Error),

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
        let issues_db_source = entomologist::database::IssuesDatabaseSource::Branch("entomologist-data");
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
}