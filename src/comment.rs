use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Comment {
    pub uuid: String,
    pub author: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub description: String,

    /// This is the directory that the comment lives in.  Only used
    /// internally by the entomologist library.
    pub dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CommentError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error("Failed to parse comment")]
    CommentParseError,
    #[error("Failed to run git")]
    GitError(#[from] crate::git::GitError),
    #[error("Failed to run editor")]
    EditorError,
}

impl Comment {
    pub fn new_from_dir(comment_dir: &std::path::Path) -> Result<Self, CommentError> {
        let mut description: Option<String> = None;

        for direntry in comment_dir.read_dir()? {
            if let Ok(direntry) = direntry {
                let file_name = direntry.file_name();
                if file_name == "description" {
                    description = Some(std::fs::read_to_string(direntry.path())?);
                } else {
                    #[cfg(feature = "log")]
                    debug!(
                        "ignoring unknown file in comment directory: {:?}",
                        file_name
                    );
                }
            }
        }
        if description == None {
            return Err(CommentError::CommentParseError);
        }

        let author = crate::git::git_log_oldest_author(comment_dir)?;
        let timestamp = crate::git::git_log_oldest_timestamp(comment_dir)?;
        let dir = std::path::PathBuf::from(comment_dir);

        Ok(Self {
            uuid: String::from(dir.file_name().unwrap().to_string_lossy()),
            author,
            timestamp,
            description: description.unwrap(),
            dir: std::path::PathBuf::from(comment_dir),
        })
    }

    pub fn set_description(&mut self, description: &str) -> Result<(), CommentError> {
        self.description = String::from(description);
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        let mut description_file = std::fs::File::create(&description_filename)?;
        write!(description_file, "{}", description)?;
        crate::git::git_commit_file(&description_filename)?;
        Ok(())
    }

    pub fn read_description(&mut self) -> Result<(), CommentError> {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        self.description = std::fs::read_to_string(description_filename)?;
        Ok(())
    }

    pub fn edit_description(&mut self) -> Result<(), CommentError> {
        let mut description_filename = std::path::PathBuf::from(&self.dir);
        description_filename.push("description");
        let result = std::process::Command::new("vi")
            .arg(&description_filename.as_mut_os_str())
            .spawn()?
            .wait_with_output()?;
        if !result.status.success() {
            println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
            println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
            return Err(CommentError::EditorError);
        }
        crate::git::git_commit_file(&description_filename)?;
        self.read_description()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_comment_0() {
        let comment_dir =
            std::path::Path::new("test/0001/dd79c8cfb8beeacd0460429944b4ecbe95a31561/comments/9055dac36045fe36545bed7ae7b49347");
        let comment = Comment::new_from_dir(comment_dir).unwrap();
        let expected = Comment {
            uuid: String::from("9055dac36045fe36545bed7ae7b49347"),
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            timestamp: chrono::DateTime::parse_from_rfc3339("2025-07-07T15:26:26-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            description: String::from("This is a comment on issue dd79c8cfb8beeacd0460429944b4ecbe95a31561\n\nIt has multiple lines\n"),
            dir: std::path::PathBuf::from(comment_dir),
        };
        assert_eq!(comment, expected);
    }
}
