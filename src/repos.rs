use std::io::Result;
use std::io;

use git2::Repository;
use url::Url;

#[derive(Debug)]
pub struct Repo {
    pub url: String,
    pub status: String,
}

impl Repo {
    pub fn new(url: String) -> Repo {
        let status = String::from("cloned");

        Repo { url, status }
    }
}

pub fn spy_repo(url: String, username: Option<String>, token: Option<String>) -> Result<()> {
    let parsed_url = Url::parse(&url);
    //let token = token.to_owned();

    match parsed_url {
        Ok(mut url) => {

            if let Some(username) = username {
                if let Err(e) = url.set_username(&username) { panic!("Error: {:?}", e) };
            }

            if let Some(token) = token {
                let token_str: &str = &token[..];
                if let Err(e) = url.set_password(Option::from(token_str)) { panic!("Error: {:?}", e) };
            }

            let path_vec = url.path_segments().map(|c| c.collect::<Vec<_>>());
            let last = path_vec.unwrap();
            let last = last.last();
            let mut target_path: String = "/tmp/goa_wd/".to_owned();
            target_path.push_str(last.unwrap());
            let _cloned_repo = match Repository:: clone(url.as_str(), target_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo = Repo::new(String::from(url.as_str()));
            println!("Spying repo: {:?}", repo);
            Ok(())
        },
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid URL {}", e))),
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
        assert_eq!(Err(io::ErrorKind::InvalidData), res);
    }

    #[test]
    fn test_get_repo_from_url() {
        
        assert!(true);
    }
}