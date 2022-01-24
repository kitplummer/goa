use std::env::temp_dir;
use std::io::Result;

use url::Url;
use uuid::Uuid;

use crate::repos::Repo;

pub fn spy_repo(mut repo: Repo) -> Result<()> {
    if repo.verbosity > 0 {
        info!("starting to spy {}:{}", repo.url, repo.branch);
    }

    repo.url = match Url::parse(&repo.url) {
        Ok(mut parsed_url) => {
            if let Some(ref username) = repo.username {
                if let Err(e) = parsed_url.set_username(username) {
                    eprintln!("goa error: {:?}", e);
                    std::process::exit(1);
                };
            }

            if let Some(ref token) = repo.token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) {
                    eprintln!("goa error: {:?}", e);
                    std::process::exit(1);
                };
            }
            parsed_url.to_string()
        }
        Err(e) => {
            eprintln!("goa error: invalid URL or path, {}", e);
            std::process::exit(1);
        }
    };

    if repo.local_path == None {
        // Get a temp directory to do work in
        let temp_dir = temp_dir();
        let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
        let tmp_dir_name = format!("/{}/", Uuid::new_v4());
        local_path.push_str(&tmp_dir_name);

        // Set the local repo path in the repo struct
        repo.local_path = Some(local_path);
    }

    // Clone the repo and set the local path
    repo.clone_repo();

    // This is where the loop happens...
    repo.spy_for_changes();

    Ok(())
}

// Use functional tests to evaluate this code
