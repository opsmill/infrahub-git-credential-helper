# Architecture

## Project shape

Two Rust binaries sharing `src/lib.rs`:

- `infrahub-git-credential` — git credential helper protocol (`get`, `store`, `erase`)
- `infrahub-git-askpass` — `GIT_ASKPASS` protocol

`src/lib.rs` loads config, talks to Infrahub over GraphQL, and returns `(username, password)`.

## GraphQL with compile-time bindings

`src/lib.rs` uses `graphql-client` to generate typed Rust bindings from the schema and query at build time:

```rust
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "src/graphql/get_repo_credential.graphql",
    response_derives = "Debug"
)]
struct GetRepoCredential;
```

The macro reads both files during compilation and generates a `get_repo_credential` module with typed `Variables` and `ResponseData`. **The schema is only needed at build time** — binaries do not ship it.

The query uses an inline fragment (`... on CorePasswordCredential`) to fetch the repo location and credentials in a single round trip. The fragment becomes a Rust enum (`CredentialOn`) that the compiler forces us to match exhaustively.

When the upstream schema changes, `cargo check` catches breakage at build time. CI has a `schema` job that fetches the latest upstream schema and rebuilds to detect upstream-only drift.

## Config resolution

`InfrahubConfig::load()` resolves in this order:

1. Env vars (`INFRAHUB_INTERNAL_ADDRESS`, `INFRAHUB_API_TOKEN`, `INFRAHUB_USERNAME` / `INFRAHUB_PASSWORD`, `INFRAHUB_PROXY`, `INFRAHUB_TLS_*`, `INFRAHUB_TIMEOUT`)
2. `[main].internal_address` from a TOML file (path from `--config-file`, then `INFRAHUB_CONFIG`, default `infrahub.toml`)
3. `INFRAHUB_ADDRESS` (SDK-compatible fallback)

## Authentication

`resolve_auth_header()` returns one of:

- `X-INFRAHUB-KEY: <token>` if `api_token` is set
- `Authorization: Bearer <access_token>` after `POST /api/auth/login` with `username` / `password`
- `None` (unauthenticated) otherwise

Token auth takes priority when both are configured. Every request also sends `X-Infrahub-Tracker: infrahub-git-credential-helper` for server-side log filtering.

## Tests and env vars

Many tests exercise `InfrahubConfig::load()` by setting and removing process env vars. `env::set_var` / `env::remove_var` are `unsafe` in Rust 2024 edition because concurrent access is UB, so tests must run with `--test-threads=1` (wired into `make test` and both CI workflows).
