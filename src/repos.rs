use std::io::{Error, Read as IoRead, Result};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// For processing the command
use run_script::ScriptOptions;
use wait_timeout::ChildExt;

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::Repository;

use crate::git::{self, CommitMetadata};

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub username: Option<String>,
    pub token: Option<String>,
    #[allow(dead_code)] // TODO: Issue #11 - store run count and timestamps
    pub status: Option<String>,
    pub local_path: Option<String>,
    pub branch: String,
    pub command: String,
    pub delay: u16,
    pub verbosity: u8,
    pub exec_on_start: bool,
    pub exit_on_first_diff: bool,
    pub timeout: u64,
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
        exit_on_first_diff: bool,
        timeout: u64,
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
            exit_on_first_diff,
            timeout,
        }
    }

    pub fn clone_repo(&self) -> Result<()> {
        let local_path = self
            .local_path
            .as_ref()
            .ok_or_else(|| Error::other("local_path is not set"))?;

        // Some OS-specific non-sense with trailing / in paths
        let local_target = str::replace(local_path, "//", "/");
        match Repository::clone(self.url.as_str(), local_target) {
            Ok(_repo) => {
                if self.verbosity > 0 {
                    info!("cloned remote repo to {}", local_path);
                }
                Ok(())
            }
            Err(e) => {
                let msg = format!("goa error: failed to clone -> {}", e);
                Err(Error::other(msg))
            }
        }
    }

    pub fn spy_for_changes(&self) -> Result<()> {
        if self.verbosity > 0 {
            info!("checking for diffs every {} seconds", self.delay);
        }

        // Create a new scheduler
        let mut scheduler = Scheduler::new();
        let delay = self.delay as u32;
        let cloned_repo = Arc::new(Mutex::new(self.clone()));

        if self.exec_on_start {
            let mut mut_repo = cloned_repo
                .lock()
                .map_err(|e| Error::other(format!("Failed to acquire lock: {}", e)))?;
            match do_process_once(mut_repo.deref_mut()) {
                Ok(()) => {
                    if self.verbosity > 0 {
                        info!("exec on startup complete");
                    }
                }
                Err(e) => {
                    // exec_on_start failure is fatal - return the error
                    return Err(Error::other(format!("failed to exec on startup: {}", e)));
                }
            }
        }

        // Add the repo to scheduler
        scheduler.every(delay.seconds()).run(move || {
            match cloned_repo.lock() {
                Ok(mut mut_repo) => {
                    if let Err(e) = do_process(mut_repo.deref_mut()) {
                        eprintln!("goa error: unable to process repo: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("goa error: failed to acquire lock: {}", e);
                }
            }
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
        std::fs::read_to_string(goa_path).unwrap_or_else(|_| {
            String::from("echo 'failed to read .goa file'")
        })
    } else {
        String::from("echo 'no goa file found yet'")
    }
}

pub fn do_process_once(repo: &mut Repo) -> Result<()> {
    let local_path = repo
        .local_path
        .as_ref()
        .ok_or_else(|| Error::other("local_path is not set"))?;

    let local_repo = Repository::open(local_path).map_err(|e| {
        Error::other(format!("goa error: failed to open the cloned repo: {}", e))
    })?;

    // Get commit metadata (thread-safe, no global env var mutation)
    let metadata = git::get_last_commit_metadata(&local_repo, &repo.branch, repo.verbosity)
        .map_err(|e| Error::other(format!("branch '{}' not found: {}", repo.branch, e)))?;

    if repo.command.is_empty() {
        repo.command = read_goa_file(format!("{}/.goa", local_path));
        if repo.verbosity > 2 {
            debug!(".goa file command {}", repo.command);
        }
    }

    match do_task(repo, Some(&metadata)) {
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
    let local_path = repo
        .local_path
        .as_ref()
        .ok_or_else(|| Error::other("local_path is not set"))?
        .clone();

    let local_repo = Repository::open(&local_path).map_err(|e| {
        Error::other(format!("goa error: failed to open the cloned repo: {}", e))
    })?;

    if repo.verbosity > 1 {
        info!("checking for diffs at origin/{}!", repo.branch);
    }

    match git::is_diff(&local_repo, "origin", &repo.branch, repo.verbosity) {
        Ok(commit) => {
            match git::do_merge(&local_repo, &repo.branch, commit, repo.verbosity) {
                Ok(metadata) => {
                    if repo.command.is_empty() {
                        repo.command = read_goa_file(format!("{}/.goa", local_path));
                        if repo.verbosity > 2 {
                            debug!(".goa file command {}", repo.command);
                        }
                    }
                    match do_task(repo, Some(&metadata)) {
                        Ok(output) => {
                            if repo.verbosity > 0 {
                                info!("command stdout: {}", output);
                            } else {
                                println!("{output}");
                            }

                            if repo.exit_on_first_diff {
                                // Intentional exit after first diff processed
                                std::process::exit(0);
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

/// Execute a command with optional commit metadata passed as env vars to child process.
/// This is thread-safe as env vars are only set in the child process, not globally.
/// If timeout > 0, the command will be killed after the specified number of seconds.
fn do_task(repo: &mut Repo, metadata: Option<&CommitMetadata>) -> Result<String> {
    let local_path = repo
        .local_path
        .as_ref()
        .ok_or_else(|| Error::other("local_path is not set"))?;

    if repo.verbosity > 1 {
        info!("running -> {}", repo.command);
        if repo.timeout > 0 {
            info!("timeout -> {} seconds", repo.timeout);
        }
    }

    if repo.verbosity > 2 {
        debug!("path -> {}", local_path);
    }

    // Use timeout-based execution if timeout is set
    if repo.timeout > 0 {
        return do_task_with_timeout(repo, metadata, local_path);
    }

    // No timeout - use run_script for simpler execution
    let mut options = ScriptOptions::new();
    options.working_directory = Some(PathBuf::from(local_path));

    // Pass commit metadata as env vars to child process only (thread-safe)
    if let Some(meta) = metadata {
        options.env_vars = Some(meta.to_env_vars());
    }

    let args = vec![];

    // run the script and get the script execution output
    let (code, output, error) = run_script::run(&repo.command, &args, &options)
        .map_err(|e| Error::other(format!("Failed to run script: {}", e)))?;

    if repo.verbosity > 1 {
        info!("command status: {}", code);
        info!("command stderr:\n{}", error);
    }

    if !error.is_empty() {
        eprintln!("{}", error);
        // Exit with the command's exit code - this is intentional behavior
        std::process::exit(code);
    }

    Ok(output)
}

/// Execute a command with a timeout. Kills the process if it exceeds the timeout.
fn do_task_with_timeout(
    repo: &Repo,
    metadata: Option<&CommitMetadata>,
    local_path: &str,
) -> Result<String> {
    let timeout_duration = Duration::from_secs(repo.timeout);

    // Build the command using sh -c for shell interpretation
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&repo.command)
        .current_dir(local_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Pass commit metadata as env vars
    if let Some(meta) = metadata {
        for (key, value) in meta.to_env_vars() {
            cmd.env(key, value);
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::other(format!("Failed to spawn command: {}", e)))?;

    // Wait with timeout
    match child.wait_timeout(timeout_duration) {
        Ok(Some(status)) => {
            // Process completed within timeout
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let Some(mut out) = child.stdout.take() {
                out.read_to_string(&mut stdout)
                    .map_err(|e| Error::other(format!("Failed to read stdout: {}", e)))?;
            }
            if let Some(mut err) = child.stderr.take() {
                err.read_to_string(&mut stderr)
                    .map_err(|e| Error::other(format!("Failed to read stderr: {}", e)))?;
            }

            let code = status.code().unwrap_or(-1);

            if repo.verbosity > 1 {
                info!("command status: {}", code);
                info!("command stderr:\n{}", stderr);
            }

            if !stderr.is_empty() {
                eprintln!("{}", stderr);
                // Exit with the command's exit code - this is intentional behavior
                std::process::exit(code);
            }

            Ok(stdout)
        }
        Ok(None) => {
            // Timeout expired - kill the process
            if repo.verbosity > 0 {
                warn!(
                    "Command timed out after {} seconds, killing process",
                    repo.timeout
                );
            }
            child
                .kill()
                .map_err(|e| Error::other(format!("Failed to kill timed-out process: {}", e)))?;
            child.wait().ok(); // Clean up zombie process

            Err(Error::other(format!(
                "Command timed out after {} seconds",
                repo.timeout
            )))
        }
        Err(e) => Err(Error::other(format!("Failed to wait on command: {}", e))),
    }
}

#[cfg(test)]
mod repos_tests {
    use super::*;
    use std::io::ErrorKind;

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
            false,
            0,
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
            false,
            0,
        );

        let res = do_task(&mut repo, None);
        assert_eq!(String::from("hello\n"), res.unwrap());
    }

    #[test]
    fn test_do_process() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", uuid::Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);
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
            false,
            0,
        );

        repo.clone_repo()?;

        assert_eq!(do_process(&mut repo)?, ());
        Ok(())
    }

    #[test]
    fn test_do_process_no_clone() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", uuid::Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);
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
            false,
            0,
        );

        repo.clone_repo()?;
        repo.local_path = Some(String::from("/blahdyblahblah"));
        let res = do_process(&mut repo).unwrap_err();
        assert_eq!(res.kind(), ErrorKind::Other);

        Ok(())
    }

    #[test]
    fn test_do_process_no_command() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", uuid::Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);
        println!("local_path: {:?}", local_path);
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
            false,
            0,
        );

        repo.clone_repo()?;

        assert_eq!(do_process(&mut repo)?, ());
        Ok(())
    }

    #[test]
    fn test_no_goa_file() {
        let res = read_goa_file(String::from("/blahdy/.goa"));
        assert_eq!(res, String::from("echo 'no goa file found yet'"));
    }

    #[test]
    fn test_do_task_with_timeout() {
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
            false,
            5, // 5 second timeout
        );

        let res = do_task(&mut repo, None);
        assert_eq!(String::from("hello\n"), res.unwrap());
    }

    #[test]
    fn test_do_task_timeout_exceeded() {
        let mut repo = Repo::new(
            String::from("file://."),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from("")),
            Some(String::from(".")),
            String::from("develop"),
            String::from("sleep 10"),
            120,
            1,
            false,
            false,
            1, // 1 second timeout - should fail
        );

        let res = do_task(&mut repo, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("timed out"));
    }
}
