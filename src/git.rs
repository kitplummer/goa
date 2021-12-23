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

use git2::{AutotagOption, Diff, DiffStatsFormat, FetchOptions, Object, ObjectType, RemoteCallbacks, Repository};
use std::io::{self, Write};
use std::str;

pub fn is_diff<'a>(
    repo: &'a git2::Repository,
    remote_name: &str,
    branch_name: &str
) -> bool {
  println!("Fetching {} for repo", remote_name);
  let mut cb = RemoteCallbacks::new();
  let mut remote = repo
      .find_remote(remote_name)
      .or_else(|_| repo.remote_anonymous(remote_name)).unwrap();
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

  let l = String::from(branch_name);
  let r = format!("{}/{}", remote_name, branch_name);
  let tl = tree_to_treeish(&repo, Some(&l)).unwrap();
  let tr = tree_to_treeish(&repo, Some(&r)).unwrap();

  let diff = match (tl, tr) {
      (Some(local), Some(origin)) => repo.diff_tree_to_tree(local.as_tree(), origin.as_tree(), None).unwrap(),
      (_, _) => unreachable!(),
  };

  print_stats(&diff).expect("unable to print diff stats");

  if diff.deltas().len() > 0 {
    true
  } else {
    false
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
    let obj = repo.revparse_single(arg).unwrap();
    let tree = obj.peel(ObjectType::Tree).unwrap();
    Ok(Some(tree))
}

fn print_stats(diff: &Diff) -> Result<(), git2::Error> {
    let stats = diff.stats().unwrap();
    let format = DiffStatsFormat::SHORT;
    let buf = stats.to_buf(format, 80).unwrap();
    print!("{}", std::str::from_utf8(&*buf).unwrap());
    Ok(())
}

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::new();

    // Print out our transfer progress.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        io::stdout().flush().unwrap();
        true
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    println!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        println!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        println!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
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
    println!("{}", msg);
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
        println!("Merge conficts detected...");
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

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
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
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        println!("Nothing to do...");
    }
    Ok(())
}

// fn run(args: &Args) -> Result<(), git2::Error> {
//     let remote_name = args.arg_remote.as_ref().map(|s| &s[..]).unwrap_or("origin");
//     let remote_branch = args.arg_branch.as_ref().map(|s| &s[..]).unwrap_or("master");
//     let repo = Repository::open(".")?;
//     let mut remote = repo.find_remote(remote_name)?;
//     let fetch_commit = do_fetch(&repo, &[remote_branch], &mut remote)?;
//     do_merge(&repo, &remote_branch, fetch_commit)
// }
