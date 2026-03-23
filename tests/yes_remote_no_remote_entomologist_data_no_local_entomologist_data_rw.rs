mod common;

#[test]
/// No local `entomologist-data` branch exists.
/// A remote repo exists.
/// The remote repo does not have an `entomologist-data` branch.
fn yes_remote_no_remote_entomologist_data_no_local_entomologist_data_rw() {
    let branch = "entomologist-data";

    // Make a temporary repo with an `entomologist-data` branch in it.
    let remote_repo = common::make_test_repo();
    std::env::set_current_dir(&remote_repo).unwrap();

    // Clone the "remote" repo into another temporary repo.
    let local_repo = common::clone_repo(&remote_repo.path());
    std::env::set_current_dir(&local_repo).unwrap();

    // This creates a local entomologist-data branch, with no issues
    // in it.
    let issues = entomologist::IssuesMut::new_from_git(branch).unwrap();

    let remote = "origin";
    match entomologist::git::sync(&issues.path(), remote, branch) {
        Err(e) => {
            panic!("{e}");
        }
        Ok(_) => {
            // This should work.
            ()
        }
    }
}
