mod common;

#[test]
/// No `entomologist-data` branch, local or remote.
fn no_entomologist_data_branch_exists_rw() {
    let workdir = common::make_test_repo();
    std::env::set_current_dir(&workdir).unwrap();
    println!("{workdir:?}");

    let db = entomologist::database::make_issues_database(
        &entomologist::database::IssuesDatabaseSource::Branch("entomologist-data"),
        entomologist::database::IssuesDatabaseAccess::ReadWrite,
    )
    .unwrap();

    let _issues = entomologist::issues::Issues::new_from_dir(&db.dir).unwrap();
}
