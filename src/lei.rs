use std::collections::HashMap;
use std::io::{Error, Result};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Known dependency/manifest file names across ecosystems.
const DEP_FILE_NAMES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "go.mod",
    "go.sum",
    "Gemfile",
    "Gemfile.lock",
    "requirements.txt",
    "Pipfile",
    "Pipfile.lock",
    "pyproject.toml",
    "poetry.lock",
    "setup.py",
    "setup.cfg",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "mix.exs",
    "mix.lock",
    "composer.json",
    "composer.lock",
    "pubspec.yaml",
    "pubspec.lock",
    "*.csproj",
    "*.fsproj",
    "packages.config",
    "CMakeLists.txt",
    "conanfile.txt",
    "conanfile.py",
    "Makefile.PL",
    "cpanfile",
    "cabal.project",
    "stack.yaml",
];

/// Configuration for LEI integration.
#[derive(Debug, Clone)]
pub struct LeiConfig {
    pub url: String,
    pub token: Option<String>,
}

/// Request body for the LEI `/v1/analyze` endpoint.
#[derive(Debug, Serialize)]
struct AnalyzeRequest {
    urls: Vec<String>,
}

/// Top-level response from LEI `/v1/analyze`.
#[derive(Debug, Deserialize)]
pub struct AnalyzeResponse {
    pub uuid: Option<String>,
    pub state: Option<String>,
    pub report: Option<Report>,
}

// API response types - fields kept for API completeness even if not all are used

/// Report wrapper in LEI response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Report {
    pub uuid: Option<String>,
    pub repos: Option<Vec<RepoReport>>,
}

/// Per-repo analysis result.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoReport {
    pub header: Option<RepoHeader>,
    pub data: Option<RepoData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoHeader {
    pub url: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoData {
    pub risk: Option<String>,
    pub results: Option<AnalysisResults>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AnalysisResults {
    pub contributor_count: Option<i64>,
    pub functional_contributor_names: Option<Vec<String>>,
    pub functional_contributors: Option<i64>,
    pub functional_contributors_risk: Option<String>,
    pub commit_currency_weeks: Option<i64>,
    pub commit_currency_risk: Option<String>,
    pub large_recent_commit_risk: Option<String>,
    pub recent_commit_size_in_percent_of_codebase: Option<f64>,
}

/// Check whether a git diff contains changes to known dependency files.
/// Returns the list of changed dependency file paths.
pub fn find_dependency_changes(diff: &git2::Diff) -> Vec<String> {
    let mut changed = Vec::new();

    for delta in diff.deltas() {
        for path in [delta.new_file().path(), delta.old_file().path()]
            .into_iter()
            .flatten()
        {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let full_path = path.to_string_lossy().to_string();

            if is_dep_file(filename) && !changed.contains(&full_path) {
                changed.push(full_path);
            }
        }
    }

    changed
}

/// Check if a filename matches known dependency file patterns.
fn is_dep_file(filename: &str) -> bool {
    for pattern in DEP_FILE_NAMES {
        if let Some(suffix) = pattern.strip_prefix('*') {
            // Wildcard suffix match (e.g. "*.csproj")
            if filename.ends_with(suffix) {
                return true;
            }
        } else if filename == *pattern {
            return true;
        }
    }
    false
}

/// Send a repository URL to the LEI analyze endpoint.
/// Returns the response or an error.
pub fn analyze_repo(config: &LeiConfig, repo_url: &str, verbosity: u8) -> Result<AnalyzeResponse> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| Error::other(format!("Failed to create HTTP client: {}", e)))?;

    let url = format!("{}/v1/analyze", config.url.trim_end_matches('/'));

    let body = AnalyzeRequest {
        urls: vec![repo_url.to_string()],
    };

    if verbosity > 0 {
        info!(
            "LEI: sending analysis request to {} for {}",
            url, repo_url
        );
    }

    let mut request = client.post(&url).json(&body);

    if let Some(ref token) = config.token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .map_err(|e| Error::other(format!("LEI request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().unwrap_or_default();
        return Err(Error::other(format!(
            "LEI API returned status {}: {}",
            status, body_text
        )));
    }

    let analyze_resp: AnalyzeResponse = response
        .json()
        .map_err(|e| Error::other(format!("Failed to parse LEI response: {}", e)))?;

    if verbosity > 0 {
        log_analysis_summary(&analyze_resp, repo_url);
    }

    Ok(analyze_resp)
}

/// Log a human-readable summary of the LEI analysis result.
fn log_analysis_summary(resp: &AnalyzeResponse, repo_url: &str) {
    if let Some(ref report) = resp.report {
        if let Some(ref repos) = report.repos {
            for repo in repos {
                if let Some(ref data) = repo.data {
                    let risk = data.risk.as_deref().unwrap_or("unknown");
                    info!("LEI: {} overall risk: {}", repo_url, risk);

                    if let Some(ref results) = data.results {
                        if let Some(ref fc_risk) = results.functional_contributors_risk {
                            info!("LEI:   functional contributors risk: {}", fc_risk);
                        }
                        if let Some(ref cc_risk) = results.commit_currency_risk {
                            info!("LEI:   commit currency risk: {}", cc_risk);
                        }
                        if let Some(ref lrc_risk) = results.large_recent_commit_risk {
                            info!("LEI:   large recent commit risk: {}", lrc_risk);
                        }
                    }
                }
            }
        }
    }

    if let Some(ref state) = resp.state {
        if state != "complete" {
            if let Some(ref uuid) = resp.uuid {
                info!(
                    "LEI: analysis state: {} (poll {}/v1/analyze/{})",
                    state,
                    "LEI_URL",
                    uuid
                );
            }
        }
    }
}

/// Build env vars from LEI analysis for passing to child commands.
#[allow(dead_code)]
pub fn to_env_vars(
    config: &LeiConfig,
    dep_files: &[String],
    response: &AnalyzeResponse,
) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    vars.insert("GOA_LEI_URL".to_string(), config.url.clone());
    vars.insert(
        "GOA_LEI_DEP_FILES_CHANGED".to_string(),
        dep_files.join(","),
    );

    if let Some(ref uuid) = response.uuid {
        vars.insert("GOA_LEI_JOB_UUID".to_string(), uuid.clone());
    }
    if let Some(ref state) = response.state {
        vars.insert("GOA_LEI_STATE".to_string(), state.clone());
    }

    if let Some(ref report) = response.report {
        if let Some(ref repos) = report.repos {
            if let Some(repo) = repos.first() {
                if let Some(ref data) = repo.data {
                    if let Some(ref risk) = data.risk {
                        vars.insert("GOA_LEI_RISK".to_string(), risk.clone());
                    }
                }
            }
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dep_file_exact_match() {
        assert!(is_dep_file("Cargo.toml"));
        assert!(is_dep_file("package.json"));
        assert!(is_dep_file("go.mod"));
        assert!(is_dep_file("requirements.txt"));
        assert!(is_dep_file("mix.exs"));
        assert!(is_dep_file("pom.xml"));
    }

    #[test]
    fn test_is_dep_file_wildcard_match() {
        assert!(is_dep_file("MyProject.csproj"));
        assert!(is_dep_file("App.fsproj"));
    }

    #[test]
    fn test_is_dep_file_non_match() {
        assert!(!is_dep_file("main.rs"));
        assert!(!is_dep_file("README.md"));
        assert!(!is_dep_file("index.html"));
        assert!(!is_dep_file("app.js"));
        assert!(!is_dep_file(".gitignore"));
    }

    #[test]
    fn test_lei_config() {
        let config = LeiConfig {
            url: "http://localhost:4000".to_string(),
            token: Some("test-token".to_string()),
        };
        assert_eq!(config.url, "http://localhost:4000");
        assert_eq!(config.token, Some("test-token".to_string()));
    }

    #[test]
    fn test_lei_config_no_token() {
        let config = LeiConfig {
            url: "http://localhost:4000".to_string(),
            token: None,
        };
        assert!(config.token.is_none());
    }

    #[test]
    fn test_to_env_vars() {
        let config = LeiConfig {
            url: "http://localhost:4000".to_string(),
            token: None,
        };
        let dep_files = vec!["Cargo.toml".to_string(), "Cargo.lock".to_string()];
        let response = AnalyzeResponse {
            uuid: Some("test-uuid-123".to_string()),
            state: Some("complete".to_string()),
            report: None,
        };

        let vars = to_env_vars(&config, &dep_files, &response);
        assert_eq!(
            vars.get("GOA_LEI_URL"),
            Some(&"http://localhost:4000".to_string())
        );
        assert_eq!(
            vars.get("GOA_LEI_DEP_FILES_CHANGED"),
            Some(&"Cargo.toml,Cargo.lock".to_string())
        );
        assert_eq!(
            vars.get("GOA_LEI_JOB_UUID"),
            Some(&"test-uuid-123".to_string())
        );
        assert_eq!(
            vars.get("GOA_LEI_STATE"),
            Some(&"complete".to_string())
        );
    }

    #[test]
    fn test_to_env_vars_with_risk() {
        let config = LeiConfig {
            url: "http://localhost:4000".to_string(),
            token: None,
        };
        let dep_files = vec!["package.json".to_string()];
        let response = AnalyzeResponse {
            uuid: Some("uuid-456".to_string()),
            state: Some("complete".to_string()),
            report: Some(Report {
                uuid: Some("uuid-456".to_string()),
                repos: Some(vec![RepoReport {
                    header: None,
                    data: Some(RepoData {
                        risk: Some("medium".to_string()),
                        results: None,
                    }),
                }]),
            }),
        };

        let vars = to_env_vars(&config, &dep_files, &response);
        assert_eq!(vars.get("GOA_LEI_RISK"), Some(&"medium".to_string()));
    }
}
