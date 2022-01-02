use std::io::Result;
use std::ops::DerefMut;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// For datetime/timestamp/log
use chrono::{Utc};

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;

use crate::git;

// TODO: replace all panic!s with exits

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub status: String,
    pub local_path: String,
    pub branch: String,
    pub command: String,
    pub delay: u16,
}

impl Repo {
    pub fn new(
        url: String,
        local_path: String,
        branch: String,
        command: String,
        delay: u16
    ) -> Repo {
        // We'll initialize after the clone is successful.
        let status = String::from("cloned");
        Repo {
            url,
            status,
            local_path,
            branch,
            command,
            delay,
        }
    }

    pub fn spy_for_changes(&self) {
        let dt = Utc::now();
        println!("goa [{}]: checking for changes every {} seconds", dt, self.delay);

        // Create a new scheduler
        let mut scheduler = Scheduler::new();
        let delay = self.delay as u32;
        let cloned_repo = Arc::new(Mutex::new(self.clone()));
        // Add some tasks to it
        scheduler
            .every(delay.seconds())
            .run(move || {
                self.do_process(&self.command).expect("Error: unable to attach to local repo.")
            });
        // Manually run the scheduler in an event loop
        loop {
            scheduler.run_pending();
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn do_process(&self, command: &String) -> Result<()> {
        // Get the real Repository
        let local_repo = match Repository::open(&self.local_path) {
            Ok(local_repo) => local_repo,
            Err(e) => panic!("failed to open: {}", e),
        };
        
        let dt = Utc::now();
        println!("goa [{}]: checking for diffs at origin/{}!", dt, self.branch);
        match git::is_diff(&local_repo, "origin", &self.branch.to_string()) {
            Ok(commit) => {
                // TODO - think this needs to merge first, to get the update
                // from the .goa file.
                match git::do_merge(&local_repo, &self.branch, commit) {
                Ok(()) => self.do_task(&command),
                Err(e) => {
                    let dt = Utc::now();
                    println!("goa [{}]: {}", dt, e);
                }

                }
            }
            Err(e) => {
                let dt = Utc::now();
                println!("goa [{}]: {}", dt, e);
            }
        }

        Ok(())
    }

    fn do_task(&self, command: &String) {
        let command: Vec<&str> = command.split(" ").collect();
        let dt = Utc::now();
        println!("goa [{}]: have a diff, processing the goa file", dt);

        let mut command_command = "";
        let mut command_args: Vec<&str> = [].to_vec();

        for (pos, e) in command.iter().enumerate() {
            if pos == 0 {
                command_command = e;
            } else {
                command_args.push(e);
            }
        }
        
        println!("goa [{}]: running -> {} with args {:?}", dt, command_command, command_args);
        let output = Command::new(command_command)
                        .current_dir(&self.local_path)
                        .args(command_args)
                        .output()
                        .expect("goa: Error -> failed to execute command");

        let dt = Utc::now();
        println!("goa debug: path -> {}", &self.local_path);
        println!("goa: [{}]: command status: {}", dt, output.status);
        println!("goa: [{}]: command stdout:\n{}", dt, String::from_utf8_lossy(&output.stdout));
        println!("goa: [{}]: command stderr:\n{}", dt, String::from_utf8_lossy(&output.stderr));
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
