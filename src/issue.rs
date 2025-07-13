use core::fmt;
use std::io::{IsTerminal, Write};
use std::str::FromStr;

#[cfg(feature = "log")]
use log::debug;

#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize)]
/// These are the states an issue can be in.
pub enum State {
    New,
    Backlog,
    Blocked,
    InProgress,
    Done,
    WontDo,
}

pub type IssueHandle = String;

#[derive(Debug, PartialEq)]
pub struct Issue {
    pub author: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub tags: Vec<String>,
    pub variables: std::collections::HashMap<String, String>,
    pub state: State,
    pub dependencies: Option<Vec<IssueHandle>>,
    pub assignee: Option<String>,
    pub description: String,
    pub comments: Vec<crate::comment::Comment>,

    /// This is the directory that the issue lives in.  Only used
    /// internally by the entomologist library.
    pub dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum IssueError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    EnvVarError(#[from] std::env::VarError),
    #[error(transparent)]
    CommentError(#[from] crate::comment::CommentError),
    #[error("Failed to parse issue")]
    IssueParseError,
    #[error("Failed to parse state")]
    StateParseError,
    #[error("Failed to run git")]
    GitError(#[from] crate::git::GitError),
    #[error("Failed to run editor")]
    EditorError,
    #[error("supplied description is empty")]
    EmptyDescription,
    #[error("tag {0} not found")]
    TagNotFound(String),
    #[error("stdin/stdout is not a terminal")]
    StdioIsNotTerminal,
}

impl FromStr for State {
    type Err = IssueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s == "new" {
            Ok(State::New)
        } else if s == "backlog" {
            Ok(State::Backlog)
        } else if s == "blocked" {
            Ok(State::Blocked)
        } else if s == "inprogress" {
            Ok(State::InProgress)
        } else if s == "done" {
            Ok(State::Done)
        } else if s == "wontdo" {
            Ok(State::WontDo)
        } else {
            Err(IssueError::StateParseError)
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_str = match self {
            State::New => "new",
            State::Backlog => "backlog",
            State::Blocked => "blocked",
            State::InProgress => "inprogress",
            State::Done => "done",
            State::WontDo => "wontdo",
        };
        write!(f, "{fmt_str}")
    }
}

// This is the public API of Issue.
impl Issue {
    pub fn new_from_dir(dir: &std::path::Path) -> Result<Self, IssueError> {
        let mut description: Option<String> = None;
        let mut state = State::New; // default state, if not specified in the issue
        let mut dependencies: Option<Vec<String>> = None;
        let mut comments = Vec::<crate::comment::Comment>::new();
        let mut assignee: Option<String> = None;
        let mut tags = Vec::<String>::new();
        let mut variables = std::collections::HashMap::<String, String>::new();

        for direntry in dir.read_dir()? {
            if let Ok(direntry) = direntry {
                let file_name = direntry.file_name();
                if file_name == "description" {
                    description = Some(std::fs::read_to_string(direntry.path())?);
                } else if file_name == "state" {
                    let state_string = std::fs::read_to_string(direntry.path())?;
                    state = State::from_str(state_string.trim())?;
                } else if file_name == "assignee" {
                    assignee = Some(String::from(
                        std::fs::read_to_string(direntry.path())?.trim(),
                    ));
                } else if file_name == "dependencies" {
                    let dep_strings = std::fs::read_to_string(direntry.path())?;
                    let deps: Vec<IssueHandle> = dep_strings
                        .lines()
                        .map(|dep| IssueHandle::from(dep))
                        .collect();
                    if deps.len() > 0 {
                        dependencies = Some(deps);
                    }
                } else if file_name == "tags" {
                    let contents = std::fs::read_to_string(direntry.path())?;
                    tags = contents
                        .lines()
                        .filter(|s| s.len() > 0)
                        .map(|tag| String::from(tag.trim()))
                        .collect();
                } else if file_name == "comments" && direntry.metadata()?.is_dir() {
                    Self::read_comments(&mut comments, &direntry.path())?;
                } else {
                    #[cfg(feature = "log")]
                    debug!("ignoring unknown file in issue directory: {:?}", file_name);
                }
                // FIXME: read variables file
            }
        }

        if description == None {
            return Err(IssueError::IssueParseError);
        }

        let author = crate::git::git_log_oldest_author(dir)?;
        let timestamp = crate::git::git_log_oldest_timestamp(dir)?;

        Ok(Self {
            author,
            timestamp,
            tags,
            variables,
            state: state,
            dependencies,
            assignee,
            description: description.unwrap(),
            comments,
            dir: std::path::PathBuf::from(dir),
        })
    }

    fn read_comments(
        comments: &mut Vec<crate::comment::Comment>,
        dir: &std::path::Path,
    ) -> Result<(), IssueError> {
        for direntry in dir.read_dir()? {
            if let Ok(direntry) = direntry {
                let comment = crate::comment::Comment::new_from_dir(&direntry.path())?;
                comments.push(comment);
            }
        }
        comments.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(())
    }

    /// Add a new Comment to the Issue.  Commits.
    pub fn add_comment(
        &mut self,
        description: &Option<String>,
    ) -> Result<crate::comment::Comment, IssueError> {
        let comment = crate::comment::Comment::new(self, description)?;
        Ok(comment)
    }

    /// Create a new Issue in an Issues database specified by a directory.
    /// The new Issue will live in a new subdirectory, named by a unique
    /// Issue identifier.
    ///
    /// If a description string is supplied, the new Issue's description
    /// will be initialized from it with no user interaction.
    ///
    /// If no description is supplied, the user will be prompted to
    /// input one into an editor.
    ///
    /// On success, the new Issue with its valid description is committed
    /// to the Issues database.
    pub fn new(dir: &std::path::Path, description: &Option<String>) -> Result<Self, IssueError> {
        let mut issue_dir = std::path::PathBuf::from(dir);
        let rnd: u128 = rand::random();
        let issue_id = format!("{:032x}", rnd);
        issue_dir.push(&issue_id);
        std::fs::create_dir(&issue_dir)?;

        let mut issue = Self {
            author: String::from(""),
            timestamp: chrono::Local::now(),
            tags: Vec::<String>::new(),
            variables: std::collections::HashMap::<String, String>::new(),
            state: State::New,
            dependencies: None,
            assignee: None,
            description: String::from(""), // FIXME: kind of bogus to use the empty string as None
            comments: Vec::<crate::comment::Comment>::new(),
            dir: issue_dir.clone(),
        };

        match description {
            Some(description) => {
                if description.len() == 0 {
                    return Err(IssueError::EmptyDescription);
                }
                issue.description = String::from(description);
                let description_filename = issue.description_filename();
                let mut description_file = std::fs::File::create(&description_filename)?;
                write!(description_file, "{}", description)?;
            }
            None => issue.edit_description_file()?,
        };

        crate::git::add(&issue_dir)?;
        crate::git::commit(&issue_dir, &format!("create new issue {}", issue_id))?;

        Ok(issue)
    }

    /// Interactively edit the description of an existing Issue.
    pub fn edit_description(&mut self) -> Result<(), IssueError> {
        self.edit_description_file()?;
        let description_filename = self.description_filename();
        crate::git::add(&description_filename)?;
        if crate::git::worktree_is_dirty(&self.dir.to_string_lossy())? {
            crate::git::commit(
                &description_filename.parent().unwrap(),
                &format!(
                    "edit description of issue {}",
                    description_filename
                        .parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                ),
            )?;
        }
        Ok(())
    }

    /// Return the Issue title (first line of the description).
    pub fn title<'a>(&'a self) -> &'a str {
        match self.description.find("\n") {
            Some(index) => &self.description.as_str()[..index],
            None => self.description.as_str(),
        }
    }

    /// Change the State of the Issue.
    pub fn set_state(&mut self, new_state: State) -> Result<(), IssueError> {
        let old_state = self.state.clone();
        let mut state_filename = std::path::PathBuf::from(&self.dir);
        state_filename.push("state");
        let mut state_file = std::fs::File::create(&state_filename)?;
        write!(state_file, "{}", new_state)?;
        crate::git::add(&state_filename)?;
        if crate::git::worktree_is_dirty(&self.dir.to_string_lossy())? {
            crate::git::commit(
                &self.dir,
                &format!(
                    "change state of issue {}, {} -> {}",
                    self.dir.file_name().unwrap().to_string_lossy(),
                    old_state,
                    new_state,
                ),
            )?;
        }
        Ok(())
    }

    pub fn read_state(&mut self) -> Result<(), IssueError> {
        let mut state_filename = std::path::PathBuf::from(&self.dir);
        state_filename.push("state");
        let state_string = std::fs::read_to_string(state_filename)?;
        self.state = State::from_str(state_string.trim())?;
        Ok(())
    }

    /// Set the Assignee of an Issue.
    pub fn set_assignee(&mut self, new_assignee: &str) -> Result<(), IssueError> {
        let old_assignee = match &self.assignee {
            Some(assignee) => assignee.clone(),
            None => String::from("None"),
        };
        let mut assignee_filename = std::path::PathBuf::from(&self.dir);
        assignee_filename.push("assignee");
        let mut assignee_file = std::fs::File::create(&assignee_filename)?;
        write!(assignee_file, "{}", new_assignee)?;
        crate::git::add(&assignee_filename)?;
        if crate::git::worktree_is_dirty(&self.dir.to_string_lossy())? {
            crate::git::commit(
                &self.dir,
                &format!(
                    "change assignee of issue {}, {} -> {}",
                    self.dir.file_name().unwrap().to_string_lossy(),
                    old_assignee,
                    new_assignee,
                ),
            )?;
        }
        Ok(())
    }

    /// Add a new Tag to the Issue.  Commits.
    pub fn add_tag(&mut self, tag: &str) -> Result<(), IssueError> {
        let tag_string = String::from(tag);
        if self.tags.contains(&tag_string) {
            return Ok(());
        }
        self.tags.push(tag_string);
        self.tags.sort();
        self.commit_tags(&format!(
            "issue {} add tag {}",
            self.dir.file_name().unwrap().to_string_lossy(),
            tag
        ))?;
        Ok(())
    }

    /// Remove a Tag from the Issue.  Commits.
    pub fn remove_tag(&mut self, tag: &str) -> Result<(), IssueError> {
        let tag_string = String::from(tag);
        let Some(index) = self.tags.iter().position(|x| x == &tag_string) else {
            return Err(IssueError::TagNotFound(tag_string));
        };
        self.tags.remove(index);
        self.commit_tags(&format!(
            "issue {} remove tag {}",
            self.dir.file_name().unwrap().to_string_lossy(),
            tag
        ))?;
        Ok(())
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        let tag_string = String::from(tag);
        self.tags.iter().position(|x| x == &tag_string).is_some()
    }

    pub fn has_any_tag(&self, tags: &std::collections::HashSet<&str>) -> bool {
        for tag in tags.iter() {
            if self.has_tag(tag) {
                return true;
            }
        }
        return false;
    }
}

// This is the internal/private API of Issue.
impl Issue {
    fn description_filename(&self) -> std::path::PathBuf {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        description_filename
    }

    /// Read the Issue's description file into the internal Issue representation.
    fn read_description(&mut self) -> Result<(), IssueError> {
        let description_filename = self.description_filename();
        self.description = std::fs::read_to_string(description_filename)?;
        Ok(())
    }

    /// Opens the Issue's `description` file in an editor.  Validates the
    /// editor's exit code.  Updates the Issue's internal description
    /// from what the user saved in the file.
    ///
    /// Used by Issue::new() when no description is supplied, and also
    /// used by `ent edit ISSUE`.
    fn edit_description_file(&mut self) -> Result<(), IssueError> {
        if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
            return Err(IssueError::StdioIsNotTerminal);
        }

        let description_filename = self.description_filename();
        let exists = description_filename.exists();
        let editor = match std::env::var("EDITOR") {
            Ok(editor) => editor,
            Err(std::env::VarError::NotPresent) => String::from("vi"),
            Err(e) => return Err(e.into()),
        };
        let result = std::process::Command::new(editor)
            .arg(&description_filename.as_os_str())
            .spawn()?
            .wait_with_output()?;
        if !result.status.success() {
            println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
            println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
            return Err(IssueError::EditorError);
        }
        if !description_filename.exists() || description_filename.metadata()?.len() == 0 {
            // User saved an empty file, or exited without saving while
            // editing a new description file.  Both means they changed
            // their mind and no longer want to edit the description.
            if exists {
                // File existed before the user emptied it, so restore
                // the original.
                crate::git::restore_file(&description_filename)?;
            }
            return Err(IssueError::EmptyDescription);
        }
        self.read_description()?;
        Ok(())
    }

    fn commit_tags(&self, commit_message: &str) -> Result<(), IssueError> {
        let mut tags_filename = self.dir.clone();
        tags_filename.push("tags");
        let mut tags_file = std::fs::File::create(&tags_filename)?;
        for tag in &self.tags {
            writeln!(tags_file, "{}", tag)?;
        }
        crate::git::add(&tags_filename)?;
        crate::git::commit(&self.dir, commit_message)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_issue_0() {
        let issue_dir = std::path::Path::new("test/0000/3943fc5c173fdf41c0a22251593cd476d96e6c9f/");
        let issue = Issue::new_from_dir(issue_dir).unwrap();
        let expected = Issue {
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            timestamp: chrono::DateTime::parse_from_rfc3339("2025-07-03T12:14:26-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            tags: Vec::<String>::from([
                String::from("tag1"),
                String::from("TAG2"),
                String::from("i-am-also-a-tag")
            ]),
            variables: std::collections::HashMap::<String, String>::new(),
            state: State::New,
            dependencies: None,
            assignee: None,
            description: String::from("this is the title of my issue\n\nThis is the description of my issue.\nIt is multiple lines.\n* Arbitrary contents\n* But let's use markdown by convention\n"),
            comments: Vec::<crate::comment::Comment>::new(),
            dir: std::path::PathBuf::from(issue_dir),
        };
        assert_eq!(issue, expected);
    }

    #[test]
    fn read_issue_1() {
        let issue_dir = std::path::Path::new("test/0000/7792b063eef6d33e7da5dc1856750c149ba678c6/");
        let issue = Issue::new_from_dir(issue_dir).unwrap();
        let expected = Issue {
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            timestamp: chrono::DateTime::parse_from_rfc3339("2025-07-03T12:14:26-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            tags: Vec::<String>::new(),
            variables: std::collections::HashMap::<String, String>::new(),
            state: State::InProgress,
            dependencies: None,
            assignee: Some(String::from("beep boop")),
            description: String::from("minimal"),
            comments: Vec::<crate::comment::Comment>::new(),
            dir: std::path::PathBuf::from(issue_dir),
        };
        assert_eq!(issue, expected);
    }
}
