use std::io::Result;
use std::ops::DerefMut;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// For datetime/timestamp/log
use chrono::Utc;

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;

use crate::git;

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub status: String,
    pub local_path: String,
    pub branch: String,
    pub command: String,
    pub delay: u16,
    pub verbosity: u8,
}

impl Repo {
    pub fn new(
        url: String,
        local_path: String,
        branch: String,
        command: String,
        delay: u16,
        verbosity: u8,
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
            verbosity,
        }
    }

    pub fn spy_for_changes(&self) {
        let dt = Utc::now();
        println!(
            "goa [{}]: checking for diffs every {} seconds",
            dt, self.delay
        );

        // Create a new scheduler
        let mut scheduler = Scheduler::new();
        let delay = self.delay as u32;
        let cloned_repo = Arc::new(Mutex::new(self.clone()));
        // Add some tasks to it
        scheduler.every(delay.seconds()).run(move || {
            let mut mut_repo = cloned_repo.lock().unwrap();
            do_process(mut_repo.deref_mut()).expect("Error: unable to attach to local repo.")
        });
        // Manually run the scheduler in an event loop
        loop {
            scheduler.run_pending();
            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub fn do_process(repo: &mut Repo) -> Result<()> {
    // Get the real Repository
    let dt = Utc::now();
    let local_repo = match Repository::open(&repo.local_path) {
        Ok(local_repo) => local_repo,
        Err(_e) => {
            eprintln!("goa [{}]: failed to open the cloned repo", dt);
            std::process::exit(1); 
        }
    };

    if repo.verbosity > 1 {
        println!(
            "goa [{}]: checking for diffs at origin/{}!",
            dt, repo.branch
        );
    }

    match git::is_diff(&local_repo, "origin", &repo.branch.to_string()) {
        Ok(commit) => {
            // TODO - think this needs to merge first, to get the update
            // from the .goa file.
            match git::do_merge(&local_repo, &repo.branch, commit) {
                Ok(()) => {
                    match do_task(repo) {
                        Ok(output) => {
                            let dt = Utc::now();
                            println!("goa [{}]: {}", dt, output);

                        },
                        Err(e) => {
                            let dt = Utc::now();
                            eprintln!("goa [{}]: {}", dt, e);
                        }
                    }
                },
                Err(e) => {
                    let dt = Utc::now();
                    eprintln!("goa [{}]: {}", dt, e);
                }
            }
        }
        Err(e) => {
            if repo.verbosity > 1 {
                let dt = Utc::now();
                println!("goa [{}]: {}", dt, e);
            }
        }
    }

    Ok(())
}

fn do_task(repo: &mut Repo) -> Result<String> {
    let command: Vec<&str> = repo.command.split(" ").collect();
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

    if repo.verbosity > 1 {
        println!(
            "goa [{}]: running -> {} with args {:?}",
            dt, command_command, command_args
        );
    }

    let output = Command::new(command_command)
        .current_dir(&repo.local_path)
        .args(command_args)
        .output()
        .expect("goa: Error -> failed to execute command");


    let dt = Utc::now();
    if repo.verbosity > 2 {
        println!("goa debug: path -> {}", &repo.local_path);
        println!("goa: [{}]: command status: {}", dt, output.status);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);

    if repo.verbosity > 1 {
        println!(
            "goa: [{}]: command stderr:\n{}",
            dt,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(stdout.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation_of_repo() {
        let repo = Repo::new(
            String::from("file://."),
            String::from("."),
            String::from("develop"),
            String::from("ls -l"),
            120,
            1,
        );

        assert_eq!("develop", repo.branch);
    }

    #[test]
    fn test_do_task() {
        let mut repo = Repo::new(
            String::from("file://."),
            String::from("."),
            String::from("develop"),
            String::from("echo hello"),
            120,
            1,
        );

        let res = do_task(&mut repo);
        assert_eq!(String::from("hello\n"), res.unwrap());

    }
}
