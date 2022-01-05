mod cli;
mod git;
mod repos;
mod spy;

use structopt::StructOpt;

use cli::{Action::*, CommandLineArgs};

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
        } => spy::spy_repo(url, branch, delay, username, token, command, verbosity),
    }?;

    Ok(())
}
