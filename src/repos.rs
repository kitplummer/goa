use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result};
use std::process::Command;
use std::thread;
use std::time::Duration;

// For datetime/timestamp/log
use chrono::{Utc};

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;
use url::Url;
use uuid::Uuid;

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
        Repo {
            url,
            status,
            local_path,
        }
    }

    // pub fn contain_goa_file() -> bool {
    //     false
    // }
}

pub fn spy_repo(
    url: String,
    branch: String,
    delay: u16,
    username: Option<String>,
    token: Option<String>,
    command: Option<String>,
) -> Result<()> {

    let dt = Utc::now();
    println!("goa [{}]: starting to spy {}:{}", dt, url, branch);
    let parsed_url = Url::parse(&url);

    match parsed_url {
        Ok(mut parsed_url) => {
            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) {
                    panic!("Error: {:?}", e)
                };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) {
                    panic!("Error: {:?}", e)
                };
            }

            // Get a temp directory to do work in
            let temp_dir = temp_dir();
            let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
            local_path.push_str("/goa_wd/");

            let tmp_dir_name = format!("{}", Uuid::new_v4());
            local_path.push_str(&String::from(tmp_dir_name));

            // TODO: investigate shallow clone here
            let cloned_repo = match Repository::clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo = Repo::new(
                String::from(parsed_url.as_str()),
                String::from(cloned_repo.path().to_str().unwrap()),
            );

            let command = match command {
                Some(command) => command,
                None => String::from(""),
            };

            // This is where the loop happens...
            // For thread safety, we're going to have to simply pass the repo struct through, and
            // recreate the Repository "object" in each thread.  Perhaps not most performant,
            // but only sane way to manage through the thread scheduler.
            spy_for_changes(repo, branch, delay, command);

            Ok(())
        }
        Err(e) => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Error: Invalid URL {}", e),
        )),
    }
}

pub fn do_process(repo: &Repo, branch: &String, command: &String) -> Result<()> {
    // Get the real Repository
    let local_repo = match Repository::open(&repo.local_path) {
        Ok(local_repo) => local_repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    
    let dt = Utc::now();
    println!("goa [{}]: checking for diffs at origin/{}!", dt, branch);
    match git::is_diff(&local_repo, "origin", &branch.to_string()) {
        Ok(commit) => {
            do_task(&command);
            let _ = git::do_merge(&local_repo, "git2", commit);
        }
        Err(e) => {
            let dt = Utc::now();
            println!("goa [{}]: {}", dt, e);
        }
    }

    Ok(())
}

fn do_task(command: &String) {

    let command: Vec<&str> = command.split(" ").collect();
    let dt = Utc::now();
    println!("goa [{}]: have a diff, processing the goa file", dt);
    println!("goa [{}]: running -> {:?}", dt, command);

    let mut command_command = "";
    let mut command_args: Vec<&str> = [].to_vec();

    for (pos, e) in command.iter().enumerate() {
        if pos == 1 {
            command_command = e;
        } else {
            command_args.push(e);
        }
    }
    
    let output = Command::new(command_command)
                    .args(command_args)
                    .output()
                    .expect("goa: Error -> failed to execute command");

    let dt = Utc::now();
    println!("goa: [{}]: status: {}", dt, output.status);

    println!("goa: [{}]: {}", dt, String::from_utf8_lossy(&output.stdout));



}

pub fn spy_for_changes(repo: Repo, branch: String, delay: u16, command: String) {
    let dt = Utc::now();
    println!("goa [{}]: checking for changes every {} seconds", dt, delay);

    // Create a new scheduler
    let mut scheduler = Scheduler::new();
    let delay = delay as u32;
    // Add some tasks to it
    scheduler
        .every(delay.seconds())
        .run(move || do_process(&repo, &branch, &command).expect("Error: unable to attach to local repo."));
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
    fn test_spy_repo_with_bad_url() {
        let res = spy_repo(
            String::from("test"),
            String::from("branch"),
            120,
            Some(String::from("test")),
            Some(String::from("test")),
            Some(String::from("test")),
        )
        .map_err(|e| e.kind());

        assert_eq!(Err(ErrorKind::InvalidData), res);
    }

    // #[test]
    // fn test_contain_a_goa_file() {
    //     assert!(true);
    // }
}
