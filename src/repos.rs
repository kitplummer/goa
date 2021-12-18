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

pub fn spy_repo(url: String, username: String, token: String) -> Result<()> {
    let parsed_url = Url::parse(&url);
    let token = token.to_owned();
    let token_str: &str = &token[..];

    match parsed_url {
        Ok(mut url) => {
            match url.set_username(&username) {
                Err(e) => panic!("Error: {:?}", e),
                _ => ()
            };

            match url.set_password(Option::from(token_str)) {
                Err(e) => panic!("Error: {:?}", e),
                _ => ()
            };

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
        assert!(spy_repo(String::from("https://github.com/kitplummer/goa")).is_ok());
    }

    #[test]
    fn test_spy_repo_with_bad_url() {
        let res = spy_repo(String::from("test")).map_err(|e| e.kind());
        assert_eq!(Err(io::ErrorKind::InvalidData), res);
    }

    #[test]
    fn test_get_repo_from_url() {
        
        assert!(true);
    }
}