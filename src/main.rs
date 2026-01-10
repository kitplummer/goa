mod cli;
mod git;
mod repos;
mod spy;

use crate::repos::Repo;
use clap::Parser;
use cli::{Action::*, CommandLineArgs};

#[macro_use]
extern crate log;

use env_logger::{Builder, Env, Target};

fn main() -> anyhow::Result<()> {
    let CommandLineArgs { action } = CommandLineArgs::parse();

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
            exit_on_first_diff,
            target_path,
            timeout,
        } => {
            let mut builder = Repo::builder(&url)
                .branch(branch)
                .delay(delay)
                .verbosity(verbosity)
                .exec_on_start(exec_on_start)
                .exit_on_first_diff(exit_on_first_diff)
                .timeout(timeout)
                .command(command);

            if let Some(u) = username {
                builder = builder.username(u);
            }
            if let Some(t) = token {
                builder = builder.token(t);
            }
            if let Some(p) = target_path {
                builder = builder.local_path(p);
            }

            let repo = builder.build();

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
