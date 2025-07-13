use thiserror::Error;
use crate::{git::GitError, issues::ReadIssuesError};

/// Errors that the DB can emit:
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IssuesError(#[from] ReadIssuesError),
    #[error(transparent)]
    GitError(#[from] GitError),
}


/// The main function looks at the command-line arguments and determines
/// from there where to get the Issues Database to operate on.
///
/// * If the user specified `--issues-dir` we use that.
///
/// * If the user specified `--issues-branch` we make sure the branch
///   exists, then use that.
///
/// * If the user specified neither, we use the default branch
///   `entomologist-data` (after ensuring that it exists).
///
/// * If the user specified both, it's an operator error and we abort.
///
/// The result of that code populates an IssuesDatabaseSource object,
/// that gets used later to access the database.
pub enum IssuesDatabaseSource<'a> {
    Dir(&'a std::path::Path),
    Branch(&'a str),
}



/// The IssuesDatabase type is a "fat path".  It holds a PathBuf pointing
/// at the issues database directory, and optionally a Worktree object
/// corresponding to that path.
///
/// The worktree field itself is never read: we put its path in `dir`
/// and that's all that the calling code cares about.
///
/// The Worktree object is included here *when* the IssuesDatabaseSource
/// is a branch.  In this case a git worktree is created to hold the
/// checkout of the branch.  When the IssueDatabase object is dropped,
/// the contained/owned Worktree object is dropped, which deletes the
/// worktree directory from the filesystem and prunes the worktree from
/// git's worktree list.

pub struct IssuesDatabase {
    pub dir: std::path::PathBuf,

    #[allow(dead_code)]
    pub worktree: Option<crate::git::Worktree>,
}

pub enum IssuesDatabaseAccess {
    ReadOnly,
    ReadWrite,
}

pub fn make_issues_database(
    issues_database_source: &IssuesDatabaseSource,
    access_type: IssuesDatabaseAccess,
) -> Result<IssuesDatabase, Error> {
    match issues_database_source {
        IssuesDatabaseSource::Dir(dir) => Ok(IssuesDatabase {
            dir: std::path::PathBuf::from(dir),
            worktree: None,
        }),
        IssuesDatabaseSource::Branch(branch) => {
            let worktree = match access_type {
                IssuesDatabaseAccess::ReadOnly => {
                    crate::git::Worktree::new_detached(branch)?
                }
                IssuesDatabaseAccess::ReadWrite => crate::git::Worktree::new(branch)?,
            };
            Ok(IssuesDatabase {
                dir: std::path::PathBuf::from(worktree.path()),
                worktree: Some(worktree),
            })
        }
    }
}

pub fn read_issues_database(
    issues_database_source: &IssuesDatabaseSource,
) -> Result<crate::issues::Issues, Error> {
    let issues_database =
        make_issues_database(issues_database_source, IssuesDatabaseAccess::ReadOnly)?;
    Ok(crate::issues::Issues::new_from_dir(
        &issues_database.dir,
    )?)
}