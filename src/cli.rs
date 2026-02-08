use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Action {
    /// Spy a remote git repo for changes, will continuously execute defined script/command on a diff
    Spy {
        /// The remote git repo to watch for changes
        url: String,
        /// The branch of the remote git repo to watch for changes
        #[arg(short, long, default_value = "main")]
        branch: String,
        /// The time between checks in seconds, max 65535
        #[arg(short, long, default_value = "120")]
        delay: u16,
        /// Username, owner of the token - required for private repos
        #[arg(short, long)]
        username: Option<String>,
        /// The access token for cloning and fetching of the remote repo
        #[arg(short, long)]
        token: Option<String>,
        /// The command to run when a change is detected
        #[arg(short, long, default_value = "")]
        command: String,
        /// Adjust level of stdout, 0 no goa output , max 2 (debug)
        #[arg(short, long, default_value = "1")]
        verbosity: u8,
        /// Execute the command, or .goa file, on start
        #[arg(short, long)]
        exec_on_start: bool,
        /// Exit immediately after first diff spied
        #[arg(short = 'x', long)]
        exit_on_first_diff: bool,
        /// The target path for the clone
        #[arg(short = 'T', long)]
        target_path: Option<String>,
        /// Timeout for command execution in seconds (0 = no timeout)
        #[arg(long, default_value = "0")]
        timeout: u64,
        /// LEI (LowEndInsight) API URL for dependency health analysis on change
        #[arg(long)]
        lei_url: Option<String>,
        /// Bearer token for LEI API authentication
        #[arg(long)]
        lei_token: Option<String>,
    },

    /// Watch a Radicle repository for changes via HTTP API
    Radicle {
        /// The Radicle seed node URL (e.g., https://iris.radicle.xyz)
        #[arg(short = 's', long)]
        seed_url: String,
        /// The Radicle repository ID (e.g., rad:z3fF7wV6LXz915ND1nbHTfeY3Qcq7)
        #[arg(short, long)]
        rid: String,
        /// The command to run when a change is detected
        #[arg(short, long, default_value = "")]
        command: String,
        /// The time between checks in seconds, max 65535
        #[arg(short, long, default_value = "120")]
        delay: u16,
        /// Adjust level of stdout, 0 no goa output, max 2 (debug)
        #[arg(short, long, default_value = "1")]
        verbosity: u8,
        /// Timeout for command execution in seconds (0 = no timeout)
        #[arg(long, default_value = "0")]
        timeout: u64,
        /// Watch for patch (PR) updates in addition to head changes
        #[arg(short = 'p', long, default_value = "true")]
        watch_patches: bool,
        /// Local working directory for command execution and .goa file
        #[arg(short = 'l', long)]
        local_path: Option<String>,
    },
}

#[derive(Debug, Parser)]
#[command(name = "goa", about = "A command-line GitOps utility agent")]
pub struct CommandLineArgs {
    #[command(subcommand)]
    pub action: Action,
}
