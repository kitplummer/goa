mod cli;
mod git;
mod lei;
mod radicle;
mod repos;
mod spy;

use crate::lei::LeiConfig;
use crate::radicle::RadicleConfig;
use crate::repos::Repo;
use clap::Parser;
use cli::{Action::*, CommandLineArgs};

#[macro_use]
extern crate log;

use env_logger::{Builder, Env, Target};

fn main() {
    let CommandLineArgs { action } = CommandLineArgs::parse();

    let result = match action {
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
            lei_url,
            lei_token,
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

            if let Some(lei) = lei_url {
                builder = builder.lei_config(LeiConfig {
                    url: lei,
                    token: lei_token,
                });
            }

            let repo = builder.build();

            init_logger(verbosity);
            info!("starting");

            spy::spy_repo(repo)
        }

        Radicle {
            seed_url,
            rid,
            command,
            delay,
            verbosity,
            timeout,
            watch_patches,
            local_path,
        } => {
            let mut builder = RadicleConfig::builder(&seed_url, &rid)
                .delay(delay)
                .verbosity(verbosity)
                .timeout(timeout)
                .watch_patches(watch_patches)
                .command(command);

            if let Some(path) = local_path {
                builder = builder.local_path(path);
            }

            let config = builder.build();

            init_logger(verbosity);
            info!("starting radicle watcher");

            radicle::watch_radicle(config)
        }
    };

    if let Err(e) = result {
        let msg = e.to_string();
        eprintln!("{}", msg);

        // Forward the child process exit code if available
        let code = parse_exit_code(&msg).unwrap_or(1);
        std::process::exit(code);
    }
}

/// Extract exit code from error messages containing "Command exited with code 127"
fn parse_exit_code(msg: &str) -> Option<i32> {
    let marker = "Command exited with code ";
    let start = msg.find(marker)? + marker.len();
    msg[start..].split_whitespace().next()?.parse().ok()
}

fn init_logger(verbosity: u8) {
    let log_level = match verbosity {
        0 => "error",
        1 => "info",
        _ => "debug",
    };

    Builder::from_env(Env::default().default_filter_or(log_level))
        .target(Target::Stdout)
        .init();
}
