# AGENTS.md

Guidance for coding agents (Claude Code, Cursor, Copilot, etc.) working in this repository.

## What this is

Git credential helpers for Infrahub, written in Rust. See `README.md` for the user-facing description.

## Commands

All common workflows are wrapped in the `Makefile`:

- `make build` / `make release` — debug / release builds
- `make test` — tests (must run with `--test-threads=1`, see `dev/ARCHITECTURE.md`)
- `make lint` — `cargo fmt --check` + `cargo clippy -- -D warnings`
- `make fmt` — format
- `make update-schema` — pull the latest GraphQL schema from upstream Infrahub
- `make docker` — local Docker build

Run a single test: `cargo test --lib <test_name> -- --test-threads=1`.

## Where to read before making changes

- **`dev/ARCHITECTURE.md`** — code design, GraphQL codegen, config/auth flow, test constraints
- **`dev/DOCKER.md`** — Dockerfile caching quirks, multi-arch pipeline, GoReleaser
- **`dev/COMPATIBILITY.md`** — **read this before "fixing" anything**. Lists quirks (splitting semantics, stdout/stderr routing) that are intentional drop-in-replacement constraints
- **`dev/RELEASE.md`** — release process

## Commands (slash)

- `/release <version>` (defined in `dev/commands/release.md`, symlinked from `.claude/commands/`) — bumps version, commits, tags, pushes, creates GitHub release. Requires `CHANGELOG.md` updated first.
