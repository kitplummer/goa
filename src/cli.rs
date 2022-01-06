use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum Action {
    /// Spy a remote git repo for changes, will continuously execute defined script/command on a diff
    Spy {
        /// The remote git repo to watch for changes
        #[structopt()]
        url: String,
        /// The branch of the remote git repo to watch for changes
        #[structopt(short, long, default_value = "main")]
        branch: String,
        /// The time between checks in seconds, max 65535
        #[structopt(short, long, default_value = "120")]
        delay: u16,
        /// Username, owner of the token - required for private repos
        #[structopt(short, long)]
        username: Option<String>,
        /// The access token for cloning and fetching of the remote repo
        #[structopt(short, long)]
        token: Option<String>,
        /// The command to run when a change is detected
        #[structopt(short, long)]
        command: Option<String>,
        /// The level of output to both standard out and error, max 3
        #[structopt(short, long, default_value = "1")]
        verbosity: u8
    },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "goa", about = "A command-line GitOps utility agent")]

pub struct CommandLineArgs {
    #[structopt(subcommand)]
    pub action: Action,
}
