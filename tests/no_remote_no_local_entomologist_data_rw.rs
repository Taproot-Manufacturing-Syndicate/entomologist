mod common;

#[test]
/// No remote repo exists.
/// No local `entomologist-data` branch exists.
fn no_remote_no_local_entomologist_data_rw() {
    let repo = common::make_test_repo();
    std::env::set_current_dir(&repo).unwrap();

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadWrite,
    )
    .unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();
}
