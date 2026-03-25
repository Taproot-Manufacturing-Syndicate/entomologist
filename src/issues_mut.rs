#[cfg(feature = "log")]
use log::debug;

/// `IssuesMut` is a deserialization of the GitDB, using a long-lived
/// ephemeral worktree. The worktree is made from the named branch,
/// and is not dropped until the IssuesMut object is dropped.
///
/// This means you can make changes to the IssuesMut object and the
/// changes will be incorporated into the GitDb ref as commits.
#[derive(Debug)]
pub struct IssuesMut {
    gitdb_mut: crate::gitdb::GitDbMut,
    issues: crate::Issues,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Issues(#[from] crate::issues::Error),

    #[error(transparent)]
    GitDB(#[from] crate::gitdb::Error),
}

/// Public API of Issues.
impl IssuesMut {
    /// Read Issues from a git ref (typically the `entomologist-data`
    /// branch). The resulting Issues struct provides a mutable, read-write
    /// view of the issues recorded in the git ref. The IssuesMut includes
    /// a git worktree with a checkout of the specified git ref, which
    /// enables adding/modifying/removing Issue objects and committing
    /// to the git ref.
    ///
    /// For an immutable read-only view use Issues instead.
    pub fn new_from_git(git_ref: &str) -> Result<Self, Error> {
        let gitdb_mut = crate::gitdb::GitDbMut::get(git_ref)?;
        let issues = crate::Issues::new_from_dir(&gitdb_mut.path())?;
        // The GitDbMut goes in the IssuesMut, so the underlying worktree survives as long as the
        // IssuesMut survives.
        Ok(Self { gitdb_mut, issues })
    }

    /// Get the path of the git worktree used as the backing store.
    pub fn path(&self) -> std::path::PathBuf {
        self.gitdb_mut.path()
    }

    // /// Add an Issue to this IssuesMut.
    // ///
    // /// Commits.
    // pub fn add_issue(&mut self, issue: crate::Issue) {
    //     self.issues.add_issue(issue);
    // }

    /// Look up an Issue by its id.
    pub fn get_issue(&self, issue_id: &str) -> Option<&crate::Issue> {
        self.issues.get_issue(issue_id)
    }

    /// Look up an Issue by its id (mutable).
    pub fn get_issue_mut(&mut self, issue_id: &str) -> Option<&mut crate::Issue> {
        self.issues.get_issue_mut(issue_id)
    }

    /// Iterate over the Issue objects.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, crate::Issue> {
        self.issues.iter()
    }

    /// Iterate over the Issue objects (mutable).
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, String, crate::Issue> {
        self.issues.iter_mut()
    }

    /// This converts an IssuesMut into an Issues, which drops the
    /// long-lived named-branch worktree of the IssuesMut.
    pub fn drop_mut(self) -> crate::Issues {
        self.issues
    }
}
