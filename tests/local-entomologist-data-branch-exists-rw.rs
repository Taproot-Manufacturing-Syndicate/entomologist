mod common;

#[test]
fn local_entomologist_data_branch_exists_rw() {
    let repo = common::make_test_repo();
    std::env::set_current_dir(&repo).unwrap();
    common::make_entomologist_branch(&repo.path());

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadWrite,
    )
    .unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();
}
