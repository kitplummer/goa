use std::io::Result;

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
    let repo = Repo::new(url);
    println!("Spying repo: {:?}", repo);
    Ok(())
}
