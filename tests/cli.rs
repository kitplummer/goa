use assert_cmd::prelude::*; // Add methods on commands
//use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn test_spy_repo_command_bad_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("test");
    cmd.assert()
        //.failure()
        .stderr(predicates::str::contains("Error: Invalid URL"));
    Ok(())
}

// Gotta figure this one out, since it is a long-running proc

// #[test]
// fn test_spy_repo_command_good_url() -> Result<(), Box<dyn std::error::Error>> {
//     let mut cmd = Command::cargo_bin("goa")?;
//     cmd.arg("spy");
//     cmd.arg("https://github.com/gtri/lowendinsight");
//     cmd.assert()
//         .stdout(predicates::str::contains("Spying repo \"https://github.com/gtri/lowendinsight\""));
//     Ok(())
// }