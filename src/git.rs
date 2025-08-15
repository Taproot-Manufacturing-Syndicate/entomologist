use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Failed to fetch from remote {remote:?}:\n{error}")]
    FetchError { remote: String, error: String },
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
    pub fn new(branch: &str) -> Result<Worktree, GitError> {
        let path = tempfile::tempdir()?;
        let result = std::process::Command::new("git")
            .args(["worktree", "add", &path.path().to_string_lossy(), branch])
            .output()?;
        if !result.status.success() {
            println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
            println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
            return Err(GitError::Oops);
        }
        Ok(Self { path })
    }

    pub fn new_detached(branch: &str) -> Result<Worktree, GitError> {
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
            println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
            println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
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
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_worktree_prune() -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_remove_branch(branch: &str) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["branch", "-D", branch])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_branch_exists(branch: &str) -> Result<bool, GitError> {
    let result = std::process::Command::new("git")
        .args(["show-ref", "--quiet", branch])
        .output()?;
    Ok(result.status.success())
}

pub fn ensure_branch_exists(branch: &str) -> Result<(), GitError> {
    // Check for a local branch with the specified name.
    if git_branch_exists(&format!("refs/heads/{branch}"))? {
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
            let line = output.split('\n').next().ok_or(GitError::Oops)?;
            let remote_branch = line.split_whitespace().last().ok_or(GitError::Oops)?;

            let result = std::process::Command::new("git")
                .args(["branch", branch, remote_branch])
                .output()?;
            if !result.status.success() {
                println!("failed to make branch {branch:?} from remote branch {remote_branch:?}");
                println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
                println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
                return Err(GitError::Oops);
            }
        }
        false => {
            // No remote has this branch, make an empty one locally now.
            create_orphan_branch(branch)?;
        }
    }

    Ok(())
}

pub fn worktree_is_dirty(dir: &str) -> Result<bool, GitError> {
    // `git status --porcelain` prints a terse list of files added or
    // modified (both staged and not), and new untracked files.  So if
    // says *anything at all* it means the worktree is dirty.
    let result = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(dir)
        .output()?;
    Ok(!result.stdout.is_empty())
}

pub fn add(file: &std::path::Path) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["add", &file.to_string_lossy()])
        .current_dir(
            file.parent()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
        )
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn restore_file(file: &std::path::Path) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["restore", &file.to_string_lossy()])
        .current_dir(
            file.parent()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
        )
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn commit(dir: &std::path::Path, msg: &str) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    Ok(())
}

pub fn git_commit_file(file: &std::path::Path) -> Result<(), GitError> {
    let mut git_dir = std::path::PathBuf::from(file);
    git_dir.pop();

    let result = std::process::Command::new("git")
        .args([
            "add",
            &file
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    let result = std::process::Command::new("git")
        .args([
            "commit",
            "-m",
            &format!(
                "update '{}' in issue {}",
                file.file_name()
                    .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                    .to_string_lossy(),
                git_dir
                    .file_name()
                    .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                    .to_string_lossy()
            ),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    Ok(())
}

fn fetch(dir: &std::path::Path, remote: &str) -> Result<(), GitError> {
    let result = std::process::Command::new("git")
        .args(["fetch", remote])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        return Err(GitError::FetchError {
            remote: String::from(remote),
            error: String::from_utf8_lossy(&result.stderr).into_owned(),
        });
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

    fetch(dir, remote)?;

    // FIXME: Possible things to add:
    // * `git log -p` shows diff
    // * `git log --numstat` shows machine-readable diffstat

    // Show what we just fetched from the remote.
    let result = std::process::Command::new("git")
        .args([
            "log",
            "--no-merges",
            "--pretty=format:%an: %s",
            &format!("{}/{}", remote, branch),
            &format!("^{}", branch),
        ])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!(
            "Sync failed!  'git log' error!  Help, a human needs to fix the mess in {:?}",
            branch
        );
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    if !result.stdout.is_empty() {
        println!("Changes fetched from remote {}:", remote);
        println!("{}", &String::from_utf8_lossy(&result.stdout));
        println!();
    }

    // Show what we are about to push to the remote.
    let result = std::process::Command::new("git")
        .args([
            "log",
            "--no-merges",
            "--pretty=format:%an: %s",
            branch,
            &format!("^{}/{}", remote, branch),
        ])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!(
            "Sync failed!  'git log' error!  Help, a human needs to fix the mess in {:?}",
            branch
        );
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }
    if !result.stdout.is_empty() {
        println!("Changes to push to remote {}:", remote);
        println!("{}", &String::from_utf8_lossy(&result.stdout));
        println!();
    }

    // Merge remote branch into local.
    let result = std::process::Command::new("git")
        .args(["merge", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .output()?;
    if !result.status.success() {
        println!(
            "Sync failed!  Merge error!  Help, a human needs to fix the mess in {:?}",
            branch
        );
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
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
            branch
        );
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    Ok(())
}

pub fn git_log_oldest_author_timestamp(
    path: &std::path::Path,
) -> Result<(String, chrono::DateTime<chrono::Local>), GitError> {
    let mut git_dir = std::path::PathBuf::from(path);
    git_dir.pop();
    let result = std::process::Command::new("git")
        .args([
            "log",
            "--pretty=format:%at %an <%ae>",
            "--",
            &path
                .file_name()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string_lossy(),
        ])
        .current_dir(&git_dir)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    let raw_output_str = String::from_utf8_lossy(&result.stdout);
    let Some(raw_output_last) = raw_output_str.split("\n").last() else {
        return Err(GitError::Oops);
    };
    let Some(index) = raw_output_last.find(' ') else {
        return Err(GitError::Oops);
    };
    let author_str = &raw_output_last[index + 1..];
    let timestamp_str = &raw_output_last[0..index];
    let timestamp_i64 = timestamp_str.parse::<i64>()?;
    let timestamp = chrono::DateTime::from_timestamp(timestamp_i64, 0)
        .unwrap()
        .with_timezone(&chrono::Local);

    Ok((String::from(author_str), timestamp))
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
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    Ok(())
}

fn create_orphan_branch_at_path(
    branch: &str,
    worktree_path: &std::path::Path,
) -> Result<(), GitError> {
    let worktree_dir = worktree_path.to_string_lossy();

    // Create a worktree at the path, with a detached head.
    let result = std::process::Command::new("git")
        .args(["worktree", "add", &worktree_dir, "HEAD"])
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    // Create an empty orphan branch in the worktree.
    let result = std::process::Command::new("git")
        .args(["switch", "--orphan", branch])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
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
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    let result = std::process::Command::new("git")
        .args(["commit", "-m", "create entomologist issue branch"])
        .current_dir(worktree_path)
        .output()?;
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        return Err(GitError::Oops);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
        // FIXME: I'm not super happy with this, for a couple of reasons:
        //
        // * A user might have cloned the repo with a different remote
        //   name than "origin".
        //
        // * In Github workflow's "actions/checkout" by default
        //   only the branch being built gets fetched, which means
        //   'origin/main' usually doesn't exist.  We fix this by
        //   setting "actions/checkout" fetch-depth to 0, which means
        //   fetch everything.
        //
        // This works for now but could be better.
        let r = git_branch_exists("origin/main").unwrap();
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
