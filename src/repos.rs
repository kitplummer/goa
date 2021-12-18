use std::io::Result;
use std::io;
use url::Url;

#[derive(Debug)]
pub struct Repo {
    pub url: String,
    pub status: String,
}

impl Repo {
    pub fn new(url: String) -> Repo {
        let status = String::from("started");

        Repo { url, status }
    }
}

pub fn spy_repo(url: String) -> Result<()> {
    let parsed_url = Url::parse(&url);
    match parsed_url {
        Ok(url) => {
            let repo = Repo::new(String::from(url.as_str()));
            println!("Spying repo: {:?}", repo);
            Ok(())
        },
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid URL".to_string())),
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
}