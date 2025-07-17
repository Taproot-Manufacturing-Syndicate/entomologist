use std::str::FromStr;

pub mod comment;
pub mod database;
pub mod git;
pub mod issue;
pub mod issues;

use crate::issue::State;

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
    pub include_tags: std::collections::HashSet<&'a str>,
    pub exclude_tags: std::collections::HashSet<&'a str>,
}

impl<'a> Filter<'a> {
    pub fn new() -> Filter<'a> {
        Self {
            include_states: std::collections::HashSet::<crate::issue::State>::from([
                State::InProgress,
                State::Blocked,
                State::Backlog,
                State::New,
            ]),
            include_assignees: std::collections::HashSet::<&'a str>::new(),
            include_tags: std::collections::HashSet::<&'a str>::new(),
            exclude_tags: std::collections::HashSet::<&'a str>::new(),
        }
    }

    pub fn parse(&mut self, filter_str: &'a str) -> Result<(), ParseFilterError> {
        let tokens: Vec<&str> = filter_str.split("=").collect();
        if tokens.len() != 2 {
            return Err(ParseFilterError::ParseError);
        }

        match tokens[0] {
            "state" => {
                self.include_states.clear();
                for s in tokens[1].split(",") {
                    self.include_states
                        .insert(crate::issue::State::from_str(s)?);
                }
            }

            "assignee" => {
                self.include_assignees.clear();
                for s in tokens[1].split(",") {
                    self.include_assignees.insert(s);
                }
            }

            "tag" => {
                self.include_tags.clear();
                self.exclude_tags.clear();
                for s in tokens[1].split(",") {
                    if s.len() == 0 {
                        return Err(ParseFilterError::ParseError);
                    }
                    if s.chars().nth(0).unwrap() == '-' {
                        self.exclude_tags.insert(&s[1..]);
                    } else {
                        self.include_tags.insert(s);
                    }
                }
            }

            _ => {
                println!("unknown filter string '{}'", filter_str);
                return Err(ParseFilterError::ParseError);
            }
        }

        Ok(())
    }
}
