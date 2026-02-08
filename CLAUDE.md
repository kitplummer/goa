# CLAUDE.md - Project Context for AI Agents

## Project Overview

**goa** (GitOps Agent) is a Rust CLI tool that continuously monitors remote git repositories for changes and executes commands when diffs are detected. It supports both standard Git repos (via `spy`) and Radicle decentralized repos (via `radicle`).

- **Crate name:** `gitops-agent`
- **Binary name:** `goa`
- **Version:** 0.1.1
- **License:** MIT
- **Rust edition:** 2021

## Commands

### `goa spy <URL>`
Monitors a remote git repo by cloning it locally, then polling for changes on a configurable interval. When a diff is detected, executes a command (from `-c` flag or `.goa` file in the repo). Supports authentication via username/token for private repos.

### `goa radicle --seed-url <URL> --rid <RID>`
Watches a Radicle repository for changes via the Radicle HTTP API. Polls for head changes (pushes) and optionally patch (PR) updates. Does not clone the repo; instead queries the seed node API.

## Architecture

```
src/
  main.rs       -- Entry point, CLI dispatch via clap
  cli.rs        -- Clap CLI argument definitions (Action enum + CommandLineArgs)
  git.rs        -- libgit2 operations: fetch, diff, merge, commit metadata extraction
  repos.rs      -- Repo struct (builder pattern), clone/spy loop, command execution
  spy/mod.rs    -- spy subcommand: URL auth setup, temp dir management, starts spy loop
  radicle/mod.rs -- radicle subcommand: HTTP API polling, patch tracking, change detection
```

Key patterns:
- **Builder pattern** for both `Repo` and `RadicleConfig`
- **Scheduler loop** using `clokwerk` crate for periodic polling
- **Command execution** via `run_script` (no timeout) or `std::process::Command` with `wait-timeout` (with timeout)
- Commit metadata passed as env vars to child processes (`GOA_LAST_COMMIT_*`, `GOA_RADICLE_*`)

## Build and Test

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests (unit + integration)
cargo clippy -- -D warnings  # Lint
cargo llvm-cov --html    # Coverage (requires cargo-llvm-cov)
```

Or via Makefile: `make build`, `make test`, `make lint`, `make coverage`, `make clean`

## Tests

- **Unit tests** in `src/repos.rs`, `src/git.rs`, `src/radicle/mod.rs` (17 tests)
- **Integration tests** in `tests/cli.rs` (15 tests) using `assert_cmd` and `predicates`
- Integration tests use a real public repo (`kitplummer/goa_tester`) for clone/diff tests
- Tests create temp directories for local clones; cleanup is automatic

## Key Dependencies

- `git2` (libgit2 bindings) -- git clone, fetch, diff, merge
- `clap` (derive) -- CLI argument parsing
- `clokwerk` -- scheduling periodic checks
- `reqwest` (blocking) -- HTTP client for Radicle API
- `run_script` -- shell command execution
- `wait-timeout` -- process timeout support
- `openssl-sys` (vendored) -- TLS support for git operations

## Environment Variables Set by goa

When executing commands on detected changes:

**Spy mode:** `GOA_LAST_COMMIT_ID`, `GOA_LAST_COMMIT_TIME`, `GOA_LAST_COMMIT_AUTHOR`, `GOA_LAST_COMMIT_MESSAGE`

**Radicle mode:** `GOA_RADICLE_RID`, `GOA_RADICLE_URL`, `GOA_TRIGGER_TYPE`, `GOA_COMMIT_OID`, `GOA_PATCH_ID`, `GOA_BASE_COMMIT`, `GOA_PATCH_STATE`, `GOA_PATCH_TITLE`

## Security Note

goa executes arbitrary commands from `-c` flags or `.goa` files. Never run as root. Only monitor trusted repositories. Use `--timeout` to limit command execution time.
