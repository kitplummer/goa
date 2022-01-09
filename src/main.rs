mod cli;
mod git;
mod repos;
mod spy;

use crate::repos::Repo;
use cli::{Action::*, CommandLineArgs};
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    let CommandLineArgs { action } = CommandLineArgs::from_args();

    match action {
        Spy {
            url,
            branch,
            delay,
            username,
            token,
            command,
            verbosity,
            exec_on_start,
        } => {
            let repo = Repo::new(
                url,
                username,
                token,
                Some(String::from("initialize")),
                Some(String::from("initialize")),
                branch,
                command,
                delay,
                verbosity,
                exec_on_start,
            );
            spy::spy_repo(repo)
        }
    }?;

    Ok(())
}
