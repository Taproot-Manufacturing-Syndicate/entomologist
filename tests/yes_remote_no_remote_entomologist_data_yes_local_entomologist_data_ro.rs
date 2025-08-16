mod common;

#[test]
/// A remote repo exists.
/// The remote repo has no `entomologist-data` branch.
/// The local repo has its own `entomologist-data` branch.
fn yes_remote_no_remote_entomolgist_data_yes_local_entomologist_data_ro() {
    // Make a temporary repo with an `entomologist-data` branch in it.
    let remote_repo = common::make_test_repo();
    std::env::set_current_dir(&remote_repo).unwrap();

    // Clone the "remote" repo into another temporary repo.
    let local_repo = common::clone_repo(&remote_repo.path());
    std::env::set_current_dir(&local_repo).unwrap();
    common::make_entomologist_branch();

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadOnly,
    )
    .unwrap();

    // Make a local issue.
    entomologist::issue::Issue::new(&db.dir, &Some(String::from("issue created locally"))).unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();
}
