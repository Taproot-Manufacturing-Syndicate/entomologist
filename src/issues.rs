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
    #[error("Failed to parse issue")]
    IssueParseError(#[from] crate::issue::ReadIssueError),
    #[error("cannot handle filename")]
    FilenameError(std::ffi::OsString),
    #[error(transparent)]
    TomlDeserializeError(#[from] toml::de::Error),
}

impl Issues {
    pub fn new() -> Self {
        Self {
            issues: std::collections::HashMap::new(),
            config: Config {},
        }
    }

    pub fn add_issue(&mut self, uuid: String, issue: crate::issue::Issue) {
        self.issues.insert(uuid, issue);
    }

    fn parse_config(&mut self, config_path: &std::path::Path) -> Result<(), ReadIssuesError> {
        let config_contents = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_contents)?;
        self.config = config;
        Ok(())
    }

    pub fn new_from_dir(dir: &std::path::Path) -> Result<Self, ReadIssuesError> {
        let mut issues = Self::new();

        for direntry in dir.read_dir()? {
            if let Ok(direntry) = direntry {
                if direntry.metadata()?.is_dir() {
                    let uuid = match direntry.file_name().into_string() {
                        Ok(uuid) => uuid,
                        Err(orig_string) => {
                            return Err(ReadIssuesError::FilenameError(orig_string))
                        }
                    };
                    let issue = crate::issue::Issue::new_from_dir(direntry.path().as_path())?;
                    issues.add_issue(uuid, issue);
                } else if direntry.file_name() == "config.toml" {
                    issues.parse_config(direntry.path().as_path())?;
                } else {
                    println!(
                        "ignoring unknown file in issues directory: {:?}",
                        direntry.file_name()
                    );
                }
            }
        }
        return Ok(issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_issues_0000() {
        let issues_dir = std::path::Path::new("test/0000/");
        let issues = Issues::new_from_dir(issues_dir).unwrap();

        let mut expected = Issues::new();
        expected.add_issue(
            String::from("7792b063eef6d33e7da5dc1856750c149ba678c6"),
            crate::issue::Issue {
                title: String::from("minimal"),
                description: None,
                state: crate::issue::State::InProgress,
            },
        );
        expected.add_issue(
            String::from("3943fc5c173fdf41c0a22251593cd476d96e6c9f"),
            crate::issue::Issue {
                title: String::from("this is the title of my issue"),
                description: Some(String::from("This is the description of my issue.\nIt is multiple lines.\n* Arbitrary contents\n* But let's use markdown by convention\n")),
                state: crate::issue::State::New,
            }
        );
        assert_eq!(issues, expected);
    }

    #[test]
    fn read_issues_0001() {
        let issues_dir = std::path::Path::new("test/0001/");
        let issues = Issues::new_from_dir(issues_dir).unwrap();

        let mut expected = Issues::new();
        expected.add_issue(
            String::from("3fa5bfd93317ad25772680071d5ac3259cd2384f"),
            crate::issue::Issue {
                title: String::from("oh yeah we got titles"),
                description: None,
                state: crate::issue::State::Done,
            },
        );
        expected.add_issue(
            String::from("dd79c8cfb8beeacd0460429944b4ecbe95a31561"),
            crate::issue::Issue {
                title: String::from("issues out the wazoo"),
                description: Some(String::from(
                    "Lots of words\nthat don't say much\nbecause this is just\na test\n",
                )),
                state: crate::issue::State::WontDo,
            },
        );
        assert_eq!(issues, expected);
    }
}
