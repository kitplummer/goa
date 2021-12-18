use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result};

use git2::Repository;
use uuid::Uuid;
use url::Url;

#[derive(Debug, Clone)]
pub struct Repo {
    pub url: String,
    pub status: String,
    pub local_path: String,
}

impl Repo {
    pub fn new(url: String, local_path: String) -> Repo {
        // We'll initialize after the clone is successful.
        let status = String::from("cloned");
        Repo { url, status, local_path }
    }
}

pub fn spy_repo(url: String, username: Option<String>, token: Option<String>) -> Result<()> {

    let parsed_url = Url::parse(&url);

    match parsed_url {
        Ok(mut parsed_url) => {

            if let Some(username) = username {
                if let Err(e) = parsed_url.set_username(&username) { panic!("Error: {:?}", e) };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = parsed_url.set_password(Option::from(token_str)) { panic!("Error: {:?}", e) };
            }

            // Get a temp directory to do work in
            let temp_dir = temp_dir();
            let mut local_path: String = temp_dir.into_os_string().into_string().unwrap();
            local_path.push_str("/goa_wd/");

            let tmp_dir_name = format!("{}", Uuid::new_v4());
            local_path.push_str(&String::from(tmp_dir_name));

            let cloned_repo = match Repository:: clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo = Repo::new(String::from(parsed_url.as_str()), String::from(cloned_repo.path().to_str().unwrap()));
            println!("Spying repo {:?} at {:?}", url, repo.local_path);
            Ok(())
        },
        Err(e) => Err(Error::new(ErrorKind::InvalidData, format!("Invalid URL {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_spy_repo_with_good_url() {
        assert!(spy_repo(String::from("https://github.com/kitplummer/clikan"), Some(String::from("")), Some(String::from(""))).is_ok());
    }

    #[test]
    fn test_spy_repo_with_bad_url() {
        let res = spy_repo(String::from("test"), Some(String::from("test")), Some(String::from("test"))).map_err(|e| e.kind());
        assert_eq!(Err(ErrorKind::InvalidData), res);
    }

    #[test]
    fn test_get_repo_from_url() {
        
        assert!(true);
    }
}