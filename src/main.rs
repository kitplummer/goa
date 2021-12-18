mod cli;
mod repos;

use structopt::StructOpt;

use cli::{Action::*, CommandLineArgs};

fn main() -> anyhow::Result<()> {
    let CommandLineArgs {
        action,
    } = CommandLineArgs::from_args();

    match action {
        Spy { url } => repos::spy_repo(url),
    }?;

    Ok(())
}
