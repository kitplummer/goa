mod cli;
mod git;
mod repos;
mod spy;

use crate::repos::Repo;
use cli::{Action::*, CommandLineArgs};
use structopt::StructOpt;

#[macro_use]
extern crate log;

use env_logger::{Builder, Env, Target};

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

            let log_level = match verbosity {
                0 => "error",
                1 => "info",
                _ => "debug",
            };

            Builder::from_env(Env::default().default_filter_or(log_level))
                .target(Target::Stdout)    
                .init();

            info!("starting");

            spy::spy_repo(repo)
        }
    }?;

    Ok(())
}
