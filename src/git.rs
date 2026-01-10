/*
 * libgit2 "pull" example - shows how to pull remote data into a local branch.
 *
 * Written by the libgit2 contributors
 *
 * To the extent possible under law, the author(s) have dedicated all copyright
 * and related and neighboring rights to this software to the public domain
 * worldwide. This software is distributed without any warranty.
 *
 * You should have received a copy of the CC0 Public Domain Dedication along
 * with this software. If not, see
 * <http://creativecommons.org/publicdomain/zero/1.0/>.
 */

use chrono::{DateTime, Utc};
use git2::{
    AutotagOption, Commit, Diff, DiffStatsFormat, FetchOptions, Object, ObjectType,
    RemoteCallbacks, RemoteUpdateFlags, Repository,
};
use std::collections::HashMap;
use std::io::Write;
use std::str;

/// Metadata extracted from a git commit, to be passed to child processes as env vars.
/// This avoids global env var mutation which is not thread-safe.
#[derive(Debug, Clone, Default)]
pub struct CommitMetadata {
    pub id: String,
    pub author: String,
    pub message: String,
    pub time: String,
}

impl CommitMetadata {
    /// Convert to a HashMap suitable for passing to child process environment
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("GOA_LAST_COMMIT_ID".to_string(), self.id.clone());
        vars.insert("GOA_LAST_COMMIT_AUTHOR".to_string(), self.author.clone());
        vars.insert("GOA_LAST_COMMIT_MESSAGE".to_string(), self.message.clone());
        vars.insert("GOA_LAST_COMMIT_TIME".to_string(), self.time.clone());
        vars
    }
}

pub fn is_diff<'a>(
    repo: &'a git2::Repository,
    remote_name: &str,
    branch_name: &str,
    verbosity: u8,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = RemoteCallbacks::new();
    let mut remote = repo
        .find_remote(remote_name)
        .or_else(|_| repo.remote_anonymous(remote_name))?;

    cb.sideband_progress(|data| {
        if verbosity >= 2 {
            let dt = Utc::now();
            if let Ok(msg) = std::str::from_utf8(data) {
                print!("goa [{}]: remote: {}", dt, msg);
            }
        }
        let _ = std::io::stdout().flush();
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    remote.download(&[] as &[&str], Some(&mut fo))?;

    // Disconnect the underlying connection to prevent from idling.
    remote.disconnect()?;

    // Update the references in the remote's namespace to point to the right
    // commits. This may be needed even if there was no packfile to download,
    // which can happen e.g. when the branches have been changed but all the
    // needed objects are available locally.
    remote.update_tips(
        None,
        RemoteUpdateFlags::UPDATE_FETCHHEAD,
        AutotagOption::Unspecified,
        None,
    )?;

    let l = String::from(branch_name);
    let r = format!("{}/{}", remote_name, branch_name);
    let tl = tree_to_treeish(repo, Some(&l))?;
    let tr = tree_to_treeish(repo, Some(&r))?;

    let head = repo.head()?;
    let oid = head
        .target()
        .ok_or_else(|| git2::Error::from_str("HEAD has no target"))?;
    let commit = repo.find_commit(oid)?;

    let _branch = repo.branch(branch_name, &commit, false);

    let obj = repo.revparse_single(&("refs/heads/".to_owned() + branch_name))?;

    repo.checkout_tree(&obj, None)?;

    repo.set_head(&("refs/heads/".to_owned() + branch_name))?;

    let diff = match (tl, tr) {
        (Some(local), Some(origin)) => {
            repo.diff_tree_to_tree(local.as_tree(), origin.as_tree(), None)?
        }
        (_, _) => return Err(git2::Error::from_str("Could not resolve local or remote tree")),
    };

    if diff.deltas().len() > 0 {
        if verbosity >= 2 {
            if let Err(e) = display_stats(&diff) {
                eprintln!("Warning: unable to print diff stats: {}", e);
            }
        }
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        repo.reference_to_annotated_commit(&fetch_head)
    } else {
        let msg = "no diffs, back to sleep.";
        Err(git2::Error::from_str(msg))
    }
}

/// Get commit metadata for the last commit on a branch.
/// Returns CommitMetadata to be passed to child processes.
pub fn get_last_commit_metadata(
    repo: &git2::Repository,
    branch_name: &str,
    verbosity: u8,
) -> Result<CommitMetadata, git2::Error> {
    let commit = find_last_commit_on_branch(repo, branch_name)?;
    Ok(extract_commit_metadata(&commit, verbosity))
}

pub fn tree_to_treeish<'a>(
    repo: &'a Repository,
    arg: Option<&String>,
) -> Result<Option<Object<'a>>, git2::Error> {
    let arg = match arg {
        Some(s) => s,
        None => return Ok(None),
    };
    let obj = repo.revparse_single(arg).map_err(|e| {
        git2::Error::from_str(&format!("branch '{}' not found: {}", arg, e))
    })?;
    let tree = obj.peel(ObjectType::Tree)?;
    Ok(Some(tree))
}

fn display_stats(diff: &Diff) -> Result<(), git2::Error> {
    let stats = diff.stats()?;
    let format = DiffStatsFormat::FULL;
    let buf = stats.to_buf(format, 80)?;
    let dt = Utc::now();
    if let Ok(s) = std::str::from_utf8(&buf) {
        print!("goa [{}]: {}", dt, s);
    }
    Ok(())
}

fn find_last_commit_on_branch<'a>(
    repo: &'a Repository,
    branch_name: &str,
) -> Result<Commit<'a>, git2::Error> {
    let (object, reference) = repo.revparse_ext(branch_name)?;

    repo.checkout_tree(&object, None)?;

    match reference {
        // gref is an actual reference like branches or tags
        Some(gref) => {
            let name = gref
                .name()
                .ok_or_else(|| git2::Error::from_str("Reference has no name"))?;
            repo.set_head(name)?;
        }
        // this is a commit, not a reference
        None => repo.set_head_detached(object.id())?,
    }

    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

fn find_last_commit(repo: &Repository) -> Result<Commit<'_>, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

/// Extract metadata from a commit. Prints commit info if verbosity > 0.
/// Returns CommitMetadata instead of setting global env vars (thread-safe).
fn extract_commit_metadata(commit: &Commit, verbosity: u8) -> CommitMetadata {
    let timestamp = commit.time().seconds();
    let tm = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);

    if verbosity > 0 {
        let dt = Utc::now();
        println!(
            "goa [{}]: commit {}\nAuthor: {}\nDate:   {}\n\n    {}",
            dt,
            commit.id(),
            commit.author(),
            tm,
            commit.message().unwrap_or("no commit message")
        );
    }

    CommitMetadata {
        id: commit.id().to_string(),
        author: commit.author().to_string(),
        message: commit.message().unwrap_or("").to_string(),
        time: tm.to_string(),
    }
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        eprintln!("Error: Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

/// Perform a merge and return the commit metadata for the resulting commit.
pub fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
    verbosity: u8,
) -> Result<CommitMetadata, git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
        let commit = find_last_commit(repo)?;
        Ok(extract_commit_metadata(&commit, verbosity))
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
        let commit = find_last_commit(repo)?;
        Ok(extract_commit_metadata(&commit, verbosity))
    } else {
        eprintln!("Error: Nothing to do?");
        Ok(CommitMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_metadata_default() {
        let metadata = CommitMetadata::default();
        assert_eq!(metadata.id, "");
        assert_eq!(metadata.author, "");
        assert_eq!(metadata.message, "");
        assert_eq!(metadata.time, "");
    }

    #[test]
    fn test_commit_metadata_to_env_vars() {
        let metadata = CommitMetadata {
            id: "abc123def456".to_string(),
            author: "Test Author <test@example.com>".to_string(),
            message: "Fix bug in feature".to_string(),
            time: "2024-01-15 10:30:00 UTC".to_string(),
        };

        let vars = metadata.to_env_vars();

        assert_eq!(vars.len(), 4);
        assert_eq!(
            vars.get("GOA_LAST_COMMIT_ID"),
            Some(&"abc123def456".to_string())
        );
        assert_eq!(
            vars.get("GOA_LAST_COMMIT_AUTHOR"),
            Some(&"Test Author <test@example.com>".to_string())
        );
        assert_eq!(
            vars.get("GOA_LAST_COMMIT_MESSAGE"),
            Some(&"Fix bug in feature".to_string())
        );
        assert_eq!(
            vars.get("GOA_LAST_COMMIT_TIME"),
            Some(&"2024-01-15 10:30:00 UTC".to_string())
        );
    }

    #[test]
    fn test_commit_metadata_clone() {
        let original = CommitMetadata {
            id: "abc123".to_string(),
            author: "Author".to_string(),
            message: "Message".to_string(),
            time: "Time".to_string(),
        };

        let cloned = original.clone();

        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.author, original.author);
        assert_eq!(cloned.message, original.message);
        assert_eq!(cloned.time, original.time);
    }

    #[test]
    fn test_commit_metadata_empty_values() {
        let metadata = CommitMetadata {
            id: "".to_string(),
            author: "".to_string(),
            message: "".to_string(),
            time: "".to_string(),
        };

        let vars = metadata.to_env_vars();

        // Even empty values should be present in the env vars
        assert_eq!(vars.len(), 4);
        assert_eq!(vars.get("GOA_LAST_COMMIT_ID"), Some(&"".to_string()));
        assert_eq!(vars.get("GOA_LAST_COMMIT_AUTHOR"), Some(&"".to_string()));
        assert_eq!(vars.get("GOA_LAST_COMMIT_MESSAGE"), Some(&"".to_string()));
        assert_eq!(vars.get("GOA_LAST_COMMIT_TIME"), Some(&"".to_string()));
    }

    #[test]
    fn test_commit_metadata_special_chars() {
        let metadata = CommitMetadata {
            id: "abc123".to_string(),
            author: "Author Name <author@example.com>".to_string(),
            message: "Fix: handle \"special\" chars & newlines\nLine 2".to_string(),
            time: "2024-01-15T10:30:00+00:00".to_string(),
        };

        let vars = metadata.to_env_vars();

        assert_eq!(
            vars.get("GOA_LAST_COMMIT_MESSAGE"),
            Some(&"Fix: handle \"special\" chars & newlines\nLine 2".to_string())
        );
    }
}
