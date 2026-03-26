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
    StdIoError(#[from] std::io::Error),

    #[error("Error from git:\nstdout: {stdout}\nstderr: {stderr}")]
    Git { stdout: String, stderr: String },

    // The GitError is a leftover, should go away when the
    // entomologist::git module gets incorporated into entomologist::gitdb.
    #[error(transparent)]
    GitError(#[from] crate::git::GitError),

    #[error(transparent)]
    Worktree(#[from] worktree::Error),
}

impl GitDb {
    /// Check out a git ref into an ephemeral worktree, in detached
    /// head mode..
    pub fn get(git_ref: &str) -> Result<GitDb, Error> {
        ensure_branch_exists(git_ref)?;
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
        ensure_branch_exists(git_ref)?;
        Ok(GitDbMut {
            worktree: worktree::Worktree::new(git_ref)?,
        })
    }

    /// Get the path of the worktree.
    pub fn path(&self) -> std::path::PathBuf {
        self.worktree.path().into()
    }
}

fn ensure_branch_exists(branch: &str) -> Result<(), Error> {
    // Check for a local branch with the specified name.
    if crate::git::git_branch_exists(&format!("refs/heads/{branch}"))? {
        return Ok(());
    }

    // Check for *any* branch with the specified name, even remote.
    let result = std::process::Command::new("git")
        .args(["show-ref", branch])
        .output()?;
    match result.status.success() {
        true => {
            // Some remote has this branch, make a local branch from
            // the first one found.
            let output = String::from_utf8_lossy(&result.stdout);
            let line = output.split('\n').next().ok_or(Error::Git {
                stdout: String::from_utf8_lossy(&result.stdout).into(),
                stderr: String::from_utf8_lossy(&result.stderr).into(),
            })?;
            let remote_branch = line.split_whitespace().last().ok_or(Error::Git {
                stdout: String::from_utf8_lossy(&result.stdout).into(),
                stderr: String::from_utf8_lossy(&result.stderr).into(),
            })?;

            let result = std::process::Command::new("git")
                .args(["branch", branch, remote_branch])
                .output()?;
            if !result.status.success() {
                return Err(Error::Git {
                    stdout: String::from_utf8_lossy(&result.stdout).into(),
                    stderr: String::from_utf8_lossy(&result.stderr).into(),
                });
            }
        }
        false => {
            // No remote has this branch, make an empty one locally now.
            crate::git::create_orphan_branch(branch)?;
        }
    }

    Ok(())
}
