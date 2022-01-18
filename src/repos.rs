use std::io::{Error, ErrorKind, Result};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// For processing the command
use run_script::ScriptOptions;

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;

use crate::git;

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub username: Option<String>,
    pub token: Option<String>,
    pub status: Option<String>,
    pub local_path: Option<String>,
    pub branch: String,
    pub command: String,
    pub delay: u16,
    pub verbosity: u8,
    pub exec_on_start: bool,
}

impl Repo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        url: String,
        username: Option<String>,
        token: Option<String>,
        status: Option<String>,
        local_path: Option<String>,
        branch: String,
        command: String,
        delay: u16,
        verbosity: u8,
        exec_on_start: bool,
    ) -> Repo {
        // We'll initialize after the clone is successful.
        Repo {
            url,
            username,
            token,
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
        match Repository::clone(self.url.as_str(), self.local_path.as_ref().unwrap()) {
            Ok(_repo) => {
                if self.verbosity > 0 {
                    info!(
                        "cloned remote repo to {}",
                        self.local_path.as_ref().unwrap()
                    );
                }
            }
            Err(e) => {
                eprintln!("goa error: failed to clone -> {}", e);
                std::process::exit(1);
            }
        };
    }

    pub fn spy_for_changes(&self) {
        if self.verbosity > 0 {
            info!("checking for diffs every {} seconds", self.delay);
        }

        // Create a new scheduler
        let mut scheduler = Scheduler::new();
        let delay = self.delay as u32;
        let cloned_repo = Arc::new(Mutex::new(self.clone()));
        if self.exec_on_start {
            let mut mut_repo = cloned_repo.lock().unwrap();
            match do_process_once(mut_repo.deref_mut()) {
                Ok(()) => {
                    if self.verbosity > 0 {
                        info!("exec on startup complete");
                    }
                }
                Err(_e) => {
                    eprintln!("goa error: failed to exec on startup");
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
        std::fs::read_to_string(goa_path).unwrap()
    } else {
        String::from("echo 'no goa file found yet'")
    }
}

pub fn do_process_once(repo: &mut Repo) -> Result<()> {
    let local_repo = match Repository::open(&repo.local_path.as_ref().unwrap()) {
        Ok(local_repo) => local_repo,
        Err(e) => {
            eprintln!("goa error: failed to open the cloned repo");
            //std::process::exit(1);
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
    };

    git::set_last_commit(&local_repo, &repo.branch.to_string(), repo.verbosity);

    if repo.command.is_empty() {
        repo.command = read_goa_file(format!("{}/.goa", repo.local_path.as_ref().unwrap()));
        if repo.verbosity > 2 {
            debug!(".goa file command {}", repo.command);
        }
    }
    match do_task(repo) {
        Ok(output) => {
            if repo.verbosity > 0 {
                info!("command stdout: {}", output);
            } else {
                println!("{output}");
            }
        }
        Err(e) => {
            eprintln!("goa error: do_task error {}", e);
        }
    }
    Ok(())
}

pub fn do_process(repo: &mut Repo) -> Result<()> {
    // Get the real Repository
    let local_repo = match Repository::open(&repo.local_path.as_ref().unwrap()) {
        Ok(local_repo) => local_repo,
        Err(e) => {
            eprintln!("goa error: failed to open the cloned repo");
            //std::process::exit(1);
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
    };

    if repo.verbosity > 1 {
        info!("checking for diffs at origin/{}!", repo.branch);
    }

    match git::is_diff(
        &local_repo,
        "origin",
        &repo.branch.to_string(),
        repo.verbosity,
    ) {
        Ok(commit) => {
            match git::do_merge(&local_repo, &repo.branch, commit, repo.verbosity) {
                Ok(()) => {
                    if repo.command.is_empty() {
                        repo.command =
                            read_goa_file(format!("{}/.goa", repo.local_path.as_ref().unwrap()));
                        if repo.verbosity > 2 {
                            debug!(".goa file command {}", repo.command);
                        }
                    }
                    match do_task(repo) {
                        Ok(output) => {
                            if repo.verbosity > 0 {
                                info!("command stdout: {}", output);
                            } else {
                                println!("{output}");
                            }
                        }
                        Err(e) => {
                            eprintln!("goa error: do_task error {}", e);
                        }
                    }

                    // Reset the .goa file command
                    repo.command = String::from("");
                }
                Err(e) => {
                    eprintln!("goa error: do_merge error {}", e);
                }
            }
        }
        Err(e) => {
            // There were no diffs, so we move right along
            if repo.verbosity > 1 {
                debug!("{}", e);
            }
        }
    }

    Ok(())
}

fn do_task(repo: &mut Repo) -> Result<String> {
    let command: Vec<&str> = repo.command.split(' ').collect();

    if repo.verbosity > 1 {
        info!("running -> {:?}", command);
    }
    let mut options = ScriptOptions::new();
    options.working_directory = Some(PathBuf::from(&repo.local_path.as_ref().unwrap()));

    let args = vec![];

    // run the script and get the script execution output
    let (code, output, error) = run_script::run(&repo.command, &args, &options).unwrap();

    if repo.verbosity > 2 {
        debug!("path -> {}", &repo.local_path.as_ref().unwrap());
    }

    if repo.verbosity > 1 {
        info!("command status: {}", code);
        info!("command stderr:\n{}", error);
    }

    if !error.is_empty() {
        eprintln!("goa error: {}", error);
        std::process::exit(code);
    }

    Ok(output)
}

#[cfg(test)]
mod repos_tests {
    use super::*;

    #[test]
    fn test_creation_of_repo() {
        let repo = Repo::new(
            String::from("file://."),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(".")),
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
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(".")),
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
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(local_path)),
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
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(local_path)),
            String::from("main"),
            String::from("echo hello"),
            120,
            2,
            false,
        );

        repo.clone_repo();
        repo.local_path = Some(String::from("/blahdyblahblah"));
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
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(local_path)),
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
        assert_eq!(res, String::from("echo 'no goa file found yet'"));
    }
}
