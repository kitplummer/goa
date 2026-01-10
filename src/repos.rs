use std::io::{Error, Read as IoRead, Result};
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

/// Repository configuration for monitoring a git repository.
/// Use `RepoBuilder` to construct instances.
#[derive(Debug, Clone)]
pub struct Repo {
    url: String,
    username: Option<String>,
    token: Option<String>,
    local_path: Option<String>,
    branch: String,
    command: Option<String>,
    delay: u16,
    verbosity: u8,
    exec_on_start: bool,
    exit_on_first_diff: bool,
    timeout: u64,
}

impl Repo {
    /// Create a new builder for constructing a Repo
    pub fn builder(url: impl Into<String>) -> RepoBuilder {
        RepoBuilder::new(url)
    }

    // Getters for all fields
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn set_url(&mut self, url: String) {
        self.url = url;
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn local_path(&self) -> Option<&str> {
        self.local_path.as_deref()
    }

    pub fn set_local_path(&mut self, path: String) {
        self.local_path = Some(path);
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub fn delay(&self) -> u16 {
        self.delay
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    pub fn exec_on_start(&self) -> bool {
        self.exec_on_start
    }

    pub fn exit_on_first_diff(&self) -> bool {
        self.exit_on_first_diff
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }
}

/// Builder for constructing `Repo` instances with a fluent API.
#[derive(Debug, Clone)]
pub struct RepoBuilder {
    url: String,
    username: Option<String>,
    token: Option<String>,
    local_path: Option<String>,
    branch: String,
    command: Option<String>,
    delay: u16,
    verbosity: u8,
    exec_on_start: bool,
    exit_on_first_diff: bool,
    timeout: u64,
}

impl RepoBuilder {
    /// Create a new builder with the required URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            token: None,
            local_path: None,
            branch: String::from("main"),
            command: None,
            delay: 120,
            verbosity: 1,
            exec_on_start: false,
            exit_on_first_diff: false,
            timeout: 0,
        }
    }

    /// Set the username for authentication
    pub fn username(mut self, username: impl Into<String>) -> Self {
        let u = username.into();
        self.username = if u.is_empty() { None } else { Some(u) };
        self
    }

    /// Set the token for authentication
    pub fn token(mut self, token: impl Into<String>) -> Self {
        let t = token.into();
        self.token = if t.is_empty() { None } else { Some(t) };
        self
    }

    /// Set the local path for cloning
    pub fn local_path(mut self, path: impl Into<String>) -> Self {
        self.local_path = Some(path.into());
        self
    }

    /// Set the branch to monitor (default: "main")
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    /// Set the command to execute on changes (if not set, reads from .goa file)
    pub fn command(mut self, command: impl Into<String>) -> Self {
        let c = command.into();
        self.command = if c.is_empty() { None } else { Some(c) };
        self
    }

    /// Set the delay between checks in seconds (default: 120)
    pub fn delay(mut self, delay: u16) -> Self {
        self.delay = delay;
        self
    }

    /// Set the verbosity level (default: 1)
    pub fn verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Execute command on startup (default: false)
    pub fn exec_on_start(mut self, exec: bool) -> Self {
        self.exec_on_start = exec;
        self
    }

    /// Exit after first diff is processed (default: false)
    pub fn exit_on_first_diff(mut self, exit: bool) -> Self {
        self.exit_on_first_diff = exit;
        self
    }

    /// Set command timeout in seconds (0 = no timeout, default: 0)
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the Repo instance
    pub fn build(self) -> Repo {
        Repo {
            url: self.url,
            username: self.username,
            token: self.token,
            local_path: self.local_path,
            branch: self.branch,
            command: self.command,
            delay: self.delay,
            verbosity: self.verbosity,
            exec_on_start: self.exec_on_start,
            exit_on_first_diff: self.exit_on_first_diff,
            timeout: self.timeout,
        }
    }
}

impl Repo {
    pub fn clone_repo(&self) -> Result<()> {
        let local_path = self
            .local_path
            .as_ref()
            .ok_or_else(|| Error::other("local_path is not set"))?;

        // Some OS-specific non-sense with trailing / in paths
        let local_target = str::replace(local_path, "//", "/");
        match Repository::clone(&self.url, local_target) {
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
            let repo_guard = cloned_repo
                .lock()
                .map_err(|e| Error::other(format!("Failed to acquire lock: {}", e)))?;
            match do_process_once(&repo_guard) {
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
                Ok(repo_guard) => {
                    if let Err(e) = do_process(&repo_guard) {
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

pub fn read_goa_file(goa_path: &str) -> String {
    if std::path::Path::new(goa_path).exists() {
        std::fs::read_to_string(goa_path).unwrap_or_else(|_| {
            String::from("echo 'failed to read .goa file'")
        })
    } else {
        String::from("echo 'no goa file found yet'")
    }
}

/// Get the effective command to execute: use configured command or read from .goa file
fn get_effective_command(repo: &Repo, local_path: &str) -> String {
    match repo.command() {
        Some(cmd) => cmd.to_string(),
        None => read_goa_file(&format!("{}/.goa", local_path)),
    }
}

pub fn do_process_once(repo: &Repo) -> Result<()> {
    let local_path = repo
        .local_path()
        .ok_or_else(|| Error::other("local_path is not set"))?;

    let local_repo = Repository::open(local_path).map_err(|e| {
        Error::other(format!("goa error: failed to open the cloned repo: {}", e))
    })?;

    // Get commit metadata (thread-safe, no global env var mutation)
    let metadata = git::get_last_commit_metadata(&local_repo, repo.branch(), repo.verbosity())
        .map_err(|e| Error::other(format!("branch '{}' not found: {}", repo.branch(), e)))?;

    // Determine the effective command: use configured command or read from .goa file
    let effective_command = get_effective_command(repo, local_path);
    if repo.verbosity() > 2 {
        debug!("effective command: {}", effective_command);
    }

    match do_task(repo, &effective_command, Some(&metadata)) {
        Ok(output) => {
            if repo.verbosity() > 0 {
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

pub fn do_process(repo: &Repo) -> Result<()> {
    let local_path = repo
        .local_path()
        .ok_or_else(|| Error::other("local_path is not set"))?;

    let local_repo = Repository::open(local_path).map_err(|e| {
        Error::other(format!("goa error: failed to open the cloned repo: {}", e))
    })?;

    if repo.verbosity() > 1 {
        info!("checking for diffs at origin/{}!", repo.branch());
    }

    match git::is_diff(&local_repo, "origin", repo.branch(), repo.verbosity()) {
        Ok(commit) => {
            match git::do_merge(&local_repo, repo.branch(), commit, repo.verbosity()) {
                Ok(metadata) => {
                    // Determine the effective command: use configured command or read from .goa file
                    let effective_command = get_effective_command(repo, local_path);
                    if repo.verbosity() > 2 {
                        debug!("effective command: {}", effective_command);
                    }

                    match do_task(repo, &effective_command, Some(&metadata)) {
                        Ok(output) => {
                            if repo.verbosity() > 0 {
                                info!("command stdout: {}", output);
                            } else {
                                println!("{output}");
                            }

                            if repo.exit_on_first_diff() {
                                // Intentional exit after first diff processed
                                std::process::exit(0);
                            }
                        }
                        Err(e) => {
                            eprintln!("goa error: do_task error {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("goa error: do_merge error {}", e);
                }
            }
        }
        Err(e) => {
            // There were no diffs, so we move right along
            if repo.verbosity() > 1 {
                debug!("{}", e);
            }
        }
    }

    Ok(())
}

/// Execute a command with optional commit metadata passed as env vars to child process.
/// This is thread-safe as env vars are only set in the child process, not globally.
/// If timeout > 0, the command will be killed after the specified number of seconds.
fn do_task(repo: &Repo, command: &str, metadata: Option<&CommitMetadata>) -> Result<String> {
    let local_path = repo
        .local_path()
        .ok_or_else(|| Error::other("local_path is not set"))?;

    if repo.verbosity() > 1 {
        info!("running -> {}", command);
        if repo.timeout() > 0 {
            info!("timeout -> {} seconds", repo.timeout());
        }
    }

    if repo.verbosity() > 2 {
        debug!("path -> {}", local_path);
    }

    // Use timeout-based execution if timeout is set
    if repo.timeout() > 0 {
        return do_task_with_timeout(repo, command, metadata, local_path);
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
    let (code, output, error) = run_script::run(command, &args, &options)
        .map_err(|e| Error::other(format!("Failed to run script: {}", e)))?;

    if repo.verbosity() > 1 {
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
    command: &str,
    metadata: Option<&CommitMetadata>,
    local_path: &str,
) -> Result<String> {
    let timeout_duration = Duration::from_secs(repo.timeout());

    // Build the command using sh -c for shell interpretation
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
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

            if repo.verbosity() > 1 {
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
            if repo.verbosity() > 0 {
                warn!(
                    "Command timed out after {} seconds, killing process",
                    repo.timeout()
                );
            }
            child
                .kill()
                .map_err(|e| Error::other(format!("Failed to kill timed-out process: {}", e)))?;
            child.wait().ok(); // Clean up zombie process

            Err(Error::other(format!(
                "Command timed out after {} seconds",
                repo.timeout()
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
        let repo = Repo::builder("file://.")
            .local_path(".")
            .branch("develop")
            .build();

        assert_eq!("develop", repo.branch());
    }

    #[test]
    fn test_builder_defaults() {
        let repo = Repo::builder("https://github.com/test/repo").build();

        assert_eq!("main", repo.branch());
        assert_eq!(120, repo.delay());
        assert_eq!(1, repo.verbosity());
        assert!(!repo.exec_on_start());
        assert!(!repo.exit_on_first_diff());
        assert_eq!(0, repo.timeout());
    }

    #[test]
    fn test_do_task() {
        let repo = Repo::builder("file://.")
            .local_path(".")
            .branch("develop")
            .command("echo hello")
            .verbosity(3)
            .build();

        let res = do_task(&repo, "echo hello", None);
        assert_eq!(String::from("hello\n"), res.unwrap());
    }

    #[test]
    fn test_do_process() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", uuid::Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);

        let repo = Repo::builder("https://github.com/kitplummer/goa_tester")
            .local_path(&local_path)
            .branch("main")
            .command("echo hello")
            .verbosity(2)
            .build();

        repo.clone_repo()?;

        assert_eq!(do_process(&repo)?, ());
        Ok(())
    }

    #[test]
    fn test_do_process_no_clone() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", uuid::Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);

        let mut repo = Repo::builder("https://github.com/kitplummer/goa_tester")
            .local_path(&local_path)
            .branch("main")
            .command("echo hello")
            .verbosity(2)
            .build();

        repo.clone_repo()?;
        repo.set_local_path(String::from("/blahdyblahblah"));
        let res = do_process(&repo).unwrap_err();
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

        let repo = Repo::builder("https://github.com/kitplummer/goa_tester")
            .local_path(&local_path)
            .branch("main")
            .verbosity(3)
            .build();

        repo.clone_repo()?;

        assert_eq!(do_process(&repo)?, ());
        Ok(())
    }

    #[test]
    fn test_no_goa_file() {
        let res = read_goa_file("/blahdy/.goa");
        assert_eq!(res, String::from("echo 'no goa file found yet'"));
    }

    #[test]
    fn test_do_task_with_timeout() {
        let repo = Repo::builder("file://.")
            .local_path(".")
            .branch("develop")
            .command("echo hello")
            .verbosity(3)
            .timeout(5)
            .build();

        let res = do_task(&repo, "echo hello", None);
        assert_eq!(String::from("hello\n"), res.unwrap());
    }

    #[test]
    fn test_do_task_timeout_exceeded() {
        let repo = Repo::builder("file://.")
            .local_path(".")
            .branch("develop")
            .command("sleep 10")
            .verbosity(1)
            .timeout(1)
            .build();

        let res = do_task(&repo, "sleep 10", None);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("timed out"));
    }
}
