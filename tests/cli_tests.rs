use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help_menu() {
    let mut cmd = Command::cargo_bin("mcvt").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("MCVT acts as a smart layer"));
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

#[test]
fn test_cli_skip_existing_single_file() {
    let temp_dir = std::env::temp_dir().join("mcvt_test_skip_existing_single");
    let _ = std::fs::create_dir_all(&temp_dir);

    let in_file = temp_dir.join("input.txt");
    let out_file = temp_dir.join("output.txt");

    std::fs::write(&in_file, "hello").unwrap();
    std::fs::write(&out_file, "existing").unwrap();

    let mut cmd = Command::cargo_bin("mcvt").unwrap();
    cmd.arg(&in_file)
        .arg(&out_file)
        .arg("--skip-existing")
        .assert()
        .success()
        .stdout(predicate::str::contains("Skipping existing output file"));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_skip_existing_batch_mode() {
    let temp_dir = std::env::temp_dir().join("mcvt_test_skip_existing_batch");
    let in_dir = temp_dir.join("in");
    let out_dir = temp_dir.join("out");

    let _ = std::fs::create_dir_all(&in_dir);
    let _ = std::fs::create_dir_all(&out_dir);

    let file1 = in_dir.join("file1.txt");
    let file2 = in_dir.join("file2.txt");
    let existing_out1 = out_dir.join("file1.mkv");

    std::fs::write(&file1, "file 1 content").unwrap();
    std::fs::write(&file2, "file 2 content").unwrap();
    std::fs::write(&existing_out1, "already converted").unwrap();

    let mut cmd = Command::cargo_bin("mcvt").unwrap();
    cmd.arg(&in_dir)
        .arg(&out_dir)
        .arg("--batch-ext")
        .arg("mkv")
        .arg("--skip-existing")
        .assert()
        .stdout(predicate::str::contains("Skipping existing output file: [file1.txt]"));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_custom_config_flag() {
    let temp_dir = std::env::temp_dir().join("mcvt_test_custom_config");
    let _ = std::fs::create_dir_all(&temp_dir);

    let cfg_file = temp_dir.join("custom.toml");
    let in_file = temp_dir.join("test.txt");
    let out_file = temp_dir.join("test_out.txt");

    std::fs::write(
        &cfg_file,
        r#"
        [ffmpeg.output_defaults]
        txt = ["-custom", "arg"]
        "#,
    )
    .unwrap();
    std::fs::write(&in_file, "content").unwrap();
    std::fs::write(&out_file, "existing").unwrap();

    let mut cmd = Command::cargo_bin("mcvt").unwrap();
    cmd.arg(&in_file)
        .arg(&out_file)
        .arg("--config")
        .arg(&cfg_file)
        .arg("--skip-existing")
        .assert()
        .success();

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}
