/// Create a tempdir with automatic cleanup-on-drop, initialize a git
/// repo in it, and create a valid `master` branch.
pub fn make_test_repo() -> tempfile::TempDir {
    // Create tempdir.
    let workdir = tempfile::tempdir().unwrap();

    // Make a git repo in it.
    let result = std::process::Command::new("git")
        .args(["init", &workdir.path().to_string_lossy()])
        .output()
        .unwrap();
    if !result.status.success() {
        println!("failed to git init in {}", workdir.path().to_string_lossy());
        println!("stdout:\n{}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr:\n{}", &String::from_utf8_lossy(&result.stderr));
        panic!();
    }

    // Make an empty commit in the master branch so it's normal and valid.
    let result = std::process::Command::new("git")
        .args(["commit", "--allow-empty", "-m", "empty commit"])
        .current_dir(&workdir.path())
        .output()
        .unwrap();
    if !result.status.success() {
        println!(
            "failed to git commit in {}",
            workdir.path().to_string_lossy()
        );
        println!("stdout:\n{}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr:\n{}", &String::from_utf8_lossy(&result.stderr));
        panic!();
    }

    workdir
}

/// Create an `entomologist-data` branch in the repo that we're
/// currently in.
#[allow(dead_code)]
pub fn make_entomologist_branch() {
    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadWrite,
    )
    .unwrap();
    entomologist::issue::Issue::new(&db.dir, &Some(String::from("issue created on remote")))
        .unwrap();
}

#[allow(dead_code)]
pub fn clone_repo(remote_repo: &std::path::Path) -> tempfile::TempDir {
    let workdir = tempfile::tempdir().unwrap();

    let result = std::process::Command::new("git")
        .args([
            "clone",
            &remote_repo.to_string_lossy(),
            &workdir.path().to_string_lossy(),
        ])
        .output()
        .unwrap();
    if !result.status.success() {
        println!("stdout: {}", &String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", &String::from_utf8_lossy(&result.stderr));
        panic!(
            "failed to git clone in {}",
            workdir.path().to_string_lossy()
        );
    }

    workdir
}
