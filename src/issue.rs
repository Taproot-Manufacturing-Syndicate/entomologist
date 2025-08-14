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
    pub id: String,
    pub author: String,
    pub creation_time: chrono::DateTime<chrono::Local>,
    pub done_time: Option<chrono::DateTime<chrono::Local>>,
    pub tags: Vec<String>,
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
    #[error(transparent)]
    ChronoParseError(#[from] chrono::format::ParseError),
    #[error("Failed to parse issue")]
    IssueParseError,
    #[error("invalid escape character {escape:?} in tag file {filename:?}")]
    TagInvalidEscape { escape: String, filename: String },
    #[error("invalid trailing escape character ',' in tag file {filename:?}")]
    TagTrailingEscape { filename: String },
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
    #[error("Failed to parse issue ID")]
    IdError,
    #[error("Dependency not found")]
    DepNotFound,
    #[error("Dependency already exists")]
    DepExists,
    #[error("Self-dependency not allowed")]
    DepSelf,
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
        let mut author: Option<String> = None;
        let mut creation_time: Option<chrono::DateTime<chrono::Local>> = None;
        let mut description: Option<String> = None;
        let mut state = State::New; // default state, if not specified in the issue
        let mut dependencies: Option<Vec<String>> = None;
        let mut comments = Vec::<crate::comment::Comment>::new();
        let mut assignee: Option<String> = None;
        let mut tags = Vec::<String>::new();
        let mut done_time: Option<chrono::DateTime<chrono::Local>> = None;

        for direntry in (dir.read_dir()?).flatten() {
            let file_name = direntry.file_name();
            if file_name == "author" {
                author = Some(std::fs::read_to_string(direntry.path())?);
            } else if file_name == "creation_time" {
                let raw_creation_time = chrono::DateTime::<_>::parse_from_rfc3339(
                    std::fs::read_to_string(direntry.path())?.trim(),
                )?;
                creation_time = Some(raw_creation_time.into());
            } else if file_name == "description" {
                description = Some(std::fs::read_to_string(direntry.path())?);
            } else if file_name == "state" {
                let state_string = std::fs::read_to_string(direntry.path())?;
                state = State::from_str(state_string.trim())?;
            } else if file_name == "assignee" {
                assignee = Some(String::from(
                    std::fs::read_to_string(direntry.path())?.trim(),
                ));
            } else if file_name == "done_time" {
                let raw_done_time = chrono::DateTime::<_>::parse_from_rfc3339(
                    std::fs::read_to_string(direntry.path())?.trim(),
                )?;
                done_time = Some(raw_done_time.into());
            } else if file_name == "dependencies" && direntry.metadata()?.is_dir() {
                dependencies = Self::read_dependencies(&direntry.path())?;
            } else if file_name == "tags" {
                tags = Self::read_tags(&direntry)?;
            } else if file_name == "comments" && direntry.metadata()?.is_dir() {
                Self::read_comments(&mut comments, &direntry.path())?;
            } else {
                #[cfg(feature = "log")]
                debug!("ignoring unknown file in issue directory: {:?}", file_name);
            }
        }

        let Some(description) = description else {
            return Err(IssueError::IssueParseError);
        };

        // parse the issue ID from the directory name
        let id = if let Some(parsed_id) = match dir.file_name() {
            Some(name) => name.to_str(),
            None => Err(IssueError::IdError)?,
        } {
            String::from(parsed_id)
        } else {
            Err(IssueError::IdError)?
        };

        if author.is_none() || creation_time.is_none() {
            let (git_author, git_creation_time) = crate::git::git_log_oldest_author_timestamp(dir)?;
            if author.is_none() {
                author = Some(git_author);
            }
            if creation_time.is_none() {
                creation_time = Some(git_creation_time);
            }
        }
        let Some(author) = author else {
            return Err(IssueError::IssueParseError);
        };
        let Some(creation_time) = creation_time else {
            return Err(IssueError::IssueParseError);
        };

        Ok(Self {
            id,
            author,
            creation_time,
            done_time,
            tags,
            state,
            dependencies,
            assignee,
            description,
            comments,
            dir: std::path::PathBuf::from(dir),
        })
    }

    fn read_comments(
        comments: &mut Vec<crate::comment::Comment>,
        dir: &std::path::Path,
    ) -> Result<(), IssueError> {
        for direntry in (dir.read_dir()?).flatten() {
            let comment = crate::comment::Comment::new_from_dir(&direntry.path())?;
            comments.push(comment);
        }
        comments.sort_by(|a, b| a.creation_time.cmp(&b.creation_time));
        Ok(())
    }

    fn read_dependencies(dir: &std::path::Path) -> Result<Option<Vec<IssueHandle>>, IssueError> {
        let mut dependencies: Option<Vec<String>> = None;
        for direntry in (dir.read_dir()?).flatten() {
            match &mut dependencies {
                Some(deps) => {
                    deps.push(direntry.file_name().into_string().unwrap());
                }
                None => {
                    dependencies = Some(vec![direntry.file_name().into_string().unwrap()]);
                }
            }
        }
        if let Some(deps) = &mut dependencies {
            deps.sort();
        }
        Ok(dependencies)
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
            id: String::from(&issue_id),
            author: String::from(""),
            creation_time: chrono::Local::now(),
            done_time: None,
            tags: Vec::<String>::new(),
            state: State::New,
            dependencies: None,
            assignee: None,
            description: String::from(""), // FIXME: kind of bogus to use the empty string as None
            comments: Vec::<crate::comment::Comment>::new(),
            dir: issue_dir.clone(),
        };

        match description {
            Some(description) => {
                if description.is_empty() {
                    return Err(IssueError::EmptyDescription);
                }
                issue.description = String::from(description);
                let description_filename = issue.description_filename();
                let mut description_file = std::fs::File::create(&description_filename)?;
                write!(description_file, "{}", description)?;
            }
            None => issue.edit_description_file()?,
        };

        issue.commit(&format!("create new issue {}", issue_id))?;

        Ok(issue)
    }

    /// Interactively edit the description of an existing Issue.
    pub fn edit_description(&mut self) -> Result<(), IssueError> {
        self.edit_description_file()?;
        let description_filename = self.description_filename();
        self.commit(&format!(
            "edit description of issue {}",
            description_filename
                .parent()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
        ))?;
        Ok(())
    }

    /// Return the Issue title (first line of the description).
    pub fn title(&self) -> &str {
        match self.description.find("\n") {
            Some(index) => &self.description.as_str()[..index],
            None => self.description.as_str(),
        }
    }

    /// Change the State of the Issue.  If the new state is `Done`,
    /// set the Issue `done_time`.  Commits.
    pub fn set_state(&mut self, new_state: State) -> Result<(), IssueError> {
        let old_state = self.state.clone();
        let mut state_filename = std::path::PathBuf::from(&self.dir);
        state_filename.push("state");
        let mut state_file = std::fs::File::create(&state_filename)?;
        write!(state_file, "{}", new_state)?;
        self.commit(&format!(
            "change state of issue {}, {} -> {}",
            self.dir
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
            old_state,
            new_state,
        ))?;
        if new_state == State::Done {
            self.set_done_time(chrono::Local::now())?;
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

    /// Set the `done_time` of the Issue.  Commits.
    pub fn set_done_time(
        &mut self,
        done_time: chrono::DateTime<chrono::Local>,
    ) -> Result<(), IssueError> {
        let mut done_time_filename = std::path::PathBuf::from(&self.dir);
        done_time_filename.push("done_time");
        let mut done_time_file = std::fs::File::create(&done_time_filename)?;
        write!(done_time_file, "{}", done_time.to_rfc3339())?;
        self.done_time = Some(done_time);
        self.commit(&format!(
            "set done-time of issue {} to {}",
            self.dir
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
            done_time,
        ))?;
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
        self.commit(&format!(
            "change assignee of issue {}, {} -> {}",
            self.dir
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
            old_assignee,
            new_assignee,
        ))?;
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
            self.dir
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
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
            self.dir
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
            tag
        ))?;
        Ok(())
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        let tag_string = String::from(tag);
        self.tags.iter().any(|x| x == &tag_string)
    }

    pub fn has_any_tag(&self, tags: &std::collections::HashSet<&str>) -> bool {
        for tag in tags.iter() {
            if self.has_tag(tag) {
                return true;
            }
        }
        false
    }

    pub fn add_dependency(&mut self, dep: IssueHandle) -> Result<(), IssueError> {
        if self.id == dep {
            Err(IssueError::DepSelf)?;
        }
        match &mut self.dependencies {
            Some(v) => v.push(dep.clone()),
            None => self.dependencies = Some(vec![dep.clone()]),
        }
        let mut dir = std::path::PathBuf::from(&self.dir);
        dir.push("dependencies");
        if !dir.exists() {
            std::fs::create_dir(&dir)?;
        }

        dir.push(dep.clone());

        if !dir.exists() {
            std::fs::File::create(&dir)?;
            self.commit(&format!("add dep {} to issue {}", dep, self.id))?;
        } else {
            Err(IssueError::DepExists)?;
        }
        Ok(())
    }

    pub fn remove_dependency(&mut self, dep: IssueHandle) -> Result<(), IssueError> {
        match &mut self.dependencies {
            Some(v) => {
                if let Some(i) = v.iter().position(|d| d == &dep) {
                    v.remove(i);
                } else {
                    Err(IssueError::DepNotFound)?;
                }
            }
            None => Err(IssueError::DepNotFound)?,
        }
        self.commit(&format!("remove dep {} from issue {}", dep, self.id))?;
        Ok(())
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
            .arg(description_filename.as_os_str())
            .spawn()?
            .wait_with_output()?;
        if !result.status.success() {
            println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
            println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
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

    fn read_tags(tags_direntry: &std::fs::DirEntry) -> Result<Vec<String>, IssueError> {
        if !tags_direntry.metadata()?.is_dir() {
            eprintln!("issue has old-style tags file");
            return Err(IssueError::IssueParseError);
        }
        let mut tags = Vec::<String>::new();
        for direntry in (tags_direntry.path().read_dir()?).flatten() {
            let tag = Issue::tag_from_filename(&direntry.file_name().to_string_lossy())?;
            tags.push(tag);
        }
        tags.sort();
        Ok(tags)
    }

    /// Perform un-escape on a filename to make it into a tag:
    /// ",0" => ","
    /// ",1" => "/"
    fn tag_from_filename(filename: &str) -> Result<String, IssueError> {
        let mut tag = String::new();
        let mut token_iter = filename.split(',');
        let Some(start) = token_iter.next() else {
            return Err(IssueError::StdIoError(std::io::Error::from(
                std::io::ErrorKind::NotFound,
            )));
        };
        tag.push_str(start);
        for token in token_iter {
            match token.chars().nth(0) {
                Some('0') => {
                    tag.push(',');
                    tag.push_str(&token[1..]);
                }
                Some('1') => {
                    tag.push('/');
                    tag.push_str(&token[1..]);
                }
                Some(bogus) => {
                    return Err(IssueError::TagInvalidEscape {
                        escape: String::from(bogus),
                        filename: String::from(filename),
                    });
                }
                None => {
                    return Err(IssueError::TagTrailingEscape {
                        filename: String::from(filename),
                    });
                }
            }
        }
        Ok(tag)
    }

    // Perform escape on a tag to make it into a filename:
    // "," => ",0"
    // "/" => ",1"
    fn tag_to_filename(tag: &str) -> String {
        let mut filename = tag.replace(",", ",0");
        filename = filename.replace("/", ",1");
        filename
    }

    fn commit_tags(&self, commit_message: &str) -> Result<(), IssueError> {
        let mut tags_dir_name = self.dir.clone();
        tags_dir_name.push("tags");
        match std::fs::remove_dir_all(&tags_dir_name) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
            Ok(_) => (),
        }
        std::fs::create_dir(&tags_dir_name)?;
        for tag in &self.tags {
            let mut tag_filename = tags_dir_name.clone();
            tag_filename.push(Issue::tag_to_filename(tag));
            std::fs::File::create(&tag_filename)?;
        }
        self.commit(commit_message)?;
        Ok(())
    }

    fn commit(&self, commit_message: &str) -> Result<(), IssueError> {
        crate::git::add(&self.dir)?;
        if !crate::git::worktree_is_dirty(&self.dir.to_string_lossy())? {
            return Ok(());
        }
        crate::git::commit(&self.dir, commit_message)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tag_from_filename_0() {
        assert_eq!(
            Issue::tag_from_filename("hello").unwrap(),
            String::from("hello")
        );
    }

    #[test]
    fn tag_from_filename_1() {
        assert_eq!(
            Issue::tag_from_filename("hello,0world").unwrap(),
            String::from("hello,world")
        );
    }

    #[test]
    fn tag_from_filename_2() {
        assert_eq!(
            Issue::tag_from_filename("hello,1world").unwrap(),
            String::from("hello/world")
        );
    }

    #[test]
    fn tag_from_filename_3() {
        assert_eq!(
            Issue::tag_from_filename(",0hello,1world,0").unwrap(),
            String::from(",hello/world,")
        );
    }

    #[test]
    fn tag_from_filename_4() {
        // std::io::Error does not impl PartialEq :-(
        let filename = "hello,";
        match Issue::tag_from_filename(filename) {
            Ok(tag) => panic!(
                "tag_from_filename() accepted invalid input {:?} and returned {:?}",
                filename, tag
            ),
            Err(_e) => (),
        }
    }

    #[test]
    fn tag_from_filename_5() {
        // std::io::Error does not impl PartialEq :-(
        let filename = "hello,world";
        match Issue::tag_from_filename(filename) {
            Ok(tag) => panic!(
                "tag_from_filename() accepted invalid input {:?} and returned {:?}",
                filename, tag
            ),
            Err(_e) => (),
        }
    }

    #[test]
    fn tag_to_filename_0() {
        let tag = "hello";
        assert_eq!(Issue::tag_to_filename(tag), "hello");
    }

    #[test]
    fn tag_to_filename_1() {
        let tag = "hello,";
        assert_eq!(Issue::tag_to_filename(tag), "hello,0");
    }

    #[test]
    fn tag_to_filename_2() {
        let tag = "/hello";
        assert_eq!(Issue::tag_to_filename(tag), ",1hello");
    }

    #[test]
    fn tag_to_filename_3() {
        let tag = "hello/bye,boo";
        assert_eq!(Issue::tag_to_filename(tag), "hello,1bye,0boo");
    }

    #[test]
    fn tag_to_filename_4() {
        let tag = ",,,///,,,";
        assert_eq!(Issue::tag_to_filename(tag), ",0,0,0,1,1,1,0,0,0");
    }

    #[test]
    fn tag_to_filename_5() {
        let tag = ",0,0,1,1";
        assert_eq!(Issue::tag_to_filename(tag), ",00,00,01,01");
    }

    #[test]
    fn read_issue_0() {
        let issue_dir = std::path::Path::new("test/0000/3943fc5c173fdf41c0a22251593cd476/");
        let issue = Issue::new_from_dir(issue_dir).unwrap();
        let expected = Issue {
            id: String::from("3943fc5c173fdf41c0a22251593cd476"),
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:36:25-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            done_time: None,
            tags: Vec::<String>::from([
                String::from("TAG2"),
                String::from("bird/wing"),
                String::from("bird/wing/feather"),
                String::from("deer,antler"),
                String::from("deer,antler,tassle"),
                String::from("hop,scotch/shoe"),
                String::from("i-am-also-a-tag"),
                String::from("tag1"),
            ]),
            state: State::New,
            dependencies: None,
            assignee: None,
            description: String::from(
                "this is the title of my issue\n\nThis is the description of my issue.\nIt is multiple lines.\n* Arbitrary contents\n* But let's use markdown by convention\n",
            ),
            comments: Vec::<crate::comment::Comment>::new(),
            dir: std::path::PathBuf::from(issue_dir),
        };
        assert_eq!(issue, expected);
    }

    #[test]
    fn read_issue_1() {
        let issue_dir = std::path::Path::new("test/0000/7792b063eef6d33e7da5dc1856750c14/");
        let issue = Issue::new_from_dir(issue_dir).unwrap();
        let expected = Issue {
            id: String::from("7792b063eef6d33e7da5dc1856750c14"),
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:37:07-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            done_time: None,
            tags: Vec::<String>::new(),
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
