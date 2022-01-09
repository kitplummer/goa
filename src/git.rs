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

use chrono::{NaiveDateTime, Utc};
use git2::{
    AutotagOption, Commit, Diff, DiffStatsFormat, FetchOptions, Object, ObjectType,
    RemoteCallbacks, Repository,
};
use std::io::Write;
use std::str;

pub fn is_diff<'a>(
    repo: &'a git2::Repository,
    remote_name: &str,
    branch_name: &str,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = RemoteCallbacks::new();
    let mut remote = repo
        .find_remote(remote_name)
        .or_else(|_| repo.remote_anonymous(remote_name))
        .unwrap();
    cb.sideband_progress(|data| {
        let dt = Utc::now();
        print!(
            "goa [{}]: remote: {}",
            dt,
            std::str::from_utf8(data).unwrap()
        );
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
    remote
        .update_tips(None, true, AutotagOption::Unspecified, None)
        .unwrap();

    let l = String::from(branch_name);
    let r = format!("{}/{}", remote_name, branch_name);
    let tl = tree_to_treeish(repo, Some(&l)).unwrap();
    let tr = tree_to_treeish(repo, Some(&r)).unwrap();

    let head = repo.head().unwrap();
    let oid = head.target().unwrap();
    let commit = repo.find_commit(oid).unwrap();

    let _branch = repo.branch(branch_name, &commit, false);

    let obj = repo
        .revparse_single(&("refs/heads/".to_owned() + branch_name))
        .unwrap();

    repo.checkout_tree(&obj, None)?;

    repo.set_head(&("refs/heads/".to_owned() + branch_name))?;

    let diff = match (tl, tr) {
        (Some(local), Some(origin)) => repo
            .diff_tree_to_tree(local.as_tree(), origin.as_tree(), None)
            .unwrap(),
        (_, _) => unreachable!(),
    };

    if diff.deltas().len() > 0 {
        // TODO: make this a verbose thing
        display_stats(&diff).expect("ERROR: unable to print diff stats");
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        repo.reference_to_annotated_commit(&fetch_head)
    } else {
        let msg = "no diffs, back to sleep.";
        Err(git2::Error::from_str(msg))
    }
}

pub fn tree_to_treeish<'a>(
    repo: &'a Repository,
    arg: Option<&String>,
) -> Result<Option<Object<'a>>, git2::Error> {
    let arg = match arg {
        Some(s) => s,
        None => return Ok(None),
    };
    let obj = match repo.revparse_single(arg) {
        Ok(obj) => obj,
        Err(_) => {
            println!("Error: branch not found");
            std::process::exit(1);
        }
    };
    let tree = obj.peel(ObjectType::Tree).unwrap();
    Ok(Some(tree))
}

fn display_stats(diff: &Diff) -> Result<(), git2::Error> {
    let stats = diff.stats().unwrap();
    let format = DiffStatsFormat::FULL;
    let buf = stats.to_buf(format, 80).unwrap();
    let dt = Utc::now();
    print!("goa [{}]: {}", dt, std::str::from_utf8(&*buf).unwrap());
    Ok(())
}

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

fn display_commit(commit: &Commit) {
    let timestamp = commit.time().seconds();
    let tm = NaiveDateTime::from_timestamp(timestamp, 0);
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
    // TODO: make this a verbose things
    // println!("{}", msg);
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
        println!("Error: Merge conficts detected...");
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

pub fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
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
        let commit = find_last_commit(repo).expect("Couldn't find last commit");
        display_commit(&commit);
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
        let commit = find_last_commit(repo).expect("Couldn't find last commit");
        display_commit(&commit);
    } else {
        println!("Error: Nothing to do?");
    }
    Ok(())
}
