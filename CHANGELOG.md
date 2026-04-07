# Changelog

## [0.1.2] - 2026-04-07

### Fixed

- GoReleaser: removed universal binaries in favor of per-architecture archives for consistent Linux/macOS parity

## [0.1.1] - 2026-04-07

### Fixed

- GoReleaser config: unique IDs for universal binary entries
- Release workflow: `grep -c` exit code handling for non-prerelease versions

## [0.1.0] - 2026-04-07

### Added

- `infrahub-git-credential` binary implementing the git credential helper protocol (`get`, `store`, `erase`)
- `infrahub-git-askpass` binary implementing the GIT_ASKPASS protocol
- Configuration via environment variables (`INFRAHUB_INTERNAL_ADDRESS`, `INFRAHUB_API_TOKEN`) with TOML file fallback
- GraphQL string escaping to prevent injection
- GraphQL type name validation
- Multi-arch Dockerfile producing statically linked musl binaries (`amd64`, `arm64`)
- CI workflow with formatting, linting, testing, and Docker build validation
- Release workflow with Docker image publishing and binary distribution via GoReleaser
