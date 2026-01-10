use std::collections::HashMap;
use std::io::{Error, Result};
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use clokwerk::{Scheduler, TimeUnits};

use crate::repos::{execute_command, read_goa_file};

/// Radicle repository configuration
#[derive(Debug, Clone)]
pub struct RadicleConfig {
    seed_url: String,
    rid: String,
    command: Option<String>,
    delay: u16,
    verbosity: u8,
    timeout: u64,
    watch_patches: bool,
    local_path: Option<String>,
}

impl RadicleConfig {
    pub fn builder(seed_url: impl Into<String>, rid: impl Into<String>) -> RadicleConfigBuilder {
        RadicleConfigBuilder::new(seed_url, rid)
    }

    pub fn seed_url(&self) -> &str {
        &self.seed_url
    }

    pub fn rid(&self) -> &str {
        &self.rid
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

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn watch_patches(&self) -> bool {
        self.watch_patches
    }

    pub fn local_path(&self) -> Option<&str> {
        self.local_path.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct RadicleConfigBuilder {
    seed_url: String,
    rid: String,
    command: Option<String>,
    delay: u16,
    verbosity: u8,
    timeout: u64,
    watch_patches: bool,
    local_path: Option<String>,
}

impl RadicleConfigBuilder {
    pub fn new(seed_url: impl Into<String>, rid: impl Into<String>) -> Self {
        Self {
            seed_url: seed_url.into(),
            rid: rid.into(),
            command: None,
            delay: 120,
            verbosity: 1,
            timeout: 0,
            watch_patches: true,
            local_path: None,
        }
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        let c = command.into();
        self.command = if c.is_empty() { None } else { Some(c) };
        self
    }

    pub fn delay(mut self, delay: u16) -> Self {
        self.delay = delay;
        self
    }

    pub fn verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn watch_patches(mut self, watch: bool) -> Self {
        self.watch_patches = watch;
        self
    }

    pub fn local_path(mut self, path: impl Into<String>) -> Self {
        self.local_path = Some(path.into());
        self
    }

    pub fn build(self) -> RadicleConfig {
        RadicleConfig {
            seed_url: self.seed_url,
            rid: self.rid,
            command: self.command,
            delay: self.delay,
            verbosity: self.verbosity,
            timeout: self.timeout,
            watch_patches: self.watch_patches,
            local_path: self.local_path,
        }
    }
}

/// Metadata for Radicle triggers, passed as env vars to commands
#[derive(Debug, Clone, Default)]
pub struct RadicleMetadata {
    pub rid: String,
    pub seed_url: String,
    pub trigger_type: String, // "push" or "patch"
    pub commit_oid: String,
    pub patch_id: Option<String>,
    pub base_commit: Option<String>,
    pub patch_state: Option<String>,
    pub patch_title: Option<String>,
}

impl RadicleMetadata {
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("GOA_RADICLE_RID".to_string(), self.rid.clone());
        vars.insert("GOA_RADICLE_URL".to_string(), self.seed_url.clone());
        vars.insert("GOA_TRIGGER_TYPE".to_string(), self.trigger_type.clone());
        vars.insert("GOA_COMMIT_OID".to_string(), self.commit_oid.clone());

        if let Some(ref patch_id) = self.patch_id {
            vars.insert("GOA_PATCH_ID".to_string(), patch_id.clone());
        }
        if let Some(ref base) = self.base_commit {
            vars.insert("GOA_BASE_COMMIT".to_string(), base.clone());
        }
        if let Some(ref state) = self.patch_state {
            vars.insert("GOA_PATCH_STATE".to_string(), state.clone());
        }
        if let Some(ref title) = self.patch_title {
            vars.insert("GOA_PATCH_TITLE".to_string(), title.clone());
        }

        vars
    }
}

// API Response types - fields kept for API completeness even if not all are used

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoResponse {
    pub rid: String,
    pub payloads: Payloads,
}

#[derive(Debug, Deserialize)]
pub struct Payloads {
    #[serde(rename = "xyz.radicle.project")]
    pub project: ProjectPayload,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ProjectPayload {
    pub data: ProjectData,
    pub meta: ProjectMeta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectMeta {
    pub head: String,
}

#[derive(Debug, Deserialize)]
pub struct Patch {
    pub id: String,
    pub title: String,
    pub state: PatchState,
    pub revisions: Vec<Revision>,
}

#[derive(Debug, Deserialize)]
pub struct PatchState {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Revision {
    pub id: String,
    pub oid: String,
    pub base: String,
    pub timestamp: i64,
}

/// State tracking for detecting changes
#[derive(Debug, Clone, Default)]
struct WatchState {
    last_head: Option<String>,
    last_patch_timestamps: HashMap<String, i64>,
}

/// Watch a Radicle repository for changes
pub fn watch_radicle(config: RadicleConfig) -> Result<()> {
    if config.verbosity() > 0 {
        info!(
            "watching Radicle repo {} at {} every {} seconds",
            config.rid(),
            config.seed_url(),
            config.delay()
        );
        if config.watch_patches() {
            info!("also watching for patch updates");
        }
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::other(format!("Failed to create HTTP client: {}", e)))?;

    let mut state = WatchState::default();

    // Do initial fetch to establish baseline
    if let Ok(repo_info) = fetch_repo_info(&client, &config) {
        state.last_head = Some(repo_info.payloads.project.meta.head.clone());
        if config.verbosity() > 0 {
            info!("initial head: {}", repo_info.payloads.project.meta.head);
        }
    }

    if config.watch_patches() {
        if let Ok(patches) = fetch_patches(&client, &config) {
            for patch in patches {
                if let Some(rev) = patch.revisions.last() {
                    state.last_patch_timestamps.insert(patch.id.clone(), rev.timestamp);
                }
            }
            if config.verbosity() > 0 {
                info!("tracking {} patches", state.last_patch_timestamps.len());
            }
        }
    }

    // Set up scheduler
    let mut scheduler = Scheduler::new();
    let delay = config.delay() as u32;

    scheduler.every(delay.seconds()).run(move || {
        if let Err(e) = check_for_changes(&client, &config, &mut state) {
            eprintln!("goa error: failed to check Radicle repo: {}", e);
        }
    });

    // Event loop
    loop {
        scheduler.run_pending();
        thread::sleep(Duration::from_millis(10));
    }
}

fn fetch_repo_info(client: &Client, config: &RadicleConfig) -> Result<RepoResponse> {
    let url = format!("{}/api/v1/repos/{}", config.seed_url(), config.rid());

    if config.verbosity() > 1 {
        debug!("fetching repo info from {}", url);
    }

    let response = client
        .get(&url)
        .send()
        .map_err(|e| Error::other(format!("HTTP request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::other(format!(
            "API returned status {}",
            response.status()
        )));
    }

    response
        .json::<RepoResponse>()
        .map_err(|e| Error::other(format!("Failed to parse repo response: {}", e)))
}

fn fetch_patches(client: &Client, config: &RadicleConfig) -> Result<Vec<Patch>> {
    let url = format!(
        "{}/api/v1/repos/{}/patches",
        config.seed_url(),
        config.rid()
    );

    if config.verbosity() > 1 {
        debug!("fetching patches from {}", url);
    }

    let response = client
        .get(&url)
        .send()
        .map_err(|e| Error::other(format!("HTTP request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::other(format!(
            "API returned status {}",
            response.status()
        )));
    }

    response
        .json::<Vec<Patch>>()
        .map_err(|e| Error::other(format!("Failed to parse patches response: {}", e)))
}

fn check_for_changes(
    client: &Client,
    config: &RadicleConfig,
    state: &mut WatchState,
) -> Result<()> {
    if config.verbosity() > 1 {
        info!("checking for changes...");
    }

    // Check for head changes (push to main branch)
    if let Ok(repo_info) = fetch_repo_info(client, config) {
        let current_head = &repo_info.payloads.project.meta.head;

        if let Some(ref last_head) = state.last_head {
            if current_head != last_head {
                if config.verbosity() > 0 {
                    info!("detected push: {} -> {}", last_head, current_head);
                }

                let metadata = RadicleMetadata {
                    rid: config.rid().to_string(),
                    seed_url: config.seed_url().to_string(),
                    trigger_type: "push".to_string(),
                    commit_oid: current_head.clone(),
                    ..Default::default()
                };

                execute_radicle_command(config, &metadata)?;
                state.last_head = Some(current_head.clone());
            }
        } else {
            state.last_head = Some(current_head.clone());
        }
    }

    // Check for patch changes
    if config.watch_patches() {
        if let Ok(patches) = fetch_patches(client, config) {
            for patch in patches {
                // Only process open patches
                if patch.state.status != "open" {
                    continue;
                }

                if let Some(rev) = patch.revisions.last() {
                    let last_timestamp = state.last_patch_timestamps.get(&patch.id).copied();

                    let is_new_or_updated = match last_timestamp {
                        Some(ts) => rev.timestamp > ts,
                        None => true,
                    };

                    if is_new_or_updated {
                        if config.verbosity() > 0 {
                            info!(
                                "detected patch update: {} - {}",
                                &patch.id[..8],
                                patch.title
                            );
                        }

                        let metadata = RadicleMetadata {
                            rid: config.rid().to_string(),
                            seed_url: config.seed_url().to_string(),
                            trigger_type: "patch".to_string(),
                            commit_oid: rev.oid.clone(),
                            patch_id: Some(patch.id.clone()),
                            base_commit: Some(rev.base.clone()),
                            patch_state: Some(patch.state.status.clone()),
                            patch_title: Some(patch.title.clone()),
                        };

                        execute_radicle_command(config, &metadata)?;
                        state.last_patch_timestamps.insert(patch.id.clone(), rev.timestamp);
                    }
                }
            }
        }
    }

    Ok(())
}

fn execute_radicle_command(config: &RadicleConfig, metadata: &RadicleMetadata) -> Result<()> {
    let effective_command = match config.command() {
        Some(cmd) => cmd.to_string(),
        None => {
            // Try to read from local .goa file if local_path is set
            match config.local_path() {
                Some(path) => read_goa_file(&format!("{}/.goa", path)),
                None => {
                    return Err(Error::other(
                        "No command specified and no local path for .goa file",
                    ))
                }
            }
        }
    };

    if config.verbosity() > 1 {
        info!("executing: {}", effective_command);
    }

    let working_dir = config.local_path().unwrap_or(".");

    match execute_command(
        &effective_command,
        working_dir,
        Some(metadata.to_env_vars()),
        config.timeout(),
        config.verbosity(),
    ) {
        Ok(output) => {
            if config.verbosity() > 0 {
                info!("command stdout: {}", output);
            } else {
                println!("{}", output);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("goa error: command failed: {}", e);
            Ok(()) // Don't stop watching on command failure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radicle_config_builder() {
        let config = RadicleConfig::builder("https://iris.radicle.xyz", "rad:z123")
            .command("echo test")
            .delay(60)
            .verbosity(2)
            .timeout(30)
            .watch_patches(false)
            .build();

        assert_eq!(config.seed_url(), "https://iris.radicle.xyz");
        assert_eq!(config.rid(), "rad:z123");
        assert_eq!(config.command(), Some("echo test"));
        assert_eq!(config.delay(), 60);
        assert_eq!(config.verbosity(), 2);
        assert_eq!(config.timeout(), 30);
        assert!(!config.watch_patches());
    }

    #[test]
    fn test_radicle_metadata_env_vars() {
        let metadata = RadicleMetadata {
            rid: "rad:z123".to_string(),
            seed_url: "https://iris.radicle.xyz".to_string(),
            trigger_type: "patch".to_string(),
            commit_oid: "abc123".to_string(),
            patch_id: Some("patch456".to_string()),
            base_commit: Some("def789".to_string()),
            patch_state: Some("open".to_string()),
            patch_title: Some("Fix bug".to_string()),
        };

        let vars = metadata.to_env_vars();
        assert_eq!(vars.get("GOA_RADICLE_RID"), Some(&"rad:z123".to_string()));
        assert_eq!(vars.get("GOA_TRIGGER_TYPE"), Some(&"patch".to_string()));
        assert_eq!(vars.get("GOA_COMMIT_OID"), Some(&"abc123".to_string()));
        assert_eq!(vars.get("GOA_PATCH_ID"), Some(&"patch456".to_string()));
        assert_eq!(vars.get("GOA_BASE_COMMIT"), Some(&"def789".to_string()));
    }

    #[test]
    fn test_radicle_config_defaults() {
        let config = RadicleConfig::builder("https://example.com", "rad:test").build();

        assert_eq!(config.delay(), 120);
        assert_eq!(config.verbosity(), 1);
        assert_eq!(config.timeout(), 0);
        assert!(config.watch_patches());
        assert!(config.command().is_none());
    }
}
