#[cfg(feature = "log")]
use log::debug;

// Just a placeholder for now, get rid of this if we don't need it.
#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Config {}

#[derive(Debug, PartialEq)]
pub struct Issues {
    pub issues: std::collections::HashMap<String, crate::issue::Issue>,
    pub config: Config,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadIssuesError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    IssueError(#[from] crate::issue::IssueError),
    #[error("cannot handle filename")]
    FilenameError(std::ffi::OsString),
    #[error(transparent)]
    TomlDeserializeError(#[from] toml::de::Error),
}

impl Default for Issues {
    fn default() -> Self {
        Self::new()
    }
}

impl Issues {
    pub fn new() -> Self {
        Self {
            issues: std::collections::HashMap::new(),
            config: Config {},
        }
    }

    pub fn add_issue(&mut self, issue: crate::issue::Issue) {
        self.issues.insert(issue.id.clone(), issue);
    }

    pub fn get_issue(&self, issue_id: &str) -> Option<&crate::issue::Issue> {
        self.issues.get(issue_id)
    }

    pub fn get_mut_issue(&mut self, issue_id: &str) -> Option<&mut crate::issue::Issue> {
        self.issues.get_mut(issue_id)
    }

    fn parse_config(&mut self, config_path: &std::path::Path) -> Result<(), ReadIssuesError> {
        let config_contents = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_contents)?;
        self.config = config;
        Ok(())
    }

    pub fn new_from_dir(dir: &std::path::Path) -> Result<Self, ReadIssuesError> {
        let mut issues = Self::new();

        for direntry in (dir.read_dir()?).flatten() {
            if direntry.metadata()?.is_dir() {
                match crate::issue::Issue::new_from_dir(direntry.path().as_path()) {
                    Err(e) => {
                        eprintln!(
                            "failed to parse issue {}, skipping",
                            direntry.file_name().to_string_lossy()
                        );
                        eprintln!("ignoring error: {:?}", e);
                        continue;
                    }
                    Ok(issue) => {
                        issues.add_issue(issue);
                    }
                }
            } else if direntry.file_name() == "config.toml" {
                issues.parse_config(direntry.path().as_path())?;
            } else {
                #[cfg(feature = "log")]
                debug!(
                    "ignoring unknown file in issues directory: {:?}",
                    direntry.file_name()
                );
            }
        }
        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn read_issues_0000() {
        let issues_dir = std::path::Path::new("test/0000/");
        let issues = Issues::new_from_dir(issues_dir).unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("7792b063eef6d33e7da5dc1856750c14");
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
        let issues_dir = std::path::Path::new("test/0001/");
        let issues = Issues::new_from_dir(issues_dir).unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("3fa5bfd93317ad25772680071d5ac325");
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
        let issues_dir = std::path::Path::new("test/0002/");
        let issues = Issues::new_from_dir(issues_dir).unwrap();

        let mut expected = Issues::new();

        let uuid = String::from("3fa5bfd93317ad25772680071d5ac325");
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
        expected.add_issue(
            crate::issue::Issue {
                id: uuid,
                author: String::from("sigil-03 <sigil@glyphs.tech>"),
                creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-24T08:39:20-06:00")
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
        let mut dir = std::path::PathBuf::from(issues_dir);
        dir.push(&uuid);
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
