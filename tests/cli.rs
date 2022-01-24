use assert_cmd::prelude::*; // Add methods on commands
use git2::Repository;
use std::env::temp_dir;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command; // Run programs

use uuid::Uuid;

#[test]
fn test_spy_repo_command_bad_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("test");
    cmd.assert()
        //.failure()
        .stderr(predicates::str::contains("goa error: invalid URL or path"));
    Ok(())
}

#[test]
fn test_spy_repo_command_missing_auth() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("https://github.com/kitplummer/cliban");
    cmd.assert()
        //.failure()
        .stderr(predicates::str::contains(
            "goa error: failed to clone -> remote authentication required",
        ));
    Ok(())
}

#[test]
fn test_spy_repo_command_bad_auth() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("-u");
    cmd.arg("fred");
    cmd.arg("-t");
    cmd.arg("flintstone");
    cmd.arg("https://github.com/kitplummer/cliban");
    cmd.assert().stderr(predicates::str::contains(
        "goa error: failed to clone -> remote authentication required",
    ));
    Ok(())
}

#[test]
fn test_spy_repo_command_bad_command() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("https://github.com/kitplummer/goa_tester");
    cmd.arg("-e");
    cmd.arg("-c");
    cmd.arg("/notarealcommand");
    cmd.assert()
        .stderr(predicates::str::contains("/notarealcommand:"));
    cmd.assert().failure().code(127);
    Ok(())
}

#[test]
fn test_spy_repo_command_bad_command_in_goa_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("https://github.com/kitplummer/goa_tester_bad_command");
    cmd.arg("-e");
    cmd.arg("-v");
    cmd.arg("3");
    cmd.assert()
        .stderr(predicates::str::contains("/notarealcommand:"));
    cmd.assert().failure().code(127);
    Ok(())
}

#[test]
fn test_spy_repo_command_bad_branch() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("https://github.com/kitplummer/goa_tester");
    cmd.arg("-b");
    cmd.arg("blah");
    cmd.arg("-e");
    cmd.arg("-v");
    cmd.arg("3");
    cmd.assert()
        .stdout(predicates::str::contains(
            "starting to spy https://github.com/kitplummer/goa_tester",
        ))
        .stderr(predicates::str::contains("not found"));
    Ok(())
}

#[test]
fn test_spy_repo_command_good_start() -> Result<(), Box<dyn std::error::Error>> {
    // Create directory
    let temp_dir = temp_dir();
    let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
    let tmp_dir_name = format!("/{}/", Uuid::new_v4());
    local_path.push_str(&tmp_dir_name);
    let arg_path = local_path.clone();
    // Create .goa file
    let file_path = format!("{}.goa", local_path);
    // Git init and commit
    let repo = Repository::init(local_path).expect("Couldn't open repository");
    println!("REPO: {}", repo.path().display());
    let mut goa_file = File::create(file_path)?;
    goa_file.write_all(b"echo \"Hello")?;
    match add_and_commit(&repo, Path::new(".goa"), "test"){
        Ok(oid) => println!("OID: {}", oid),
        Err(e) => eprintln!("error: {}", e),
    }

    // Run against provided repo
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("-b");
    cmd.arg("master");
    cmd.arg("-d");
    cmd.arg("10");
    cmd.arg("-x");
    cmd.arg(format!("file://{}", arg_path));
    cmd.spawn()?;

    //cmd.assert()
    //    .stdout(predicates::str::contains("starting to spy"));
    let ten_millis = std::time::Duration::from_millis(1000);
    std::thread::sleep(ten_millis);

    goa_file.write_all(b", world!\"")?;

    match add_and_commit(&repo, Path::new(".goa"), "test2"){
        Ok(oid) => println!("OID: {}", oid),
        Err(e) => eprintln!("error: {}", e),
    }

    // cmd.assert()
    //     .stderr(predicates::str::contains("goa"));
    Ok(())
}

#[test]
fn test_spy_repo_command_catch_diff() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("spy");
    cmd.arg("https://github.com/kitplummer/goa_tester");
    cmd.arg("-b");
    cmd.arg("goa_integration_tests");
    cmd.arg("-e");
    cmd.arg("-v");
    cmd.arg("3");
    cmd.assert().stdout(predicates::str::contains(
        "starting to spy https://github.com/kitplummer/goa_tester",
    ));
    Ok(())
}

#[test]
fn test_help_command() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("goa")?;
    cmd.arg("help");
    cmd.assert().stdout(predicates::str::contains(
        "A command-line GitOps utility agent",
    ));
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

use git2::{Oid, Signature};
use std::path::Path;

use git2::{Commit, ObjectType};

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

fn add_and_commit(repo: &Repository, path: &Path, message: &str) -> Result<Oid, git2::Error> {
    let mut index = repo.index()?;
    index.add_path(path)?;
    let oid = index.write_tree()?;
    let signature = Signature::now("Kit Plummer", "kitplummer@gmail.com")?;
    match find_last_commit(&repo) {
        Ok(parent_commit) => {
            let tree = repo.find_tree(oid)?;
            repo.commit(
                Some("HEAD"), //  point HEAD to our new commit
                &signature,   // author
                &signature,   // committer
                message,      // commit message
                &tree,        // tree
                &[&parent_commit],
            ) // parents
        }
        Err(_e) => {
            let tree = repo.find_tree(oid)?;
            repo.commit(
                Some("HEAD"), //  point HEAD to our new commit
                &signature,   // author
                &signature,   // committer
                message,      // commit message
                &tree,        // tree
                &[],
            ) // parents
        }
    }
}
