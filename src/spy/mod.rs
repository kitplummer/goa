use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result};

// For datetime/timestamp/log
use chrono::Utc;
use git2::Repository;
use url::Url;
use uuid::Uuid;

use crate::repos::Repo;

pub fn spy_repo(
    url: String,
    branch: String,
    delay: u16,
    username: Option<String>,
    token: Option<String>,
    command: Option<String>,
    verbosity: u8,
    exec_on_start: bool,
) -> Result<()> {
    let dt = Utc::now();
    println!("goa [{}]: starting to spy {}:{}", dt, url, branch);
    let parsed_url = Url::parse(&url);

    match parsed_url {
        Ok(mut parsed_url) => {
            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) {
                    let dt = Utc::now();
                    eprintln!(
                        "goa [{}]: Error - {:?}",
                        dt,
                        e,
                    );
                    std::process::exit(1);
                };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) {
                    let dt = Utc::now();
                    eprintln!(
                        "goa [{}]: Error - {:?}",
                        dt,
                        e,
                    );
                    std::process::exit(1);
                };
            }

            // Get a temp directory to do work in
            let temp_dir = temp_dir();
            let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
            let tmp_dir_name = format!("{}", Uuid::new_v4());
            local_path.push_str("/goa_wd/");
            local_path.push_str(&String::from(tmp_dir_name));

            // TODO: investigate shallow clone here
            let cloned_repo = match Repository::clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(_e) => {
                    let dt = Utc::now();
                    eprintln!(
                        "goa [{}]: Error - failed to clone, possible invalid URL or path.",
                        dt
                    );
                    std::process::exit(1);
                },
            };
            let repo_path = cloned_repo.workdir().unwrap();
            let command = match command {
                Some(command) => command,
                None => {
                    String::from("")
                }
            };

            if verbosity > 2 {
                println!("goa debug: command: {}", command);
            }

            let repo = Repo::new(
                String::from(parsed_url.as_str()),
                String::from(repo_path.to_str().unwrap()),
                branch,
                command,
                delay,
                verbosity,
                exec_on_start,
            );

            // This is where the loop happens...
            // For thread safety, we're going to have to simply pass the repo struct through, and
            // recreate the Repository "object" in each thread.  Perhaps not most performant,
            // but only sane way to manage through the thread scheduler.
            repo.spy_for_changes();

            Ok(())
        }
        Err(e) => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Error: Invalid URL {}", e),
        )),
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
            1,
            false,
        )
        .map_err(|e| e.kind());

        assert_eq!(Err(ErrorKind::InvalidData), res);
    }

}
