# goa
GitOps Agent - continuously monitors a remote git repository against local/any change, and performs actions (e.g. executes a provided command) - given a periodicity that is defined as a time intervals

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
    goa spy [FLAGS] [OPTIONS] <url>

FLAGS:
    -e, --exec-on-start    Execute the command, or .goa file, on start
    -h, --help             Prints help information
    -V, --version          Prints version information

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

The `.goa` file can only run a single command (right now, maybe multilines in the future)

## Contributing

Nothing formal, but PRs are the means. Create an [issue](https://github.com/kitplummer/goa/issues) if you have a question, comment, or just because. :D

## License

```
MIT License

Copyright (c) 2021 Kit Plummer

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
