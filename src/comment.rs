use std::io::{IsTerminal, Write};

#[derive(Debug, PartialEq)]
pub struct Comment {
    pub uuid: String,
    pub author: String,
    pub creation_time: chrono::DateTime<chrono::Local>,
    pub description: String,

    /// This is the directory that the comment lives in.  Only used
    /// internally by the entomologist library.
    pub dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CommentError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    EnvVarError(#[from] std::env::VarError),
    #[error(transparent)]
    ChronoParseError(#[from] chrono::format::ParseError),
    #[error("Failed to parse comment")]
    CommentParseError,
    #[error("Failed to run git")]
    GitError(#[from] crate::git::GitError),
    #[error("Failed to run editor")]
    EditorError,
    #[error("supplied description is empty")]
    EmptyDescription,
    #[error("stdin/stdout is not a terminal")]
    StdioIsNotTerminal,
}

impl Comment {
    pub fn new_from_dir(comment_dir: &std::path::Path) -> Result<Self, CommentError> {
        let mut author: Option<String> = None;
        let mut creation_time: Option<chrono::DateTime<chrono::Local>> = None;
        let mut description: Option<String> = None;

        for direntry in (comment_dir.read_dir()?).flatten() {
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
            } else {
                #[cfg(feature = "log")]
                debug!(
                    "ignoring unknown file in comment directory: {:?}",
                    file_name
                );
            }
        }
        let Some(description) = description else {
            return Err(CommentError::CommentParseError);
        };

        if author.is_none() || creation_time.is_none() {
            let (git_author, git_creation_time) =
                crate::git::git_log_oldest_author_timestamp(comment_dir)?;
            if author.is_none() {
                author = Some(git_author);
            }
            if creation_time.is_none() {
                creation_time = Some(git_creation_time);
            }
        }
        let Some(author) = author else {
            return Err(CommentError::CommentParseError);
        };
        let Some(creation_time) = creation_time else {
            return Err(CommentError::CommentParseError);
        };

        let dir = std::path::PathBuf::from(comment_dir);

        Ok(Self {
            uuid: String::from(
                dir.file_name()
                    .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                    .to_string_lossy(),
            ),
            author,
            creation_time,
            description,
            dir: std::path::PathBuf::from(comment_dir),
        })
    }

    /// Create a new Comment on the specified Issue.  Commits.
    pub fn new(
        issue: &crate::issue::Issue,
        description: &Option<String>,
    ) -> Result<crate::comment::Comment, CommentError> {
        let mut dir = std::path::PathBuf::from(&issue.dir);
        dir.push("comments");
        if !dir.exists() {
            std::fs::create_dir(&dir)?;
        }

        let rnd: u128 = rand::random();
        let uuid = format!("{:032x}", rnd);
        dir.push(&uuid);
        std::fs::create_dir(&dir)?;

        let mut comment = crate::comment::Comment {
            uuid,
            author: String::from(""), // this will be updated from git when we re-read this comment
            creation_time: chrono::Local::now(),
            description: String::from(""), // this will be set immediately below
            dir: dir.clone(),
        };

        match description {
            Some(description) => {
                if description.is_empty() {
                    return Err(CommentError::EmptyDescription);
                }
                comment.description = String::from(description);
                let description_filename = comment.description_filename();
                let mut description_file = std::fs::File::create(&description_filename)?;
                write!(description_file, "{}", description)?;
            }
            None => comment.edit_description_file()?,
        };

        crate::git::add(&dir)?;
        if crate::git::worktree_is_dirty(&dir.to_string_lossy())? {
            crate::git::commit(
                &dir,
                &format!(
                    "add comment {} on issue {}",
                    comment.uuid,
                    issue
                        .dir
                        .file_name()
                        .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                        .to_string_lossy(),
                ),
            )?;
        }

        Ok(comment)
    }

    pub fn read_description(&mut self) -> Result<(), CommentError> {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        self.description = std::fs::read_to_string(description_filename)?;
        Ok(())
    }

    pub fn edit_description(&mut self) -> Result<(), CommentError> {
        self.edit_description_file()?;
        let description_filename = self.description_filename();
        crate::git::add(&description_filename)?;
        if crate::git::worktree_is_dirty(&self.dir.to_string_lossy())? {
            crate::git::commit(
                description_filename
                    .parent()
                    .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
                &format!(
                    "edit comment {} on issue FIXME", // FIXME: name the issue that the comment is on
                    self.dir
                        .file_name()
                        .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                        .to_string_lossy()
                ),
            )?;
            self.read_description()?;
        }
        Ok(())
    }

    /// Opens the Comment's `description` file in an editor.  Validates
    /// the editor's exit code.  Updates the Comment's internal
    /// description from what the user saved in the file.
    ///
    /// Used by Issue::add_comment() when no description is supplied,
    /// and (FIXME: in the future) used by `ent edit COMMENT`.
    pub fn edit_description_file(&mut self) -> Result<(), CommentError> {
        if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
            return Err(CommentError::StdioIsNotTerminal);
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
            return Err(CommentError::EditorError);
        }

        if !description_filename.exists() || description_filename.metadata()?.len() == 0 {
            // User saved an empty file, which means they changed their
            // mind and no longer want to edit the description.
            if exists {
                crate::git::restore_file(&description_filename)?;
            }
            return Err(CommentError::EmptyDescription);
        }
        self.read_description()?;
        Ok(())
    }
}

// This is the private, internal API.
impl Comment {
    fn description_filename(&self) -> std::path::PathBuf {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        description_filename
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn read_comment_0() {
        let comment_dir = std::path::Path::new(
            "test/0001/dd79c8cfb8beeacd0460429944b4ecbe/comments/9055dac36045fe36545bed7ae7b49347",
        );
        let comment = Comment::new_from_dir(comment_dir).unwrap();
        let expected = Comment {
            uuid: String::from("9055dac36045fe36545bed7ae7b49347"),
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T10:08:38-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            description: String::from(
                "This is a comment on issue dd79c8cfb8beeacd0460429944b4ecbe\n\nIt has multiple lines\n",
            ),
            dir: std::path::PathBuf::from(comment_dir),
        };
        assert_eq!(comment, expected);
    }
}
