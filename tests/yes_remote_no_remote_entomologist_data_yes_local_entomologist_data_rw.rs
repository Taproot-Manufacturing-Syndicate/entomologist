mod common;

#[test]
/// A remote repo exists.
/// The remote repo has an `entomologist-data` branch.
/// The local repo has its own `entomologist-data` branch.
fn yes_remote_no_remote_entomologist_data_yes_local_entomologist_data_rw() {
    let remote_repo = common::make_test_repo();
    std::env::set_current_dir(&remote_repo).unwrap();

    // Clone the "remote" repo into another temporary repo.
    let local_repo = common::clone_repo(&remote_repo.path());
    std::env::set_current_dir(&local_repo).unwrap();
    common::make_entomologist_branch();

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadWrite,
    )
    .unwrap();

    // Make a local issue.
    entomologist::issue::Issue::new(&db.dir, &Some(String::from("issue created locally"))).unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();

    let remote = "origin";
    let branch = "entomologist-data";
    match entomologist::git::sync(&db.dir, remote, branch) {
        Err(e) => {
            panic!("unexpected sync error: {e:?}");
        }
        Ok(_) => {
            // This should work.
            ()
        }
    }
}
