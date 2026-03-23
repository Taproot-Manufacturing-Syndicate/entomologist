mod common;

#[test]
/// A remote repo exists.
/// The remote repo has no `entomologist-data` branch.
/// The local repo has its own `entomologist-data` branch.
fn yes_remote_no_remote_entomologist_data_yes_local_entomologist_data_rw() {
    let branch = "entomologist-data";

    let remote_repo = common::make_test_repo();
    std::env::set_current_dir(&remote_repo).unwrap();

    // Clone the "remote" repo into another temporary repo.
    let local_repo = common::clone_repo(&remote_repo.path());
    std::env::set_current_dir(&local_repo).unwrap();
    common::make_entomologist_branch();

    let issues = entomologist::IssuesMut::new_from_git(branch).unwrap();

    let remote = "origin";
    match entomologist::git::sync(&issues.path(), remote, branch) {
        Err(e) => {
            panic!("unexpected sync error: {e:?}");
        }
        Ok(_) => {
            // This should work.
            ()
        }
    }
}
