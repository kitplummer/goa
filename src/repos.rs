use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result, Write};
use std::thread;
use std::time::Duration;

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;
use uuid::Uuid;
use url::Url;

use crate::git;

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub status: String,
    pub local_path: String,
}

impl Repo {
    pub fn new(url: String, local_path: String) -> Repo {
        // We'll initialize after the clone is successful.
        let status = String::from("cloned");
        Repo { url, status, local_path }
    }

    // pub fn contain_goa_file() -> bool {
    //     false
    // }
}

pub fn spy_repo(url: String, branch: String, delay: u16, username: Option<String>, token: Option<String>) -> Result<()> {

    let parsed_url = Url::parse(&url);

    match parsed_url {
        Ok(mut parsed_url) => {

            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) { panic!("Error: {:?}", e) };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) { panic!("Error: {:?}", e) };
            }

            // Get a temp directory to do work in
            let temp_dir = temp_dir();
            let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
            local_path.push_str("/goa_wd/");

            let tmp_dir_name = format!("{}", Uuid::new_v4());
            local_path.push_str(&String::from(tmp_dir_name));

            // TODO: investigate shallow clone here
            let cloned_repo = match Repository:: clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo = Repo::new(String::from(parsed_url.as_str()), String::from(cloned_repo.path().to_str().unwrap()));
            println!("Spying repo {:?} at {:?}", url, repo.local_path);

            // This is where the loop happens...
            // For thread safety, we're going to have to simply pass the repo struct through, and
            // recreate the Repository "object" in each thread.  Perhaps not most performant,
            // but only sane way to manage through the thread scheduler.
            spy_for_changes(repo, delay);

            Ok(())
        },
        Err(e) => Err(Error::new(ErrorKind::InvalidData, format!("Invalid URL {}", e))),
    }
}

pub fn do_process(repo: &Repo) -> Result<()> {
    println!("Checking for diffs!");

    // Get the real Repository
    let local_repo = match Repository::open(&repo.local_path) {
        Ok(local_repo) => local_repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    // Run a is_diff() check

    if git::is_diff(&local_repo, "origin", "git2") {
        println!("DIFF!!!");
    } else {
        println!("NO DIFF");
    }



    // If diff then do the thing!

    // If thing is successful do_merge()

    Ok(())
}

pub fn spy_for_changes(repo: Repo, delay: u16) {
    println!("Checking for changes every {} seconds", delay);

    // Create a new scheduler
    let mut scheduler = Scheduler::new();
    // Add some tasks to it
    scheduler.every(30.seconds()).run(move || 
        do_process(&repo).expect("Error: unable to attach to local repo.")
    );
    // Manually run the scheduler in an event loop
    loop {
        scheduler.run_pending();
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_spy_repo_with_good_url() {
        assert!(spy_repo(
            String::from("https://github.com/kitplummer/clikan"),
            String::from("branch"),
            120,
            Some(String::from("")),
            Some(String::from(""))
        ).is_ok());
    }

    #[test]
    fn test_spy_repo_with_bad_url() {
        let res = spy_repo(
            String::from("test"),
            String::from("branch"),
            120,
            Some(String::from("test")),
            Some(String::from("test"))
        ).map_err(|e| e.kind());

        assert_eq!(Err(ErrorKind::InvalidData), res);
    }

    // #[test]
    // fn test_contain_a_goa_file() {   
    //     assert!(true);
    // }
}