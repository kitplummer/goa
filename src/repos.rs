use std::env::temp_dir;
use std::io::{Error, ErrorKind, Result, Write};
use std::thread;
use std::time::Duration;

// Scheduler, and trait for .seconds(), .minutes(), etc.
use clokwerk::{Scheduler, TimeUnits};

use git2::{Repository, Object, ObjectType, Diff, DiffStatsFormat, RemoteCallbacks, FetchOptions, AutotagOption};
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

    // pub fn contain_goa_file() -> bool {
    //     false
    // }
}

pub fn spy_repo(url: String, branch: String, delay: u16, username: Option<String>, token: Option<String>) -> Result<()> {

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

            // TODO: investigate shallow clone here
            let cloned_repo = match Repository:: clone(parsed_url.as_str(), local_path) {
                Ok(repo) => repo,
                Err(e) => panic!("Error: Failed to clone {}", e),
            };
            let repo = Repo::new(String::from(parsed_url.as_str()), String::from(cloned_repo.path().to_str().unwrap()));
            println!("Spying repo {:?} at {:?}", url, repo.local_path);

            // This is where the loop happens...
            // For thread safety, we're going to have to simply pass the repo struct through, and
            // recreate the Repository "object" in each thread.  Perhaps not most performant,
            // but only sane way to manage through the thread scheduler.
            spy_for_changes(repo, delay);

            Ok(())
        },
        Err(e) => Err(Error::new(ErrorKind::InvalidData, format!("Invalid URL {}", e))),
    }
}

pub fn git_diff(repo: &Repo) -> Result<()> {
    println!("Checking for diffs!");
    let local_repo = match Repository::open(&repo.local_path) {
        Ok(local_repo) => local_repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    // TODO - Need to fetch the origin branch first...
    let remote = "origin";
    println!("Fetching {} for repo", remote);
    let mut cb = RemoteCallbacks::new();
    let mut remote = local_repo
        .find_remote(remote)
        .or_else(|_| local_repo.remote_anonymous(remote)).unwrap();
    cb.sideband_progress(|data| {
        print!("remote: {}", std::str::from_utf8(data).unwrap());
        std::io::stdout().flush().unwrap();
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    remote.download(&[] as &[&str], Some(&mut fo)).unwrap();

    // Disconnect the underlying connection to prevent from idling.
    remote.disconnect().unwrap();

    // Update the references in the remote's namespace to point to the right
    // commits. This may be needed even if there was no packfile to download,
    // which can happen e.g. when the branches have been changed but all the
    // needed objects are available locally.
    remote.update_tips(None, true, AutotagOption::Unspecified, None).unwrap();

    let l = String::from("git2");
    let r = String::from("origin/git2");
    let tl = tree_to_treeish(&local_repo, Some(&l)).unwrap();
    let tr = tree_to_treeish(&local_repo, Some(&r)).unwrap();

    let diff = match (tl, tr) {
        (Some(local), Some(origin)) => local_repo.diff_tree_to_tree(local.as_tree(), origin.as_tree(), None),
        (_, _) => unreachable!(),
    };

    print_stats(&diff.unwrap()).expect("Error: unable to get diff stats");
    //println!("UPSTREAM: {:?}", diff.unwrap().print(DiffFormat::Raw, ));
    // pub fn diff_tree_to_workdir(
    //   &self,
    //   old_tree: Option<&Tree<'_>>,
    //   opts: Option<&mut DiffOptions>
    // ) -> Result<Diff<'_>, Error>

    Ok(())
}

pub fn spy_for_changes(repo: Repo, delay: u16) {
    println!("Checking for changes every {} seconds", delay);

    // Create a new scheduler
    let mut scheduler = Scheduler::new();
    // Add some tasks to it
    scheduler.every(30.seconds()).run(move || 
        git_diff(&repo).expect("Error: unable to attach to local repo.")
    );
    // Manually run the scheduler in an event loop
    loop {
        scheduler.run_pending();
        thread::sleep(Duration::from_millis(10));
    }
}

// Git ish
fn tree_to_treeish<'a>(
    repo: &'a Repository,
    arg: Option<&String>,
) -> Result<Option<Object<'a>>> {
    let arg = match arg {
        Some(s) => s,
        None => return Ok(None),
    };
    let obj = repo.revparse_single(arg).unwrap();
    let tree = obj.peel(ObjectType::Tree).unwrap();
    Ok(Some(tree))
}

fn print_stats(diff: &Diff) -> Result<()> {
    let stats = diff.stats().unwrap();
    let format = DiffStatsFormat::SHORT;
    let buf = stats.to_buf(format, 80).unwrap();
    print!("{}", std::str::from_utf8(&*buf).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_spy_repo_with_good_url() {
        assert!(spy_repo(
            String::from("https://github.com/kitplummer/clikan"),
            String::from("branch"),
            120,
            Some(String::from("")),
            Some(String::from(""))
        ).is_ok());
    }

    #[test]
    fn test_spy_repo_with_bad_url() {
        let res = spy_repo(
            String::from("test"),
            String::from("branch"),
            120,
            Some(String::from("test")),
            Some(String::from("test"))
        ).map_err(|e| e.kind());

        assert_eq!(Err(ErrorKind::InvalidData), res);
    }

    // #[test]
    // fn test_contain_a_goa_file() {   
    //     assert!(true);
    // }
}