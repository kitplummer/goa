use std::env::temp_dir;
use std::io::{Error, Result};

use tracing::info;
use url::Url;
use uuid::Uuid;

use crate::repos::Repo;

pub fn spy_repo(mut repo: Repo) -> Result<()> {
    if repo.verbosity() > 0 {
        info!("starting to spy {}:{}", repo.url(), repo.branch());
    }

    // Parse and potentially modify URL with credentials
    let authenticated_url = match Url::parse(repo.url()) {
        Ok(mut parsed_url) => {
            if let Some(username) = repo.username() {
                parsed_url
                    .set_username(username)
                    .map_err(|_| Error::other("Failed to set username in URL"))?;
            }

            if let Some(token) = repo.token() {
                parsed_url
                    .set_password(Some(token))
                    .map_err(|_| Error::other("Failed to set password in URL"))?;
            }
            parsed_url.to_string()
        }
        Err(e) => {
            return Err(Error::other(format!("goa error: invalid URL or path, {}", e)));
        }
    };
    repo.set_url(authenticated_url);

    if repo.local_path().is_none() {
        // Get a temp directory to do work in
        let tmp_dir_name = temp_dir()
            .join(Uuid::new_v4().to_string());
        let local_path = tmp_dir_name
            .to_str()
            .ok_or_else(|| Error::other("Failed to convert temp directory path to string"))?
            .to_string();

        // Set the local repo path in the repo struct
        repo.set_local_path(local_path);
    }

    // Clone the repo and set the local path
    repo.clone_repo()?;

    // This is where the loop happens...
    repo.spy_for_changes()?;

    Ok(())
}

// Use functional tests to evaluate this code
