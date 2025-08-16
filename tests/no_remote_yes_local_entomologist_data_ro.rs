mod common;

#[test]
/// No remote repo exists.
/// A local `entomologist-data` branch exists.
fn no_remote_yes_local_entomologist_data_ro() {
    let repo = common::make_test_repo();
    std::env::set_current_dir(&repo).unwrap();
    common::make_entomologist_branch();

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadOnly,
    )
    .unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();
}
