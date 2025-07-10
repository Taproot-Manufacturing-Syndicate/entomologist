use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
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
                println!("failed to run git: {:#?}", e);
            }
            Ok(result) => {
                if !result.status.success() {
                    println!("failed to remove git worktree: {:#?}", result);
                }
            }
        }
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

pub fn worktree_is_dirty(dir: &str) -> Result<bool, GitError> {
    // `git status --porcelain` prints a terse list of files added or
    // modified (both staged and not), and new untracked files.  So if
    // says *anything at all* it means the worktree is dirty.
    let result = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(dir)
        .output()?;
    return Ok(result.stdout.len() > 0);
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

pub fn git_fetch(dir: &std::path::Path, remote: &str) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["fetch", remote])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn sync(dir: &std::path::Path, remote: &str, branch: &str) -> Result<(), GitError> {
    // We do all the work in a directory that's (FIXME) hopefully a
    // worktree.  If anything goes wrong we just fail out and ask the
    // human to fix it by hand :-/
    // 1. `git fetch`
    // 2. `git merge REMOTE/BRANCH`
    // 3. `git push REMOTE BRANCH`

    git_fetch(dir, remote)?;

    // Merge remote branch into local.
    let result = std::process::Command::new("git")
        .args(["merge", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!(
            "Sync failed!  Merge error!  Help, a human needs to fix the mess in {:?}",
            dir
        );
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    // Push merged branch to remote.
    let result = std::process::Command::new("git")
        .args(["push", remote, branch])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!(
            "Sync failed!  Push error!  Help, a human needs to fix the mess in {:?}",
            dir
        );
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }

    Ok(())
}

pub fn git_log_oldest_timestamp(
    path: &std::path::Path,
) -> Result<chrono::DateTime<chrono::Local>, GitError> {
    let mut git_dir = std::path::PathBuf::from(path);
    git_dir.pop();
    let result = std::process::Command::new("git")
        .args([
            "log",
            "--pretty=format:%at",
            "--",
            &path.file_name().unwrap().to_string_lossy(),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    let timestamp_str = std::str::from_utf8(&result.stdout).unwrap();
    let timestamp_last = timestamp_str.split("\n").last().unwrap();
    let timestamp_i64 = timestamp_last.parse::<i64>()?;
    let timestamp = chrono::DateTime::from_timestamp(timestamp_i64, 0)
        .unwrap()
        .with_timezone(&chrono::Local);
    Ok(timestamp)
}

pub fn git_log_oldest_author(path: &std::path::Path) -> Result<String, GitError> {
    let mut git_dir = std::path::PathBuf::from(path);
    git_dir.pop();
    let result = std::process::Command::new("git")
        .args([
            "log",
            "--pretty=format:%an <%ae>",
            "--",
            &path.file_name().unwrap().to_string_lossy(),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        println!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
        return Err(GitError::Oops);
    }
    let author_str = std::str::from_utf8(&result.stdout).unwrap();
    let author_last = author_str.split("\n").last().unwrap();
    Ok(String::from(author_last))
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
        branch.push_str(&format!("{:032x}", rnd));
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
        branch.push_str(&format!("{:032x}", rnd));
        let r = git_branch_exists(&branch).unwrap();
        assert_eq!(r, false);
    }
}
