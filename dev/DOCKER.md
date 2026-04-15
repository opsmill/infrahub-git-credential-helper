# Docker & Multi-arch Builds

## Dockerfile

Two-stage musl build:

1. **Dependency layer**: copies `Cargo.toml`, `Cargo.lock`, `schema/`, and `src/graphql/`, writes stub `src/` files, then runs `cargo build --release`. Because `graphql-client` needs the schema and query files at compile time, **they must be copied before the stub build** so the dependency cache layer can resolve them.
2. **Source layer**: copies real `src/` and rebuilds just this crate.

`TARGETARCH` selects between `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`. The final stage is `FROM scratch` — binaries are fully static (musl + rustls, no libc, no OpenSSL).

## Release multi-arch pipeline

`.github/workflows/ci-docker-image.yml` is a reusable workflow that builds each platform on a native runner:

- `linux/amd64` → `ubuntu-latest`
- `linux/arm64` → `ubuntu-24.04-arm`

Each job pushes an image digest; a final `merge` job combines them into a multi-arch manifest via `docker buildx imagetools create`. No QEMU.

## GoReleaser binary distribution

`.goreleaser.yml` uses `cargo-zigbuild` to cross-compile four targets (linux amd64/arm64, darwin amd64/arm64) from a single Linux runner during releases. Zig is used purely as a cross-linker for C deps (e.g. `ring`); the Rust compiler is still `rustc`.

The same config is used in `snapshot` mode on every push to `main` to produce downloadable artifacts for HEAD.
