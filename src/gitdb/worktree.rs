#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error("Error from git:\nstdout: {stdout}\nstderr: {stderr}")]
    Git { stdout: String, stderr: String },
}

#[derive(Debug)]
/// `Worktree` is a struct that manages a temporary directory containing
/// a checkout of a specific branch.  The worktree is removed and pruned
/// when the `Worktree` struct is dropped.
pub struct Worktree {
    path: tempfile::TempDir,
}

impl Drop for Worktree {
    fn drop(&mut self) {
        let result = std::process::Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                &self.path.path().to_string_lossy(),
            ])
            .output();
        match result {
            Err(e) => {
                println!("failed to run git: {e:#?}");
            }
            Ok(result) => {
                if !result.status.success() {
                    println!("failed to remove git worktree: {result:#?}");
                }
            }
        }
    }
}

impl Worktree {
    pub fn new(branch: &str) -> Result<Worktree, Error> {
        let path = tempfile::tempdir()?;
        let result = std::process::Command::new("git")
            .args(["worktree", "add", &path.path().to_string_lossy(), branch])
            .output()?;
        if !result.status.success() {
            return Err(Error::Git {
                stdout: String::from_utf8_lossy(&result.stdout).into(),
                stderr: String::from_utf8_lossy(&result.stderr).into(),
            });
        }
        Ok(Self { path })
    }

    pub fn new_detached(branch: &str) -> Result<Worktree, Error> {
        let path = tempfile::tempdir()?;
        let result = std::process::Command::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                &path.path().to_string_lossy(),
                branch,
            ])
            .output()?;
        if !result.status.success() {
            return Err(Error::Git {
                stdout: String::from_utf8_lossy(&result.stdout).into(),
                stderr: String::from_utf8_lossy(&result.stderr).into(),
            });
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &std::path::Path {
        self.path.as_ref()
    }
}
