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
    pub gitdb_mut: crate::gitdb::GitDbMut,
    pub issues: crate::Issues,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Issues(#[from] crate::issues::Error),

    #[error(transparent)]
    GitDB(#[from] crate::gitdb::Error),
}

impl IssuesMut {
    pub fn new_from_git(git_ref: &str) -> Result<Self, Error> {
        let gitdb_mut = crate::gitdb::GitDbMut::get(git_ref)?;
        let issues = crate::Issues::new_from_dir(&gitdb_mut.path())?;
        // The GitDbMut goes in the IssuesMut, so the underlying worktree survives as long as the
        // IssuesMut survives.
        Ok(Self { gitdb_mut, issues })
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.gitdb_mut.path()
    }

    pub fn add_issue(&mut self, issue: crate::issue::Issue) {
        self.issues.issues.insert(issue.id.clone(), issue);
    }

    pub fn get_issue(&self, issue_id: &str) -> Option<&crate::issue::Issue> {
        self.issues.get_issue(issue_id)
    }

    pub fn get_issue_mut(&mut self, issue_id: &str) -> Option<&mut crate::issue::Issue> {
        self.issues.issues.get_mut(issue_id)
    }

    /// Iterate over the Issue objects.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, crate::Issue> {
        self.issues.iter()
    }

    /// Iterate over the Issue objects (mutable).
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, String, crate::Issue> {
        self.issues.issues.iter_mut()
    }
}
