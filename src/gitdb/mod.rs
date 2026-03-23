pub mod worktree;

#[derive(Debug)]
pub struct GitDb {
    worktree: crate::gitdb::worktree::Worktree,
}

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
    /// Check out a git ref into an ephemeral worktree.
    pub fn get(git_ref: &str) -> Result<GitDb, Error> {
        crate::git::ensure_branch_exists(git_ref)?;
        Ok(GitDb {
            worktree: worktree::Worktree::new_detached(git_ref)?,
        })
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.worktree.path().into()
    }
}

impl GitDbMut {
    /// Check out a git ref into an ephemeral worktree.
    pub fn get(git_ref: &str) -> Result<GitDbMut, Error> {
        crate::git::ensure_branch_exists(git_ref)?;
        Ok(GitDbMut {
            worktree: worktree::Worktree::new(git_ref)?,
        })
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.worktree.path().into()
    }
}
