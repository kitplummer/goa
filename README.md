# goa
[GitOps](https://www.redhat.com/en/topics/devops/what-is-gitops) Agent - continuously monitors a remote git repository against local/any change, and performs actions (e.g. executes a provided command) - given a periodicity that is defined as a time intervals.

## Security Warning

**goa executes arbitrary commands** from the `-c` flag or `.goa` files in monitored repositories. Before using goa:

- **Never run as root** - use a dedicated unprivileged user
- **Only monitor trusted repositories** - a compromised repo can execute malicious code
- **Use `--timeout`** - prevent runaway commands from consuming resources
- **Use deploy keys** - prefer read-only deploy keys over personal access tokens

See [SECURITY.md](SECURITY.md) for detailed security considerations and deployment recommendations.

## Installation

### From crates.io (recommended)
```bash
cargo install gitops-agent
```

### From releases
Download a binary from the [releases](https://github.com/kitplummer/goa/releases) for your OS and CPU architecture. Be sure to make the binary executable on UNIX-based OSes (e.g. `chmod +x goa`).

## Usage
### Top-level

#### Help (--help)
```
A command-line GitOps utility agent

Usage: goa <COMMAND>

Commands:
  spy      Spy a remote git repo for changes, will continuously execute defined script/command on a diff
  radicle  Watch a Radicle repository for changes via HTTP API
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### Version (--version)
This does exactly what you'd expect.

### Subcommand-level 

#### Spy
```
Spy a remote git repo for changes, will continuously execute defined script/command on a diff

Usage: goa spy [OPTIONS] <URL>

Arguments:
  <URL>  The remote git repo to watch for changes

Options:
  -b, --branch <BRANCH>            The branch of the remote git repo to watch for changes [default: main]
  -d, --delay <DELAY>              The time between checks in seconds, max 65535 [default: 120]
  -u, --username <USERNAME>        Username, owner of the token - required for private repos
  -t, --token <TOKEN>              The access token for cloning and fetching of the remote repo
  -c, --command <COMMAND>          The command to run when a change is detected [default: ]
  -v, --verbosity <VERBOSITY>      Adjust level of stdout, 0 no goa output, max 2 (debug) [default: 1]
  -e, --exec-on-start              Execute the command, or .goa file, on start
  -x, --exit-on-first-diff         Exit immediately after first diff spied
  -T, --target-path <TARGET_PATH>  The target path for the clone
      --timeout <TIMEOUT>          Timeout for command execution in seconds (0 = no timeout) [default: 0]
  -h, --help                       Print help
```

#### Examples

* `goa spy -c 'echo "hello from goa"' -e -d 20 https://github.com/kitplummer/goa_tester`

This will echo out to the command line on startup, and then on any change to the main branch, looking for changes every 20 seconds.

* `goa spy -d 120 -b develop -v 2 https://github.com/kitplummer/goa_tester`

This will execute the contents of the `.goa` file in the repo on any diffs found in the develop branch, looking for changes every 120 seconds. It will also log out debug-level details, which occur inside the processing loop (may get noisy).

* `goa spy -c 'echo "change by ${GOA_LAST_COMMIT_AUTHOR} made to main branch"' https://github.com/kitplummer/goa_tester`

This will output the author of the last commit made to the main branch, looking for changes every 120 seconds.

* `goa spy -c 'echo "changed!"' -x https://github.com/kitplummer/goa_tester`

This will output "changed!" on stdout then exit after the first diff is identified on the "main" branch of the provided remote repo.

* `goa spy -c 'echo "changed!"' -T "/tmp/goa" -x https://github.com/kitplummer/goa_tester`

Will do the same as above, but create the local clone at `/tmp/goa`.

#### Radicle

Watch a [Radicle](https://radicle.xyz) repository for changes via HTTP API. This enables CI/CD for decentralized git projects.

```
Watch a Radicle repository for changes via HTTP API

Usage: goa radicle [OPTIONS] --seed-url <SEED_URL> --rid <RID>

Options:
  -s, --seed-url <SEED_URL>      The Radicle seed node URL (e.g., https://iris.radicle.xyz)
  -r, --rid <RID>                The Radicle repository ID (e.g., rad:z3fF7wV6LXz915ND1nbHTfeY3Qcq7)
  -c, --command <COMMAND>        The command to run when a change is detected [default: ]
  -d, --delay <DELAY>            The time between checks in seconds, max 65535 [default: 120]
  -v, --verbosity <VERBOSITY>    Adjust level of stdout, 0 no goa output, max 2 (debug) [default: 1]
      --timeout <TIMEOUT>        Timeout for command execution in seconds (0 = no timeout) [default: 0]
  -p, --watch-patches            Watch for patch (PR) updates in addition to head changes
  -l, --local-path <LOCAL_PATH>  Local working directory for command execution and .goa file
```

##### Radicle Examples

* Watch for pushes and patches on a Radicle repo:
```bash
goa radicle -s https://iris.radicle.xyz -r rad:z3fF7wV6LXz915ND1nbHTfeY3Qcq7 -c './run-ci.sh'
```

* Watch only for head changes (no patches):
```bash
goa radicle -s https://iris.radicle.xyz -r rad:z3fF7wV6LXz915ND1nbHTfeY3Qcq7 -c 'echo "new push!"' --watch-patches=false
```

##### Radicle Environment Variables

When triggered by Radicle events, goa provides these environment variables:

| Variable | Description |
|----------|-------------|
| `GOA_RADICLE_RID` | Repository ID (e.g., `rad:z3fF7wV6LXz915ND1nbHTfeY3Qcq7`) |
| `GOA_RADICLE_URL` | Seed node URL |
| `GOA_TRIGGER_TYPE` | `push` or `patch` |
| `GOA_COMMIT_OID` | Commit SHA to test |
| `GOA_PATCH_ID` | Patch ID (if trigger_type=patch) |
| `GOA_BASE_COMMIT` | Base commit for patches |
| `GOA_PATCH_STATE` | Patch state: open/merged/archived |
| `GOA_PATCH_TITLE` | Patch title |

##### Example CI Script for Radicle

```bash
#!/bin/bash
echo "Testing commit $GOA_COMMIT_OID"
if [ "$GOA_TRIGGER_TYPE" = "patch" ]; then
  echo "Patch: $GOA_PATCH_ID - $GOA_PATCH_TITLE"
  echo "Base: $GOA_BASE_COMMIT"
fi

# Clone from local Radicle storage and checkout the commit
git clone ~/.radicle/storage/${GOA_RADICLE_RID#rad:} /tmp/ci-$$
cd /tmp/ci-$$
git checkout $GOA_COMMIT_OID

# Run tests
cargo test
```

### Using a `.goa` File

If no `-c`/`--command` is provided when starting `goa` - it will automatically look for a `.goa` file in the remote git repository, and execute the command within it.

The `.goa` file can only run a single command (right now, maybe multilines in the future)

An example repo with a `.goa` file can be seen here: https://github.com/kitplummer/goa_tester

### Environment Variables

When `goa` executes it provides details on the latest commit through environment variables:

* `GOA_LAST_COMMIT_ID` -> the commit hash of the last commit on the spied upon branch
* `GOA_LAST_COMMIT_TIME` -> the timestamp of the last commit
* `GOA_LAST_COMMIT_AUTHOR` -> the author of the last commit
* `GOA_LAST_COMMIT_MESSAGE` -> the message of the last commit

If there is something specific you're looking for here, let me know via an [issue](https://github.com/kitplummer/goa/issues).

### Windows

Underneath, goa is providing the `cmd /C` so you don't need to pass that in - just the command.

`goa spy -c 'echo hello' -d 20 -v 2 https://github.com/kitplummer/goa_tester`

And if you are using a `.goa` file, reference the command calling a batch like

```
.\hello.bat
```

### Running as a container

Note: public release of the container to the GitHub registry is a work-in-progress - we'll update the doc here when the container is pushed within the release process.  For now you can build the container then run it.

```
docker run -it --rm kitplummer/goa spy --help
```

Running from a container, depending on the permissions of the underlying container system may cause issues with the abilty to execute commands from goa itself.  Also, it might make more sense for goa to be integrated into a container that includes to the commands to be executed.

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
