//! gitdb is a front end that lets you access a git branch by checking
//! it out in a temporary worktree.
//!
//! This module is used internally by entomologist, the user generally
//! doesn't need to care about it or use it directly.

pub mod worktree;

use std::io::Write;

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

fn create_orphan_branch(branch: &str) -> Result<(), Error> {
    {
        let tmp_worktree = tempfile::tempdir().unwrap();
        create_orphan_branch_at_path(branch, tmp_worktree.path())?;
    }
    // The temp dir is now removed / cleaned up.

    let result = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()?;
    if !result.status.success() {
        return Err(Error::Git {
            stdout: String::from_utf8_lossy(&result.stdout).into(),
            stderr: String::from_utf8_lossy(&result.stderr).into(),
        });
    }

    Ok(())
}

fn create_orphan_branch_at_path(
    branch: &str,
    worktree_path: &std::path::Path,
) -> Result<(), Error> {
    let worktree_dir = worktree_path.to_string_lossy();

    // Create a worktree at the path, with a detached head.
    let result = std::process::Command::new("git")
        .args(["worktree", "add", &worktree_dir, "HEAD"])
        .output()?;
    if !result.status.success() {
        return Err(Error::Git {
            stdout: String::from_utf8_lossy(&result.stdout).into(),
            stderr: String::from_utf8_lossy(&result.stderr).into(),
        });
    }

    // Create an empty orphan branch in the worktree.
    let result = std::process::Command::new("git")
        .args(["switch", "--orphan", branch])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        return Err(Error::Git {
            stdout: String::from_utf8_lossy(&result.stdout).into(),
            stderr: String::from_utf8_lossy(&result.stderr).into(),
        });
    }

    let mut readme_filename = std::path::PathBuf::from(worktree_path);
    readme_filename.push("README.md");
    let mut readme = std::fs::File::create(readme_filename)?;
    write!(
        readme,
        "This branch is used by entomologist to track issues."
    )?;

    let result = std::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        return Err(Error::Git {
            stdout: String::from_utf8_lossy(&result.stdout).into(),
            stderr: String::from_utf8_lossy(&result.stderr).into(),
        });
    }

    let result = std::process::Command::new("git")
        .args(["commit", "-m", "create entomologist issue branch"])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        return Err(Error::Git {
            stdout: String::from_utf8_lossy(&result.stdout).into(),
            stderr: String::from_utf8_lossy(&result.stderr).into(),
        });
    }

    Ok(())
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
            create_orphan_branch(branch)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn git_remove_branch(branch: &str) -> Result<(), Error> {
        let result = std::process::Command::new("git")
            .args(["branch", "-D", branch])
            .output()?;
        if !result.status.success() {
            return Err(Error::Git {
                stdout: String::from_utf8_lossy(&result.stdout).into(),
                stderr: String::from_utf8_lossy(&result.stderr).into(),
            });
        }
        Ok(())
    }

    #[test]
    fn test_create_orphan_branch() {
        let rnd: u128 = rand::random();
        let mut branch = std::string::String::from("entomologist-test-branch-");
        branch.push_str(&format!("{:032x}", rnd));
        create_orphan_branch(&branch).unwrap();
        assert_eq!(crate::git::git_branch_exists(&branch).unwrap(), true);
        git_remove_branch(&branch).unwrap();
    }
}
