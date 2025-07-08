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
pub struct Filter {
    pub include_states: std::collections::HashSet<crate::issue::State>,
}

// Parses a filter description matching "state=STATE[,STATE*]"
pub fn parse_filter(filter_str: &str) -> Result<Filter, ParseFilterError> {
    let tokens: Vec<&str> = filter_str.split("=").collect();
    if tokens.len() != 2 {
        return Err(ParseFilterError::ParseError);
    }
    if tokens[0] != "state" {
        return Err(ParseFilterError::ParseError);
    }

    let mut include_states = std::collections::HashSet::<crate::issue::State>::new();
    for s in tokens[1].split(",") {
        include_states.insert(crate::issue::State::from_str(s)?);
    }

    Ok(Filter { include_states })
}
