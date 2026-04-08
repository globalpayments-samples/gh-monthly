use assert_cmd::Command;

#[test]
fn shows_top_level_help() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
