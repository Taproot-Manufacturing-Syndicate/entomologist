mod common;

#[test]
/// No remote repo exists.
/// A local `entomologist-data` branch exists.
fn no_remote_yes_local_entomologist_data_ro() {
    let repo = common::make_test_repo();
    std::env::set_current_dir(&repo).unwrap();
    common::make_entomologist_branch();
    let _issues = entomologist::issues::Issues::new_from_git("entomologist-data").unwrap();
}
