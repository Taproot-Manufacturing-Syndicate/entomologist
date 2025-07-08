use core::fmt;
use std::io::Write;
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
    pub description: String,
    pub state: State,
    pub dependencies: Option<Vec<IssueHandle>>,
    pub comments: std::collections::HashMap<String, crate::comment::Comment>,

    /// This is the directory that the issue lives in.  Only used
    /// internally by the entomologist library.
    pub dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum IssueError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
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

impl Issue {
    pub fn new_from_dir(dir: &std::path::Path) -> Result<Self, IssueError> {
        let mut description: Option<String> = None;
        let mut state = State::New; // default state, if not specified in the issue
        let mut dependencies: Option<Vec<String>> = None;
        let mut comments = std::collections::HashMap::<String, crate::comment::Comment>::new();

        for direntry in dir.read_dir()? {
            if let Ok(direntry) = direntry {
                let file_name = direntry.file_name();
                if file_name == "description" {
                    description = Some(std::fs::read_to_string(direntry.path())?);
                } else if file_name == "state" {
                    let state_string = std::fs::read_to_string(direntry.path())?;
                    state = State::from_str(state_string.trim())?;
                } else if file_name == "dependencies" {
                    let dep_strings = std::fs::read_to_string(direntry.path())?;
                    let deps: Vec<IssueHandle> = dep_strings
                        .lines()
                        .map(|dep| IssueHandle::from(dep))
                        .collect();
                    if deps.len() > 0 {
                        dependencies = Some(deps);
                    }
                } else if file_name == "comments" && direntry.metadata()?.is_dir() {
                    Self::read_comments(&mut comments, &direntry.path())?;
                } else {
                    #[cfg(feature = "log")]
                    debug!("ignoring unknown file in issue directory: {:?}", file_name);
                }
            }
        }

        if description == None {
            return Err(IssueError::IssueParseError);
        }

        Ok(Self {
            description: description.unwrap(),
            state: state,
            dependencies,
            comments,
            dir: std::path::PathBuf::from(dir),
        })
    }

    fn read_comments(
        comments: &mut std::collections::HashMap<String, crate::comment::Comment>,
        dir: &std::path::Path,
    ) -> Result<(), IssueError> {
        for direntry in dir.read_dir()? {
            if let Ok(direntry) = direntry {
                let uuid = direntry.file_name();
                let comment = crate::comment::Comment::new_from_dir(&direntry.path())?;
                comments.insert(String::from(uuid.to_string_lossy()), comment);
            }
        }
        Ok(())
    }

    pub fn new_comment(&mut self) -> Result<crate::comment::Comment, IssueError> {
        let mut dir = std::path::PathBuf::from(&self.dir);
        dir.push("comments");
        if !dir.exists() {
            println!("creating {}", dir.to_string_lossy());
            std::fs::create_dir(&dir)?;
        }

        let rnd: u128 = rand::random();
        dir.push(&format!("{:032x}", rnd));
        std::fs::create_dir(&dir)?;

        Ok(crate::comment::Comment {
            description: String::from(""), // FIXME
            dir,
        })
    }

    pub fn new(dir: &std::path::Path) -> Result<Self, IssueError> {
        let mut issue_dir = std::path::PathBuf::from(dir);
        let rnd: u128 = rand::random();
        issue_dir.push(&format!("{:032x}", rnd));
        std::fs::create_dir(&issue_dir)?;
        Ok(Self {
            description: String::from(""), // FIXME: kind of bogus to use the empty string as None
            state: State::New,
            dependencies: None,
            comments: std::collections::HashMap::<String, crate::comment::Comment>::new(),
            dir: issue_dir,
        })
    }

    pub fn set_description(&mut self, description: &str) -> Result<(), IssueError> {
        self.description = String::from(description);
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        let mut description_file = std::fs::File::create(&description_filename)?;
        write!(description_file, "{}", description)?;
        crate::git::git_commit_file(&description_filename)?;
        Ok(())
    }

    pub fn read_description(&mut self) -> Result<(), IssueError> {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        self.description = std::fs::read_to_string(description_filename)?;
        Ok(())
    }

    pub fn edit_description(&mut self) -> Result<(), IssueError> {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        let result = std::process::Command::new("vi")
            .arg(&description_filename.as_mut_os_str())
            .spawn()?
            .wait_with_output()?;
        if !result.status.success() {
            println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
            println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
            return Err(IssueError::EditorError);
        }
        crate::git::git_commit_file(&description_filename)?;
        self.read_description()?;
        Ok(())
    }

    pub fn title<'a>(&'a self) -> &'a str {
        match self.description.find("\n") {
            Some(index) => &self.description.as_str()[..index],
            None => self.description.as_str(),
        }
    }

    pub fn set_state(&mut self, new_state: State) -> Result<(), IssueError> {
        let mut state_filename = std::path::PathBuf::from(&self.dir);
        state_filename.push("state");
        let mut state_file = std::fs::File::create(&state_filename)?;
        write!(state_file, "{}", new_state)?;
        crate::git::git_commit_file(&state_filename)?;
        Ok(())
    }

    pub fn read_state(&mut self) -> Result<(), IssueError> {
        let mut state_filename = std::path::PathBuf::from(&self.dir);
        state_filename.push("state");
        let state_string = std::fs::read_to_string(state_filename)?;
        self.state = State::from_str(state_string.trim())?;
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
            description: String::from("this is the title of my issue\n\nThis is the description of my issue.\nIt is multiple lines.\n* Arbitrary contents\n* But let's use markdown by convention\n"),
            state: State::New,
            dependencies: None,
            comments: std::collections::HashMap::<String, crate::comment::Comment>::new(),
            dir: std::path::PathBuf::from(issue_dir),
        };
        assert_eq!(issue, expected);
    }

    #[test]
    fn read_issue_1() {
        let issue_dir = std::path::Path::new("test/0000/7792b063eef6d33e7da5dc1856750c149ba678c6/");
        let issue = Issue::new_from_dir(issue_dir).unwrap();
        let expected = Issue {
            description: String::from("minimal"),
            state: State::InProgress,
            dependencies: None,
            comments: std::collections::HashMap::<String, crate::comment::Comment>::new(),
            dir: std::path::PathBuf::from(issue_dir),
        };
        assert_eq!(issue, expected);
    }
}
