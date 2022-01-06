# goa
GitOps Agent - monitors remote repos against local/any change, and performs actions - given a periodicity that is defined as a time intervals

## Usage
### Sub Commands
```
A command-line GitOps utility agent

USAGE:
    goa <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    spy     Spy a remote git repo for changes, will continuously execute defined script/command on a diff
```

### Command-level 

#### Spy
```
Spy a remote git repo for changes, will continuously execute defined script/command on a diff

USAGE:
    goa spy [OPTIONS] <url>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --branch <branch>          The branch of the remote git repo to watch for changes [default: main]
    -c, --command <command>        The command to run when a change is detected
    -d, --delay <delay>            The time between checks in seconds, max 65535 [default: 120]
    -t, --token <token>            The access token for cloning and fetching of the remote repo
    -u, --username <username>      Username, owner of the token - required for private repos
    -v, --verbosity <verbosity>    The level of output to both standard out and error, max 3 [default: 1]

ARGS:
    <url>    The remote git repo to watch for changes
```

### Using a `.goa` File

If no `-c`/`--command` is provided when starting `goa` - it will automatically look for a `.goa` file in the remote git repository, and execute the command within it.