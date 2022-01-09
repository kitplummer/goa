use std::io::{Error, ErrorKind, Result};
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
    pub exec_on_start: bool,
}

impl Repo {
    pub fn new(
        url: String,
        local_path: String,
        branch: String,
        command: String,
        delay: u16,
        verbosity: u8,
        exec_on_start: bool,
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
            exec_on_start,
        }
    }

    pub fn clone_repo(&self) {
        match Repository::clone(self.url.as_str(), &self.local_path) {
            Ok(_repo) => {
                let dt = Utc::now();
                println!("goa [{}]: cloned remote repo to {}", dt, self.local_path);
            }
            Err(e) => {
                eprintln!(
                    "goa error: failed to clone -> {}",
                    e
                );
                std::process::exit(1);
            }
        };
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
        if self.exec_on_start {
            if self.command.is_empty() {
                if self.verbosity > 2 {
                    println!("goa debug: .goa file command {}", self.command);
                }

                let mut mut_repo = cloned_repo.lock().unwrap();
                mut_repo.command = read_goa_file(format!("{}/.goa", self.local_path));
                match do_task(mut_repo.deref_mut()) {
                    Ok(output) => {
                        let dt = Utc::now();
                        println!("goa [{}]: {}", dt, output);
                    }
                    Err(e) => {
                        let dt = Utc::now();
                        eprintln!("goa [{}]: do_task error {}", dt, e);
                    }
                }
            } else {
                let mut mut_repo = cloned_repo.lock().unwrap();
                match do_task(mut_repo.deref_mut()) {
                    Ok(output) => {
                        let dt = Utc::now();
                        println!("goa [{}]: {}", dt, output);
                    }
                    Err(e) => {
                        let dt = Utc::now();
                        eprintln!("goa [{}]: do_task error {}", dt, e);
                    }
                }
            }
        }

        // Add the repo to scheduler
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

pub fn read_goa_file(goa_path: String) -> String {
    if std::path::Path::new(&goa_path).exists() {
        let dt = Utc::now();
        println!(
            "goa [{}]: reading command from .goa file at {}",
            dt, goa_path
        );
        std::fs::read_to_string(goa_path).unwrap()
    } else {
        let dt = Utc::now();
        println!(
            "goa [{}]: no command given, nor a .goa file found in the repo - will proceed",
            dt
        );
        String::from("echo 'no goa file yet'")
    }
}

pub fn do_process(repo: &mut Repo) -> Result<()> {
    // Get the real Repository
    let dt = Utc::now();
    let local_repo = match Repository::open(&repo.local_path) {
        Ok(local_repo) => local_repo,
        Err(e) => {
            eprintln!("goa [{}]: failed to open the cloned repo", dt);
            //std::process::exit(1);
            return Err(Error::new(ErrorKind::Other, e.to_string()));
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
                    if repo.command.is_empty() {
                        repo.command = read_goa_file(format!("{}/.goa", repo.local_path));
                        if repo.verbosity > 2 {
                            println!("goa debug: .goa file command {}", repo.command);
                        }
                    }
                    match do_task(repo) {
                        Ok(output) => {
                            let dt = Utc::now();
                            println!("goa [{}]: {}", dt, output);
                        }
                        Err(e) => {
                            let dt = Utc::now();
                            eprintln!("goa [{}]: do_task error {}", dt, e);
                        }
                    }

                    // Reset the .goa file command
                    repo.command = String::from("");
                }
                Err(e) => {
                    let dt = Utc::now();
                    eprintln!("goa [{}]: do_merge error {}", dt, e);
                }
            }
        }
        Err(e) => {
            if repo.verbosity > 1 {
                let dt = Utc::now();
                eprintln!("goa [{}]: {}", dt, e);
            }
        }
    }

    Ok(())
}

fn do_task(repo: &mut Repo) -> Result<String> {
    let command: Vec<&str> = repo.command.split(' ').collect();
    let dt = Utc::now();

    println!("goa [{}]: processing the command", dt);

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
        .expect("goa error: failed to execute command");

    let dt = Utc::now();
    if repo.verbosity > 2 {
        println!("goa debug: path -> {}", &repo.local_path);
        println!("goa [{}]: command status: {}", dt, output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if repo.verbosity > 1 {
        println!(
            "goa [{}]: command stderr:\n{}",
            dt,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(stdout.to_string())
}

#[cfg(test)]
mod repos_tests {
    use super::*;

    #[test]
    fn test_creation_of_repo() {
        let repo = Repo::new(
            String::from("file://."),
            String::from("."),
            String::from("develop"),
            String::from(""),
            120,
            1,
            false,
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
            3,
            false,
        );

        let res = do_task(&mut repo);
        assert_eq!(String::from("hello\n"), res.unwrap());
    }

    #[test]
    fn test_do_process() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("{}", uuid::Uuid::new_v4());
        local_path.push_str("/goa_wd/");
        local_path.push_str(&String::from(tmp_dir_name));
        let mut repo = Repo::new(
            String::from("https://github.com/kitplummer/goa_tester"),
            String::from(local_path),
            String::from("main"),
            String::from("echo hello"),
            120,
            2,
            false,
        );

        repo.clone_repo();

        assert_eq!(do_process(&mut repo)?, ());
        Ok(())
    }

    #[test]
    fn test_do_process_no_clone() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("{}", uuid::Uuid::new_v4());
        local_path.push_str("/goa_wd/");
        local_path.push_str(&String::from(tmp_dir_name));
        let mut repo = Repo::new(
            String::from("https://github.com/kitplummer/goa_tester"),
            String::from(local_path),
            String::from("main"),
            String::from("echo hello"),
            120,
            2,
            false,
        );

        repo.clone_repo();
        repo.local_path = String::from("/blahdyblahblah");
        let res = do_process(&mut repo).unwrap_err();
        assert_eq!(res.kind(), ErrorKind::Other);

        Ok(())
    }

    #[test]
    fn test_do_process_no_command() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("{}", uuid::Uuid::new_v4());
        local_path.push_str("/goa_wd/");
        local_path.push_str(&String::from(tmp_dir_name));
        let mut repo = Repo::new(
            String::from("https://github.com/kitplummer/goa_tester"),
            String::from(local_path),
            String::from("main"),
            String::from(""),
            120,
            3,
            false,
        );

        repo.clone_repo();

        assert_eq!(do_process(&mut repo)?, ());
        Ok(())
    }

    #[test]
    fn test_no_goa_file() {
        let res = read_goa_file(String::from("/blahdy/.goa"));
        assert_eq!(res, String::from("echo 'no goa file yet'"));
    }
}
