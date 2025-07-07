use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error("Oops, something went wrong")]
    Oops,
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
        let _result = std::process::Command::new("git")
            .args(["worktree", "remove", &self.path.path().to_string_lossy()])
            .output();
    }
}

impl Worktree {
    pub fn new(branch: &str) -> Result<Worktree, GitError> {
        let path = tempfile::tempdir()?;
        let result = std::process::Command::new("git")
            .args(["worktree", "add", &path.path().to_string_lossy(), branch])
            .output()?;
        if !result.status.success() {
            println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
            println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
            return Err(GitError::Oops);
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &std::path::Path {
        self.path.as_ref()
    }
}

pub fn checkout_branch_in_worktree(
    branch: &str,
    worktree_dir: &std::path::Path,
) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["worktree", "add", &worktree_dir.to_string_lossy(), branch])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_worktree_prune() -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_remove_branch(branch: &str) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["branch", "-D", branch])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_branch_exists(branch: &str) -> Result<bool, GitError> {
    let result = std::process::Command::new("git")
        .args(["show-ref", "--quiet", branch])
        .output()?;
    return Ok(result.status.success());
}

pub fn git_commit_file(file: &std::path::Path) -> Result<(), GitError> {
    let mut git_dir = std::path::PathBuf::from(file);
    git_dir.pop();

    let result = std::process::Command::new("git")
        .args(["add", &file.file_name().unwrap().to_string_lossy()])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    let result = std::process::Command::new("git")
        .args([
            "commit",
            "-m",
            &format!(
                "update '{}' in issue {}",
                file.file_name().unwrap().to_string_lossy(),
                git_dir.file_name().unwrap().to_string_lossy()
            ),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    Ok(())
}

pub fn create_orphan_branch(branch: &str) -> Result<(), GitError> {
    {
        let tmp_worktree = tempfile::tempdir().unwrap();
        create_orphan_branch_at_path(branch, tmp_worktree.path())?;
    }
    // The temp dir is now removed / cleaned up.

    let result = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    Ok(())
}

fn create_orphan_branch_at_path(
    branch: &str,
    worktree_path: &std::path::Path,
) -> Result<(), GitError> {
    let worktree_dir = worktree_path.to_string_lossy();
    let result = std::process::Command::new("git")
        .args(["worktree", "add", "--orphan", "-b", branch, &worktree_dir])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
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
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    let result = std::process::Command::new("git")
        .args(["commit", "-m", "create entomologist issue branch"])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree() {
        let mut p = std::path::PathBuf::new();
        {
            let worktree = Worktree::new("origin/main").unwrap();

            p.push(worktree.path());
            assert!(p.exists());

            let mut p2 = p.clone();
            p2.push("README.md");
            assert!(p2.exists());
        }
        // The temporary worktree directory is removed when the Temp variable is dropped.
        assert!(!p.exists());
    }

    #[test]
    fn test_create_orphan_branch() {
        let rnd: u128 = rand::random();
        let mut branch = std::string::String::from("entomologist-test-branch-");
        branch.push_str(&format!("{:0x}", rnd));
        create_orphan_branch(&branch).unwrap();
        git_remove_branch(&branch).unwrap();
    }

    #[test]
    fn test_branch_exists_0() {
        let r = git_branch_exists("main").unwrap();
        assert_eq!(r, true);
    }

    #[test]
    fn test_branch_exists_1() {
        let rnd: u128 = rand::random();
        let mut branch = std::string::String::from("entomologist-missing-branch-");
        branch.push_str(&format!("{:0x}", rnd));
        let r = git_branch_exists(&branch).unwrap();
        assert_eq!(r, false);
    }
}
