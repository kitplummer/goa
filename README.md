# goa
GitOps Agent - continuously monitors a remote git repository against local/any change, and performs actions (e.g. executes a provided command) - given a periodicity that is defined as a time intervals

## Usage
### Top-level

#### Help (--help)
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

#### Version (--version)
This does exactly what you'd expect.

### Subcommand-level 

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

### Environment Variables

When `goa` executes it provides details on the latest commit through environment variables:

* `GOA_LAST_COMMIT_ID` -> the commit hash of the last commit on the spied upon branch
* `GOA_LAST_COMMIT_TIME` -> the timestamp of the last commit
* `GOA_LAST_COMMIT_AUTHOR` -> the author of the last commit
* `GOA_LAST_COMMIT_MESSAGE` -> the message of the last commit

If there is something specific you're looking for here, let me know via an [issue](https://github.com/kitplummer/goa/issues).

### Windows

Underneath, goa is providing the `cmd /C` so you don't need to pass that in - just the command.

`spy -c 'echo hello' -d 20 -v 3 https://github.com/kitplummer/goa_tester`

And if you are using a `.goa` file to hold the command calling a batch like

```
.\hello.bat
```

## Builds
For each release we're currently building binaries for:
* Generic x86_64 Linux (only tested on current Ubuntu)
* Arm 32-bit for Linux (tested on Raspian on a RaspberryPi Zero)
* Arm 64-bit for Linux (tested on Ubuntu on a RaspberryPi 4)
* 64-bit for CentOS 7
* Windows (only tested in a VM of Windows 11)
* macOS (tested on current macOS)

Need something else, let me know and i'll add the cross-compile to the GitHub Actions pipeline.

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
