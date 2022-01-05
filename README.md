# goa
GitOps Agent - monitors remote repos against local/any change, and performs actions - given a periodicity that is defined as a time intervals

## Usage

```
goa 0.0.1
A command-line GitOps utility agent

USAGE:
    goa <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    spy     Spy a remote git repo for changes, will execute defined script/command on first run
```

```
goa-spy 0.0.1
Spy a remote git repo for changes, will execute defined script/command on first run

USAGE:
    goa spy [OPTIONS] <url>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --branch <branch>        The branch of the remote git repo to watch for changes [default: main]
    -c, --command <command>      The command to run when a change is detected
    -d, --delay <delay>          The time between checks in seconds, max 65535 [default: 120]
    -t, --token <token>          The access token for cloning and fetching of the remote repo
    -u, --username <username>    Username, owner of the token - required for private repos

ARGS:
    <url>    The remote git repo to watch for changes
```
