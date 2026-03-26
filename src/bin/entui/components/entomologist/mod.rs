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
    #[error(transparent)]
    EntIssueError(#[from] entomologist::issue::IssueError),
}

#[derive(Debug, Clone)]
pub struct Entry {
    title: String,
    pub id: IssueHandle,
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

// These are the entries in the ratatui::widgets::List in the main "Issue List" view.
// We skip over Headings when selecting an Issue.
// FIXME: Use references, don't be so lazy and clone the Issue everywhere :-/
#[derive(Debug, PartialEq)]
enum IssueListItem {
    Heading(entomologist::issue::State),
    Issue(entomologist::Issue),
}

impl From<&entomologist::Issue> for IssueListItem {
    fn from(issue: &entomologist::Issue) -> Self {
        Self::Issue(issue.clone())
    }
}

impl From<entomologist::issue::State> for IssueListItem {
    fn from(state: entomologist::issue::State) -> Self {
        Self::Heading(state)
    }
}

impl From<&IssueListItem> for ratatui::widgets::ListItem<'_> {
    fn from(value: &IssueListItem) -> Self {
        match value {
            IssueListItem::Heading(state) => {
                let s = format!("\n--- {state} ---\n\n").to_uppercase();
                ratatui::widgets::ListItem::new(
                    ratatui::text::Text::from(s).style(ratatui::style::Style::default().bold()),
                )
            }
            IssueListItem::Issue(issue) => {
                let title = issue.title();
                let comments = match issue.comments.len() {
                    0 => String::from("    "),
                    n => format!("🗨️ {n}"),
                };
                ratatui::widgets::ListItem::new(format!("{comments}  {title}"))
            }
        }
    }
}

#[derive(Debug)]
pub struct IssuesList {
    issues: Issues,

    // This is the contents of the List in the main "Issue List" view. It
    // contains Heading (showing State) and Issue (showing an Issue).
    list_items: Vec<IssueListItem>,

    // safety: this is only accessed from the UI thread
    list_state: RefCell<ListState>,
    selected_issue: RefCell<Option<Entry>>,
}

impl IssuesList {
    pub fn new() -> Result<Self, Error> {
        let git_ref = "entomologist-data";
        let issues = entomologist::Issues::new_from_git(git_ref)?;

        // Vec of Issue, with only InProgress, Backlog, and New Issues
        // included, sorted by creation_time.
        let mut issue_list: Vec<Issue> = issues
            .iter()
            .map(|(_id, issue)| issue.clone())
            .filter(|issue| {
                issue.state == State::InProgress
                    || issue.state == State::Backlog
                    || issue.state == State::New
            })
            .collect();
        issue_list.sort_by(|issue_a, issue_b| issue_b.creation_time.cmp(&issue_a.creation_time));
        issue_list.sort_by(|issue_a, issue_b| issue_b.state.cmp(&issue_a.state));

        // Vec of IssueListItem (each item is Heading or Issue), with
        // only the selected Issues from above included.
        let mut list_items = Vec::<IssueListItem>::new();
        let mut prev_state: Option<entomologist::issue::State> = None;
        for issue in issue_list.iter() {
            match prev_state {
                None => {
                    list_items.push(IssueListItem::from(issue.state));
                }
                Some(state) => {
                    if issue.state != state {
                        list_items.push(IssueListItem::from(issue.state));
                    }
                }
            }
            prev_state = Some(issue.state);
            list_items.push(IssueListItem::from(issue));
        }

        Ok(Self {
            issues,
            list_items,
            list_state: RefCell::new(ListState::default()),
            selected_issue: RefCell::new(None),
        })
    }

    // Select the previous Issue item in the List, skipping over Heading items.
    pub fn select_previous(&self) {
        let mut s = self.list_state.borrow_mut();
        let old_index = match s.selected() {
            Some(old_index) => old_index,
            None => self.list_items.len(),
        };
        for index in (0..old_index).rev() {
            if let IssueListItem::Issue(_) = &self.list_items[index] {
                s.select(Some(index));
                if index == 1 {
                    *s.offset_mut() = 0;
                }
                return;
            }
        }
    }

    // Select the next Issue item in the List, skipping over Heading items.
    pub fn select_next(&self) {
        let mut s = self.list_state.borrow_mut();
        let old_index = match s.selected() {
            Some(old_index) => old_index,
            None => 0,
        };
        for index in (old_index + 1)..self.list_items.len() {
            if let IssueListItem::Issue(_) = &self.list_items[index] {
                s.select(Some(index));
                return;
            }
        }
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

    pub fn create_issue(&self) -> Result<(), Error> {
        let issues = entomologist::IssuesMut::new_from_git(&self.git_ref)?;
        entomologist::issue::Issue::new(&issues.path(), &None)?;
        Ok(())
    }

    pub fn add_comment(&self, issue_id: &IssueHandle) -> Result<(), Error> {
        let mut issues = entomologist::IssuesMut::new_from_git(&self.git_ref)?;
        if let Some(issue) = issues.get_issue_mut(&issue_id) {
            issue.add_comment(&None)?;
            Ok(())
        } else {
            Err(Error::InvalidIssue)
        }
    }
}
