use std::str::FromStr;

pub mod comment;
pub mod git;
pub mod issue;
pub mod issues;

#[derive(Debug, thiserror::Error)]
pub enum ParseFilterError {
    #[error("Failed to parse filter")]
    ParseError,
    #[error(transparent)]
    IssueParseError(#[from] crate::issue::IssueError),
}

// FIXME: It's easy to imagine a full dsl for filtering issues, for now
// i'm starting with obvious easy things.  Chumsky looks appealing but
// more research is needed.
#[derive(Debug)]
pub struct Filter<'a> {
    pub include_states: std::collections::HashSet<crate::issue::State>,
    pub include_assignees: std::collections::HashSet<&'a str>,
}

impl<'a> Filter<'a> {
    pub fn new_from_str(filter_str: &'a str) -> Result<Filter<'a>, ParseFilterError> {
        use crate::issue::State;
        let mut f = Filter {
            include_states: std::collections::HashSet::<crate::issue::State>::from([
                State::InProgress,
                State::Blocked,
                State::Backlog,
                State::New,
            ]),
            include_assignees: std::collections::HashSet::<&'a str>::new(),
        };

        for filter_chunk_str in filter_str.split(":") {
            let tokens: Vec<&str> = filter_chunk_str.split("=").collect();
            if tokens.len() != 2 {
                return Err(ParseFilterError::ParseError);
            }

            match tokens[0] {
                "state" => {
                    f.include_states.clear();
                    for s in tokens[1].split(",") {
                        f.include_states.insert(crate::issue::State::from_str(s)?);
                    }
                }
                "assignee" => {
                    f.include_assignees.clear();
                    for s in tokens[1].split(",") {
                        f.include_assignees.insert(s);
                    }
                }
                _ => {
                    println!("unknown filter chunk '{}'", filter_chunk_str);
                    return Err(ParseFilterError::ParseError);
                }
            }
        }

        Ok(f)
    }
}
