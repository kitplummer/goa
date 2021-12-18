use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum Action {
    /// Spy a remote git repo for changes, will execute defined script/command on first run
    Spy {
        /// The remote git repo to watch for changes
        #[structopt()]
        url: String,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "goa",
    about = "A command-line GitOps utility agent"
)]

pub struct CommandLineArgs {
    #[structopt(subcommand)]
    pub action: Action,
}

