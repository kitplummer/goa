use std::env::temp_dir;
use std::io::Result;

// For datetime/timestamp/log
use chrono::Utc;
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

    let parsed_url = match Url::parse(&url) {
        Ok(mut parsed_url) => {
            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) {
                    let dt = Utc::now();
                    eprintln!("goa [{}]: Error - {:?}", dt, e,);
                    std::process::exit(1);
                };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) {
                    let dt = Utc::now();
                    eprintln!("goa [{}]: Error - {:?}", dt, e,);
                    std::process::exit(1);
                };
            }
            parsed_url
        }
        Err(_e) => {
            eprintln!("goa error: invalid URL or path");
            std::process::exit(1);
        }
    };

    // Get a temp directory to do work in
    let temp_dir = temp_dir();
    let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
    let tmp_dir_name = format!("{}", Uuid::new_v4());
    local_path.push_str("/goa_wd/");
    local_path.push_str(&tmp_dir_name);

    // Preset the command from an Option
    let command = match command {
        Some(command) => command,
        None => String::from(""),
    };
    if verbosity > 2 {
        println!("goa debug: command: {}", command);
    }

    // Create the instance of the repo
    let repo = Repo::new(
        String::from(parsed_url.as_str()),
        local_path,
        branch,
        command,
        delay,
        verbosity,
        exec_on_start,
    );

    // Clone the repo and set the local path
    repo.clone_repo();

    // This is where the loop happens...
    repo.spy_for_changes();

    Ok(())
}

// Use functional tests to evaluate this code
