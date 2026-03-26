//! gitdb is a front end that lets you access a git branch by checking
//! it out in a temporary worktree.
//!
//! This module is used internally by entomologist, the user generally
//! doesn't need to care about it or use it directly.

pub mod worktree;

/// GitDb checks out a git ref in detached head mode, so any changes
/// made to the worktree can **not** be committed back to the ref. This
/// makes the GitDb effectively immutable, in the sense that there's no
/// way to make lasting changes to the git ref.
#[derive(Debug)]
pub struct GitDb {
    worktree: crate::gitdb::worktree::Worktree,
}

/// GitDbMut checks out a git ref in normal (named branch) mode, so
/// any changes made to the worktree **can** be committed back to the
/// ref. This makes the GitDbMut mutable, in the sense that it can add
/// commits to the git ref.
#[derive(Debug)]
pub struct GitDbMut {
    worktree: crate::gitdb::worktree::Worktree,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Git(#[from] crate::git::GitError),

    #[error(transparent)]
    Worktree(#[from] worktree::Error),
}

impl GitDb {
    /// Check out a git ref into an ephemeral worktree, in detached
    /// head mode..
    pub fn get(git_ref: &str) -> Result<GitDb, Error> {
        crate::git::ensure_branch_exists(git_ref)?;
        Ok(GitDb {
            worktree: worktree::Worktree::new_detached(git_ref)?,
        })
    }

    /// Get the path of the worktree.
    pub fn path(&self) -> std::path::PathBuf {
        self.worktree.path().into()
    }
}

impl GitDbMut {
    /// Check out a git ref into an ephemeral worktree, in normal (named
    /// branch) mode.
    pub fn get(git_ref: &str) -> Result<GitDbMut, Error> {
        crate::git::ensure_branch_exists(git_ref)?;
        Ok(GitDbMut {
            worktree: worktree::Worktree::new(git_ref)?,
        })
    }

    /// Get the path of the worktree.
    pub fn path(&self) -> std::path::PathBuf {
        self.worktree.path().into()
    }
}
