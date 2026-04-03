# Infrahub Git Credential Helper

Git credential helpers for [Infrahub](https://github.com/opsmill/infrahub). These tools allow git processes to authenticate against repositories managed by Infrahub by fetching credentials from the Infrahub GraphQL API.

## Binaries

### `infrahub-git-credential`

A [git credential helper](https://git-scm.com/docs/gitcredentials) that implements the `get` and `store` subcommands. When git needs credentials for a remote, this helper queries Infrahub for the repository's stored credentials.

Requires `credential.useHttpPath` to be enabled:

```sh
git config --global credential.useHttpPath true
```

### `infrahub-git-askpass`

A [GIT_ASKPASS](https://git-scm.com/docs/gitcredentials#_requesting_credentials) helper. When git needs a username or password, it calls this program with a prompt string and reads the response from stdout.

## Configuration

Configuration is resolved in the following order:

| Setting | Environment Variable | TOML Config | Default |
|---------|---------------------|-------------|---------|
| Server address | `INFRAHUB_INTERNAL_ADDRESS` | `[main].internal_address` | - |
| API token | `INFRAHUB_API_TOKEN` | - | None |
| Config file path | `INFRAHUB_CONFIG` | - | `infrahub.toml` |

Both binaries also accept `--config-file <path>` to override the config file path.

Environment variables take precedence over TOML values.

## Building

```sh
cargo build --release
```

Binaries are output to `target/release/infrahub-git-credential` and `target/release/infrahub-git-askpass`.

## Docker

The project includes a multi-stage Dockerfile that produces statically linked binaries (musl) in a scratch-based image. Supports both `amd64` and `arm64` architectures.

```sh
docker build -t registry.opsmill.io/opsmill/infrahub-git-credential-helper .
```

Use in a downstream Dockerfile:

```dockerfile
COPY --from=registry.opsmill.io/opsmill/infrahub-git-credential-helper:latest /usr/bin/infrahub-git-credential /usr/bin/infrahub-git-credential
COPY --from=registry.opsmill.io/opsmill/infrahub-git-credential-helper:latest /usr/bin/infrahub-git-askpass /usr/bin/infrahub-git-askpass
```

## License

This project is licensed under the Apache License, Version 2.0. See [LICENSE.txt](LICENSE.txt) for details.
