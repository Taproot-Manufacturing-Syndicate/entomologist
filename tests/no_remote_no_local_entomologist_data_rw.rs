mod common;

#[test]
/// No remote repo exists.
/// No local `entomologist-data` branch exists.
fn no_remote_no_local_entomologist_data_rw() {
    let branch = "entomologist-data";

    let repo = common::make_test_repo();
    std::env::set_current_dir(&repo).unwrap();
    let issues = entomologist::IssuesMut::new_from_git(branch).unwrap();

    let remote = "origin";
    match entomologist::git::sync(&issues.path(), remote, branch) {
        Err(entomologist::git::GitError::FetchError { remote, error }) => {
            // This is the error we expect.
            println!("failed to sync from remote {remote:#?}:");
            println!("{}", &error);
        }
        Err(e) => {
            panic!("unexpected sync error: {e:?}");
        }
        Ok(_) => {
            panic!("unexpected sync success with remote {remote:#?}, branch {branch:#?}");
        }
    }
}
