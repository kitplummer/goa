mod cli;
mod repos;
mod git;

use structopt::StructOpt;

use cli::{Action::*, CommandLineArgs};

fn main() -> anyhow::Result<()> {
    let CommandLineArgs {
        action,
    } = CommandLineArgs::from_args();

    match action {
        Spy { url, branch, delay, username, token } => repos::spy_repo(url, branch, delay, username, token),
    }?;

    Ok(())
}
