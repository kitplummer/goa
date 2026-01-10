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
    },
}

#[derive(Debug, Parser)]
#[command(name = "goa", about = "A command-line GitOps utility agent")]
pub struct CommandLineArgs {
    #[command(subcommand)]
    pub action: Action,
}
