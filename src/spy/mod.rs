use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result};

// For datetime/timestamp/log
use chrono::{Utc};
use git2::Repository;
use url::Url;
use uuid::Uuid;

use crate::repos::{Repo};

pub fn spy_repo(
    url: String,
    branch: String,
    delay: u16,
    username: Option<String>,
    token: Option<String>,
    command: Option<String>,
) -> Result<()> {

    let dt = Utc::now();
    println!("goa [{}]: starting to spy {}:{}", dt, url, branch);
    let parsed_url = Url::parse(&url);

    match parsed_url {
        Ok(mut parsed_url) => {
            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) {
                    panic!("Error: {:?}", e)
                };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) {
                    panic!("Error: {:?}", e)
                };
            }

            // Get a temp directory to do work in
            let temp_dir = temp_dir();
            let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
            let tmp_dir_name = format!("{}", Uuid::new_v4());
            let goa_path = format!("{}/goa_wd/{}/.goa", local_path, tmp_dir_name);
            local_path.push_str("/goa_wd/");
            local_path.push_str(&String::from(tmp_dir_name));

            // TODO: investigate shallow clone here
            let cloned_repo = match Repository::clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo_path = cloned_repo.workdir().unwrap();
            let command = match command {
                Some(command) => command,
                None => {
                    if std::path::Path::new(&goa_path).exists() {
                        let dt = Utc::now();
                        println!("goa [{}]: reading command from .goa file at {}", dt, goa_path);
                        std::fs::read_to_string(goa_path).expect("Error - failed to read .goa file")
                    } else {
                        let dt = Utc::now();
                        eprintln!("goa [{}]: Error - no command given, nor a .goa file found in the rep", dt);
                        std::process::exit(1);
                    }
                },
            };
            let repo = Repo::new(
                String::from(parsed_url.as_str()),
                String::from(repo_path.to_str().unwrap()),
                branch,
                command,
                delay,
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