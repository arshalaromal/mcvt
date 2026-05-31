use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help_menu() {
    let mut cmd = Command::cargo_bin("mcvt").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-Format Conversion Engine"));
}

#[test]
fn test_cli_invalid_force_flag() {
    let mut cmd = Command::cargo_bin("mcvt").unwrap();

    // Simulating a user passing garbage to the force flag
    cmd.arg("in.mp4")
        .arg("out.avi")
        .arg("--force")
        .arg("garbage:garbage")
        .assert()
        .failure() // We EXPECT this to fail
        .stderr(predicate::str::contains("Unknown domain"));
}

#[test]
fn test_cli_batch_missing_ext() {
    let mut cmd = Command::cargo_bin("mcvt").unwrap();

    // If we pass a directory but forget --batch-ext, the engine should catch it.
    // Note: Replace "src" with any directory that actually exists on your machine for the test.
    cmd.arg("src")
        .arg("output_dir")
        .assert()
        .failure()
        .stderr(predicate::str::contains("You must provide --batch-ext"));
}
