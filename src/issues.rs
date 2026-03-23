#[cfg(feature = "log")]
use log::debug;

// Just a placeholder for now, get rid of this if we don't need it.
#[derive(Debug, Default, PartialEq, serde::Deserialize)]
pub struct Config {}

/// `Issues` is a deserialization of the GitDb, using a short-lived,
/// ephemeral worktree. The worktree is made from the detached head of the
/// GitDb branch, and is dropped as soon as the Issues are deserialized.
#[derive(Debug, Default, PartialEq)]
pub struct Issues {
    pub issues: std::collections::HashMap<String, crate::issue::Issue>,
    pub config: Config,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),

    #[error(transparent)]
    IssueError(#[from] crate::issue::IssueError),

    #[error("cannot handle filename")]
    FilenameError(std::ffi::OsString),

    #[error(transparent)]
    TomlDeserializeError(#[from] toml::de::Error),

    #[error(transparent)]
    GitDB(#[from] crate::gitdb::Error),
}

impl Issues {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_dir(dir: &std::path::Path) -> Result<Issues, Error> {
        // Read Issues from DB.
        let mut issues = std::collections::HashMap::<String, crate::issue::Issue>::new();
        let mut config = Config::default();

        for direntry in dir.read_dir()?.flatten() {
            if direntry.metadata()?.is_dir() {
                match crate::issue::Issue::new_from_dir(direntry.path().as_path()) {
                    Err(e) => {
                        eprintln!(
                            "failed to parse issue {}, skipping",
                            direntry.file_name().to_string_lossy()
                        );
                        eprintln!("ignoring error: {e:?}");
                        continue;
                    }
                    Ok(issue) => {
                        issues.insert(issue.id.clone(), issue);
                    }
                }
            } else if direntry.file_name() == "config.toml" {
                config = Issues::parse_config(direntry.path().as_path())?;
            } else {
                #[cfg(feature = "log")]
                debug!(
                    "ignoring unknown file in issues directory: {:?}",
                    direntry.file_name()
                );
            }
        }

        Ok(Self { issues, config })
    }

    pub fn new_from_git(git_ref: &str) -> Result<Self, Error> {
        let gitdb = crate::gitdb::GitDb::get(git_ref)?;
        let issues = Self::new_from_dir(&gitdb.path())?;
        // Drop the GitDb, this destroys the underlying worktree.
        Ok(issues)
    }

    pub fn get_issue(&self, issue_id: &str) -> Option<&crate::issue::Issue> {
        self.issues.get(issue_id)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, crate::Issue> {
        self.issues.iter()
    }

    pub fn add_issue(&mut self, issue: crate::Issue) {
        self.issues.insert(issue.id.clone(), issue);
    }

    fn parse_config(config_path: &std::path::Path) -> Result<Config, Error> {
        let config_contents = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn read_issues_0000() {
        let issues = Issues::new_from_git("entomologist-data-test-0000").unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("7792b063eef6d33e7da5dc1856750c14");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(crate::issue::Issue {
            id: uuid,
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:37:07-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            done_time: None,
            tags: Vec::<String>::new(),
            state: crate::issue::State::InProgress,
            dependencies: None,
            assignee: Some(String::from("beep boop")),
            description: String::from("minimal"),
            comments: Vec::<crate::comment::Comment>::new(),
            dir,
        });

        let uuid = String::from("3943fc5c173fdf41c0a22251593cd476");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(
            crate::issue::Issue {
                id: uuid,
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
                state: crate::issue::State::New,
                dependencies: None,
                assignee: None,
                description: String::from("this is the title of my issue\n\nThis is the description of my issue.\nIt is multiple lines.\n* Arbitrary contents\n* But let's use markdown by convention\n"),
                comments: Vec::<crate::comment::Comment>::new(),
                dir,
            }
        );
        assert_eq!(issues, expected);
    }

    #[test]
    fn read_issues_0001() {
        let issues = Issues::new_from_git("entomologist-data-test-0001").unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("3fa5bfd93317ad25772680071d5ac325");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(crate::issue::Issue {
            id: uuid,
            author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:37:46-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            done_time: Some(
                chrono::DateTime::parse_from_rfc3339("2025-07-15T15:15:15-06:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
            tags: Vec::<String>::new(),
            state: crate::issue::State::Done,
            dependencies: None,
            assignee: None,
            description: String::from("oh yeah we got titles"),
            comments: Vec::<crate::comment::Comment>::new(),
            dir,
        });

        let uuid = String::from("dd79c8cfb8beeacd0460429944b4ecbe");
        let dir = std::path::PathBuf::from(&uuid);
        let mut comment_dir = dir.clone();
        let comment_uuid = String::from("9055dac36045fe36545bed7ae7b49347");
        comment_dir.push("comments");
        comment_dir.push(&comment_uuid);
        let mut expected_comments = Vec::<crate::comment::Comment>::new();
        expected_comments.push(
            crate::comment::Comment {
                uuid: comment_uuid,
                author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
                creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T10:08:38-06:00").unwrap().with_timezone(&chrono::Local),
                description: String::from("This is a comment on issue dd79c8cfb8beeacd0460429944b4ecbe\n\nIt has multiple lines\n"),
                dir: std::path::PathBuf::from(comment_dir),
            }
        );
        expected.add_issue(
            crate::issue::Issue {
                id: uuid,
                author: String::from("Sebastian Kuzminsky <seb@highlab.com>"),
                creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T10:08:24-06:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
                done_time: None,
                tags: Vec::<String>::new(),
                state: crate::issue::State::WontDo,
                dependencies: None,
                assignee: None,
                description: String::from("issues out the wazoo\n\nLots of words\nthat don't say much\nbecause this is just\na test\n"),
                comments: expected_comments,
                dir,
            },
        );
        assert_eq!(issues, expected);
    }

    #[test]
    fn read_issues_0002() {
        let issues = Issues::new_from_git("entomologist-data-test-0002").unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("3fa5bfd93317ad25772680071d5ac325");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(crate::issue::Issue {
            id: uuid,
            author: String::from("sigil-03 <sigil@glyphs.tech>"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:38:40-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            done_time: None,
            tags: Vec::<String>::new(),
            state: crate::issue::State::Done,
            dependencies: None,
            assignee: None,
            description: String::from("oh yeah we got titles\n"),
            comments: Vec::<crate::comment::Comment>::new(),
            dir,
        });

        let uuid = String::from("dd79c8cfb8beeacd0460429944b4ecbe");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(
            crate::issue::Issue {
                id: uuid,
                author: "A Person <foo@example.org>".to_owned(),
                creation_time: chrono::DateTime::parse_from_rfc3339("2025-04-01T12:34:56-06:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
                done_time: None,
                tags: Vec::<String>::new(),
                state: crate::issue::State::WontDo,
                dependencies: None,
                assignee: None,
                description: String::from("issues out the wazoo\n\nLots of words\nthat don't say much\nbecause this is just\na test\n"),
                comments: Vec::<crate::comment::Comment>::new(),
                dir,
            },
        );

        let uuid = String::from("a85f81fc5f14cb5d4851dd445dc9744c");
        let dir = std::path::PathBuf::from(&uuid);
        expected.add_issue(
            crate::issue::Issue {
                id: uuid,
                author: String::from("sigil-03 <sigil@glyphs.tech>"),
                creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:39:02-06:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
                done_time: None,
                tags: Vec::<String>::new(),
                state: crate::issue::State::WontDo,
                dependencies: Some(vec![
                    crate::issue::IssueHandle::from("3fa5bfd93317ad25772680071d5ac325"),
                    crate::issue::IssueHandle::from("dd79c8cfb8beeacd0460429944b4ecbe"),
                ]),
                assignee: None,
                description: String::from("issue with dependencies\n\na test has begun\nfor dependencies we seek\nintertwining life"),
                comments: Vec::<crate::comment::Comment>::new(),
                dir,
            },
        );
        assert_eq!(issues, expected);
    }
}
